pub mod hir;
pub mod lower;
pub mod codegen;
pub mod mono;
pub mod impls;
pub mod closures;
pub mod effects;
pub mod handlers;

use crate::ast;
use crate::bytecode::{BytecodeChunk, Chunk, NativeChunk, OpCode, Register, Module};
use crate::error::{Error, ErrorCode};
use crate::myriad::Value;
use std::collections::HashMap;
use std::rc::Rc;

use self::hir::LayoutCtx;
use crate::ty::Type as TyType;

pub struct LoopCtx {
    pub result_reg: Register,
    pub continue_target: usize,
    pub break_patches: Vec<usize>,
    pub continue_patches: Vec<usize>,
}

#[derive(Clone)]
pub struct HostFnDecl {
    pub name: String,
    pub params: Vec<TyType>,
    pub ret: TyType,
    pub fn_id: u16,
}

pub struct Compiler {
    pub(super) constants: Vec<Value>,
    pub(super) string_constants: Vec<String>,
    pub(super) code: Vec<OpCode>,
    // Next register to allocate. u16 so the "exhausted" state (256) fits.
    pub(super) next_reg: u16,
    pub(super) var_to_reg: HashMap<String, Register>,
    pub(super) var_types: HashMap<String, ast::Type>,
    pub(super) func_map: HashMap<String, usize>,
    pub(super) functions: Vec<Chunk>,
    pub(super) layouts: LayoutCtx,
    pub(super) pending_arg_patches: Vec<(usize, u8)>,
    pub(super) current_fn_fallible: bool,
    // Effect-handler lowering tables built by the pre-pass.
    // (effect_name, op_name) -> synthesised arm fn name.
    pub(super) effect_op_to_arm: HashMap<(String, String), String>,
    // Per-call-site op dispatch: handle nested effects before global fallback.
    pub(super) op_call_to_arm: HashMap<ast::Span, String>,
    // Handle expression span -> synthesised return-arm fn name.
    pub(super) return_arm_by_handle: HashMap<ast::Span, String>,
    // Captures per synthesised arm fn (return arms and op arms alike), as
    // computed by the pre-pass.
    pub(super) arm_captures: HashMap<String, Vec<closures::CaptureInfo>>,
    // Compile-time stack tracking `(arm fn name) -> env register` for each
    // active `handle` expression being compiled.
    pub(super) arm_env_stack: Vec<HashMap<String, Register>>,
    // (receiver_type_name, method_name) -> synthesised mangled fn name produced
    // by the impl-lift pass. Used by codegen to rewrite `x.method(...)` calls.
    pub method_dispatch: HashMap<(String, String), String>,
    // Closure expression spans -> the synthesised lifted fn name and capture
    // layout. Built by the closures pre-pass.
    pub(super) closure_by_span: HashMap<ast::Span, closures::ClosureInfo>,
    // Per-fn-body closure env mapping: enables direct calls to lifted fns.
    pub(super) closure_by_var: HashMap<String, closures::ClosureInfo>,
    pub(super) loop_stack: Vec<LoopCtx>,
    pub(super) concat_fn_id: Option<usize>,
    pub(super) to_str_fn_id: Option<usize>,
    pub(super) host_fns: HashMap<String, HostFnDecl>,
    pub(super) device_mask: [u8; 32],
    pub(super) current_span: ast::Span,
    pub errors: Vec<Error>,
    pub source: String,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            constants: Vec::new(),
            string_constants: Vec::new(),
            code: Vec::new(),
            next_reg: 0,
            var_to_reg: HashMap::new(),
            var_types: HashMap::new(),
            func_map: HashMap::new(),
            functions: Vec::new(),
            layouts: {
                let mut l = LayoutCtx::new();
                effects::install_result_variant(&mut l);
                l
            },
            pending_arg_patches: Vec::new(),
            current_fn_fallible: false,
            effect_op_to_arm: HashMap::new(),
            op_call_to_arm: HashMap::new(),
            return_arm_by_handle: HashMap::new(),
            arm_captures: HashMap::new(),
            arm_env_stack: Vec::new(),
            method_dispatch: HashMap::new(),
            closure_by_span: HashMap::new(),
            closure_by_var: HashMap::new(),
            loop_stack: Vec::new(),
            concat_fn_id: None,
            to_str_fn_id: None,
            host_fns: HashMap::new(),
            device_mask: [0; 32],
            current_span: ast::Span::new(0, 0),
            errors: Vec::new(),
            source: String::new(),
        }
    }

    pub fn with_source(mut self, source: String) -> Self {
        self.source = source;
        self
    }

    pub fn register_host_fn(&mut self, decl: HostFnDecl) {
        self.host_fns.insert(decl.name.clone(), decl);
    }

    pub fn pretty_print_errors(&self) -> String {
        self.errors
            .iter()
            .map(|e| e.pretty_print(&self.source))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn compile(&mut self, ast: &[ast::Decl]) -> Result<Chunk, String> {
        for decl in ast {
            if let ast::Decl::Fn(fn_decl) = decl {
                if fn_decl.name == "main" {
                    let result_reg = self.compile_block(&fn_decl.body)?;
                    self.emit(OpCode::Ret(result_reg));
                }
            }
        }
        Ok(Chunk::Bytecode(BytecodeChunk {
            code: self.code.clone(),
            constants: self.constants.clone(),
            string_constants: self.string_constants.clone(),
            reg_count: self.next_reg as usize,
            param_count: 0,
        }))
    }

    pub fn compile_module(&mut self, ast: &[ast::Decl]) -> Result<Module, Vec<Error>> {
        // Enforce typeck before codegen. Refuses to compile with type errors
        let mut checker = crate::typeck::Checker::new();
        for decl in self.host_fns.values() {
            let fn_ty = TyType::Function {
                params: decl.params.clone(),
                effects: vec![],
                ret: Box::new(decl.ret.clone()),
            };
            checker.insert_var(decl.name.clone(), fn_ty, false, ast::Span { line: 0, col: 0 });
        }
        checker.check_program(ast);
        if !checker.errors.is_empty() {
            self.errors.extend(checker.errors.iter().map(|te| Error::new(
                ErrorCode::TypeError,
                te.span,
                te.message.clone(),
            )));
            return Err(self.errors.clone());
        }

        // Impl-lift pass: synthesise concrete FnDecls and build method dispatch table.
        let mut impl_lowering = impls::ImplLowering::new();
        impl_lowering.lower(ast);
        if !impl_lowering.errors.is_empty() {
            for msg in impl_lowering.errors {
                self.errors.push(Error::new(ErrorCode::TypeError, ast::Span::new(0, 0), msg));
            }
            return Err(self.errors.clone());
        }
        self.method_dispatch = impl_lowering.method_dispatch.clone();
        let mut decls_with_impls: Vec<ast::Decl> = ast.to_vec();
        for fd in impl_lowering.synthetic_fns {
            decls_with_impls.push(ast::Decl::Fn(fd));
        }

        // Generic monomorphization (also rewrites `x.method()` calls so that
        // method dispatch inside generic specialisations binds correctly).
        let owned = match mono::monomorphize_with_methods(decls_with_impls, self.method_dispatch.clone()) {
            Ok(o) => o,
            Err(es) => {
                self.errors.extend(es);
                return Err(self.errors.clone());
            }
        };
        let ast: &[ast::Decl] = &owned;

        // Pre-pass: lift closure expressions to synthetic top-level FnDecls
        // with an env-handle first parameter.
        let mut closure_lowering = closures::ClosureLowering::new();
        closure_lowering.lower(ast);
        self.closure_by_span = closure_lowering.by_span;
        let mut decls_with_closures: Vec<ast::Decl> = ast.to_vec();
        for fd in closure_lowering.synthetic_fns {
            decls_with_closures.push(ast::Decl::Fn(fd));
        }
        let ast: &[ast::Decl] = &decls_with_closures;

        // Pre-pass: lift handler arms to synthetic top-level FnDecls.
        let mut handler_lowering = handlers::HandleLowering::new();
        handler_lowering.lower(ast);
        self.effect_op_to_arm = handler_lowering.effect_op_to_arm;
        self.op_call_to_arm = handler_lowering.op_call_to_arm;
        self.return_arm_by_handle = handler_lowering.return_arm_by_handle;
        self.arm_captures = handler_lowering.arm_captures;

        let mut fn_decls = Vec::new();

        for decl in ast {
            match decl {
                ast::Decl::Fn(fn_decl) => {
                    let idx = self.functions.len();
                    self.func_map.insert(fn_decl.name.clone(), idx);
                    self.functions.push(Chunk::Bytecode(BytecodeChunk::default()));
                    fn_decls.push((idx, fn_decl.clone()));
                }
                ast::Decl::Type { name, body, .. } => {
                    lower::register_type_decl(&mut self.layouts, name, body);
                }
                _ => {}
            }
        }

        // Register the synthetic arm fns in the function table.
        for arm_fn in handler_lowering.synthetic_fns {
            let idx = self.functions.len();
            self.func_map.insert(arm_fn.name.clone(), idx);
            self.functions.push(Chunk::Bytecode(BytecodeChunk::default()));
            fn_decls.push((idx, arm_fn));
        }

        self.register_builtins();

        let entry = match self.func_map.get("main").copied() {
            Some(idx) => idx,
            None => {
                self.errors.push(Error::new(
                    ErrorCode::CodegenError,
                    ast::Span::new(0, 0),
                    "No main function found",
                ));
                return Err(self.errors.clone());
            }
        };

        for (idx, fn_decl) in fn_decls {
            let chunk = self.compile_fn(&fn_decl)?;
            self.functions[idx] = chunk;
        }

        Ok(Module {
            functions: self.functions.clone(),
            entry,
            device_mask: self.device_mask,
        })
    }

    fn compile_fn(&mut self, fn_decl: &ast::FnDecl) -> Result<Chunk, Vec<Error>> {
        let saved_code = std::mem::take(&mut self.code);
        let saved_constants = std::mem::take(&mut self.constants);
        let saved_string_constants = std::mem::take(&mut self.string_constants);
        let saved_next_reg = self.next_reg;
        let saved_var_to_reg = std::mem::take(&mut self.var_to_reg);
        let saved_var_types = std::mem::take(&mut self.var_types);
        let saved_fallible = self.current_fn_fallible;

        self.current_fn_fallible = effects::fn_is_fallible(fn_decl);
        self.next_reg = 0;

        for param in &fn_decl.params {
            if let ast::Param::Named { pattern, ty } = param {
                if let ast::Pattern::Bind(name) = &pattern.node {
                    match self.alloc_register() {
                        Ok(reg) => {
                            self.var_to_reg.insert(name.clone(), reg);
                            self.var_types.insert(name.clone(), ty.clone());
                        }
                        Err(_) => {
                            self.errors.push(Error::new(
                                ErrorCode::CodegenError,
                                pattern.span,
                                format!("Failed to allocate register for parameter '{}'", name),
                            ));
                        }
                    }
                }
            }
        }

        if !self.errors.is_empty() {
            return Err(self.errors.clone());
        }

        let param_count = fn_decl.params.iter().filter(|p| matches!(p, ast::Param::Named { .. })).count();

        match self.compile_block(&fn_decl.body) {
            Ok(result_reg) => {
                let ret_reg = if self.current_fn_fallible {
                    match self.wrap_ok(result_reg) {
                        Ok(r) => r,
                        Err(msg) => {
                            self.errors.push(Error::new(ErrorCode::CodegenError, self.current_span, msg));
                            return Err(self.errors.clone());
                        }
                    }
                } else {
                    result_reg
                };
                self.emit(OpCode::Ret(ret_reg));
            }
            Err(msg) => {
                self.errors.push(Error::new(
                    ErrorCode::CodegenError,
                    self.current_span,
                    msg,
                ));
                return Err(self.errors.clone());
            }
        }

        if let Err(msg) = self.finalize_arg_patches() {
            self.errors.push(Error::new(
                ErrorCode::CodegenError,
                self.current_span,
                msg,
            ));
            return Err(self.errors.clone());
        }

        let reg_count = self.next_reg as usize;
        let chunk = Chunk::Bytecode(BytecodeChunk {
            code: std::mem::take(&mut self.code),
            constants: std::mem::take(&mut self.constants),
            string_constants: std::mem::take(&mut self.string_constants),
            reg_count,
            param_count,
        });

        self.code = saved_code;
        self.constants = saved_constants;
        self.string_constants = saved_string_constants;
        self.next_reg = saved_next_reg;
        self.var_to_reg = saved_var_to_reg;
        self.var_types = saved_var_types;
        self.current_fn_fallible = saved_fallible;

        Ok(chunk)
    }

    fn register_builtins(&mut self) {
        let concat = NativeChunk {
            param_count: 2,
            func: Rc::new(|pool: &mut crate::myriad::BoxPool, args: &[Value]| {
                let a = extract_string(pool, &args[0]).ok_or_else(|| format!("__concat: arg0 not a String: {:?}", args[0]))?;
                let b = extract_string(pool, &args[1]).ok_or_else(|| format!("__concat: arg1 not a String: {:?}", args[1]))?;
                let mut out = String::with_capacity(a.len() + b.len());
                out.push_str(&a);
                out.push_str(&b);
                let idx = pool.intern(crate::myriad::BoxedValue::String(out));
                Ok(Value::from_box(idx))
            }),
        };
        let to_str = NativeChunk {
            param_count: 1,
            func: Rc::new(|pool: &mut crate::myriad::BoxPool, args: &[Value]| {
                let v = &args[0];
                let s = if let Some(n) = v.as_int() { n.to_string() }
                    else if let Some(f) = v.as_float() { f.to_string() }
                    else if let Some(b) = v.as_bool() { b.to_string() }
                    else if let Some(c) = v.as_char() { c.to_string() }
                    else if v.is_unit() { "()".to_string() }
                    else if let Some(s) = extract_string(pool, v) { s }
                    else { return Err(format!("__to_str: cannot convert {:?}", v)); };
                let idx = pool.intern(crate::myriad::BoxedValue::String(s));
                Ok(Value::from_box(idx))
            }),
        };
        let cid = self.functions.len();
        self.func_map.insert("__concat".into(), cid);
        self.functions.push(Chunk::Native(concat));
        self.concat_fn_id = Some(cid);

        let tid = self.functions.len();
        self.func_map.insert("__to_str".into(), tid);
        self.functions.push(Chunk::Native(to_str));
        self.to_str_fn_id = Some(tid);
    }
}

fn extract_string(pool: &crate::myriad::BoxPool, v: &Value) -> Option<String> {
    let idx = v.as_box()?;
    match pool.get(idx)? {
        crate::myriad::BoxedValue::String(s) => Some(s.clone()),
        _ => None,
    }
}
