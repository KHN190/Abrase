pub mod hir;
pub mod lower;
pub mod codegen;
pub mod mono;
pub mod impls;
pub mod closures;
pub mod effects;
pub mod handlers;
pub mod debug;
pub mod liveness;

use crate::ast;
use crate::bytecode::{BytecodeChunk, Chunk, OpCode, Register, Module};
use crate::error::{Error, ErrorCode};
use crate::bytecode::Value;
use std::collections::HashMap;

use self::hir::LayoutCtx;
use crate::ty::Type as TyType;

pub struct LoopCtx {
    pub result_reg: Register,
    pub continue_target: usize,
    pub break_patches: Vec<usize>,
    pub continue_patches: Vec<usize>,
    // Region depth recorded BEFORE the loop's own per-iter push.
    // Used by break/continue to emit the right number of region pops when
    // exiting through nested statement-position regions or inner loops.
    pub compiler_depth_at_entry: usize,
    // block_locals_stack.len() at loop entry. break/continue emit Drop for
    // every block scope opened inside the loop body so move-typed binders
    // (and boxed-Copy ones) get rc-dec'd before the region pops force-free
    // anything still tracked.
    pub block_depth_at_entry: usize,
    pub handler_depth_at_entry: usize,
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
    pub(super) current_fn_name: String,
    pub(super) effect_ids: HashMap<String, u16>,
    pub(super) op_ids: HashMap<(String, String), u8>,
    pub(super) effect_op_counts: HashMap<String, u8>,
    pub(super) effect_op_to_arm: HashMap<(String, String), String>,
    pub(super) effect_arms_by_handle: HashMap<ast::Span, HashMap<(String, String), String>>,
    pub(super) arm_resume_counts: HashMap<String, usize>,
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
    // Builtin natives registered by `register_builtins`: name -> (params, ret).
    pub(super) builtin_types: HashMap<String, (Vec<TyType>, TyType)>,
    pub(super) device_mask: [u8; 32],
    pub(super) current_span: ast::Span,
    // Count of regions currently active inside the function being compiled.
    // Bumped by every emit_region_push / emit_region_pop — counts user
    // `region { ... }` blocks the same as compiler-inserted ones (loop bodies,
    // statement-position blocks). break/continue/return/throw unwind all of
    // them by depth diff.
    pub(super) compiler_region_depth: usize,
    // Compiler-region depth at function entry. return/throw emits
    // (compiler_region_depth - fn_compiler_depth_baseline) pops before exiting.
    pub(super) fn_compiler_depth_baseline: usize,
    // Per-block stack of binder registers introduced by `let` bindings.
    // ALL bindings (Move and Copy) are pushed — Drop on a non-heap Value is
    // a no-op, but Drop on a Copy register that happens to hold a boxed Int
    // (i48 overflow) properly rc-dec's the box. Normal block exit and every
    // abnormal exit (break/continue/return/throw/?/resume) emit Drop for
    // every reg in the layers being unwound, skipping the carried value.
    pub(super) block_locals_stack: Vec<Vec<Register>>,
    // block_locals_stack.len() at function entry. return/throw/? unwind back
    // to this baseline.
    pub(super) fn_block_baseline: usize,
    pub(super) handler_table_stack: Vec<Register>,
    pub(super) fn_handler_baseline: usize,
    pub errors: Vec<Error>,
    pub source: String,
    pub(super) debug_sink: Option<debug::CompileDebugSink>,
    // Remaining use counts for each variable in the current function body,
    // populated by liveness::count_uses before compiling each function.
    pub(super) remaining_uses: HashMap<String, usize>,
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
            current_fn_name: String::new(),
            effect_ids: HashMap::new(),
            op_ids: HashMap::new(),
            effect_op_counts: HashMap::new(),
            effect_op_to_arm: HashMap::new(),
            effect_arms_by_handle: HashMap::new(),
            arm_resume_counts: HashMap::new(),
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
            builtin_types: HashMap::new(),
            device_mask: [0; 32],
            current_span: ast::Span::new(0, 0),
            compiler_region_depth: 0,
            fn_compiler_depth_baseline: 0,
            block_locals_stack: Vec::new(),
            fn_block_baseline: 0,
            handler_table_stack: Vec::new(),
            fn_handler_baseline: 0,
            errors: Vec::new(),
            source: String::new(),
            debug_sink: None,
            remaining_uses: HashMap::new(),
        }
    }

    /// Decrement the remaining-use counter for `name`. Returns `true` when the
    /// count reaches zero, meaning this is the last use and Move can be emitted
    /// instead of Copy for Share-type values.
    pub(super) fn consume_use(&mut self, name: &str) -> bool {
        match self.remaining_uses.get_mut(name) {
            Some(c) if *c > 0 && *c != usize::MAX => {
                *c -= 1;
                *c == 0
            }
            _ => false,
        }
    }

    pub fn with_source(mut self, source: String) -> Self {
        self.source = source;
        self
    }

    pub fn with_debug(mut self, on: bool) -> Self {
        self.debug_sink = if on { Some(debug::stderr_sink()) } else { None };
        self
    }

    pub fn with_debug_sink(mut self, sink: debug::CompileDebugSink) -> Self {
        self.debug_sink = Some(sink);
        self
    }

    /// Build a Vec<String> indexed by function id (matches Module.functions
    /// order). Empty slot for unmapped indices.
    pub fn fn_names(&self) -> Vec<String> {
        let n = self.functions.len();
        let mut out = vec![String::new(); n];
        for (name, &idx) in &self.func_map {
            if idx < n {
                out[idx] = name.clone();
            }
        }
        out
    }

    pub(super) fn emit_debug(&mut self, msg: &str) {
        if let Some(sink) = &mut self.debug_sink {
            sink(msg);
        }
    }

    pub fn register_host_fn(&mut self, decl: HostFnDecl) {
        self.host_fns.insert(decl.name.clone(), decl);
    }

    // Look up a user-defined fn by name. Used by the runtime to auto-install
    // convention-named hooks (e.g. `oom_handler`).
    pub fn lookup_fn_id(&self, name: &str) -> Option<u16> {
        let id = *self.func_map.get(name)?;
        u16::try_from(id).ok()
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
        // Builtins must be before user fns 
        self.register_builtins();

        // Refuses to compile with type errors
        let mut checker = crate::typeck::Checker::new();
        for decl in self.host_fns.values() {
            let fn_ty = TyType::Function {
                params: decl.params.clone(),
                effects: vec![],
                ret: Box::new(decl.ret.clone()),
            };
            checker.insert_var(decl.name.clone(), fn_ty, false, ast::Span { line: 0, col: 0 });
        }
        for (name, (params, ret)) in &self.builtin_types {
            let fn_ty = TyType::Function {
                params: params.clone(),
                effects: vec![],
                ret: Box::new(ret.clone()),
            };
            checker.insert_var(name.clone(), fn_ty, false, ast::Span { line: 0, col: 0 });
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
        if self.debug_sink.is_some() {
            handler_lowering = handler_lowering.with_debug_sink(debug::stderr_sink());
        }
        handler_lowering.lower(ast);
        for err in handler_lowering.errors {
            self.errors.push(Error::new(ErrorCode::CodegenError, ast::Span::new(0, 0), err));
        }
        self.effect_op_to_arm = handler_lowering.effect_op_to_arm;
        self.op_call_to_arm = handler_lowering.op_call_to_arm;
        self.return_arm_by_handle = handler_lowering.return_arm_by_handle;
        self.effect_arms_by_handle = handler_lowering.effect_arms_by_handle;
        self.arm_captures = handler_lowering.arm_captures;
        self.arm_resume_counts = handler_lowering.arm_resume_counts;
        self.effect_ids = handler_lowering.effect_ids;
        self.op_ids = handler_lowering.op_ids;
        self.effect_op_counts = handler_lowering.effect_op_counts;

        let mut fn_decls = Vec::new();

        for decl in ast {
            match decl {
                ast::Decl::Fn(fn_decl) => {
                    // Reserved names — host fns (device_in/out) and builtins
                    // (print, println, …) — can't be shadowed.
                    if self.host_fns.contains_key(&fn_decl.name)
                        || self.func_map.contains_key(&fn_decl.name)
                    {
                        self.errors.push(Error::new(
                            ErrorCode::TypeError,
                            ast::Span::new(0, 0),
                            format!(
                                "cannot redefine fn `{}` — name is reserved or already declared",
                                fn_decl.name
                            ),
                        ));
                        continue;
                    }
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
            if self.debug_sink.is_some() {
                self.emit_debug(&format!("[COMPILE] idx={}, name={}", idx, fn_decl.name));
            }
            let chunk = self.compile_fn(&fn_decl)?;
            self.functions[idx] = chunk;
        }

        if let Some(sink) = &mut self.debug_sink {
            sink("[FUNC_MAP]");
            let mut entries: Vec<(&String, &usize)> = self.func_map.iter().collect();
            entries.sort_by_key(|(_, idx)| **idx);
            for (name, idx) in entries {
                sink(&format!("  {} -> {}", name, idx));
            }
            sink("[BYTECODE]");
            for (i, func) in self.functions.iter().enumerate() {
                match func {
                    Chunk::Bytecode(bc) => {
                        sink(&format!("[{}] {} instrs:", i, bc.code.len()));
                        for (j, op) in bc.code.iter().enumerate() {
                            sink(&format!("    [{}] {:?}", j, op));
                        }
                    }
                    Chunk::Native(n) => sink(&format!("[{}] Native {}", i, n.name)),
                }
            }
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
        let saved_fn_name = std::mem::take(&mut self.current_fn_name);
        let saved_compiler_region_depth = self.compiler_region_depth;
        let saved_fn_baseline = self.fn_compiler_depth_baseline;
        let saved_block_locals_stack = std::mem::take(&mut self.block_locals_stack);
        let saved_fn_block_baseline = self.fn_block_baseline;
        let saved_handler_table_stack = std::mem::take(&mut self.handler_table_stack);
        let saved_fn_handler_baseline = self.fn_handler_baseline;
        let saved_remaining_uses = std::mem::take(&mut self.remaining_uses);

        self.remaining_uses = liveness::count_uses(&fn_decl.body);
        self.current_fn_fallible = effects::fn_is_fallible(fn_decl);
        self.current_fn_name = fn_decl.name.clone();
        self.next_reg = 0;
        // Each fn starts with a fresh region tally + block stack;
        // return/throw unwind back to this baseline (0).
        self.compiler_region_depth = 0;
        self.fn_compiler_depth_baseline = 0;
        self.fn_block_baseline = 0;
        self.fn_handler_baseline = 0;
        // Open a "params layer" on block_locals_stack so each param reg is
        // Drop'd on natural fn exit AND on every abnormal exit. Without this,
        // a param holding a heap handle (closure env, record, etc.) leaks
        // because no one rc_decs it when the fn returns.
        self.block_locals_stack.push(Vec::new());

        for param in &fn_decl.params {
            if let ast::Param::Named { pattern, ty } = param {
                if let ast::Pattern::Bind(name) = &pattern.node {
                    match self.alloc_register() {
                        Ok(reg) => {
                            self.var_to_reg.insert(name.clone(), reg);
                            self.var_types.insert(name.clone(), ty.clone());
                            // For handler arm functions, don't track __env and __return_env as
                            // locals to drop, since they're shared resources managed by the caller
                            let is_handler_arm = fn_decl.name.starts_with("__handle_");
                            let skip_drop = is_handler_arm && (name == "__env" || name == "__return_env");
                            if !skip_drop {
                                if let Some(top) = self.block_locals_stack.last_mut() {
                                    top.push(reg);
                                }
                            }
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
                // Drop params (the layer pushed at fn entry), skipping ret.
                // Mirrors what every abnormal exit already does via
                // emit_drops_to_exit_fn — the natural fallthrough path must
                // match, otherwise normal vs abnormal returns leak differently.
                if let Err(msg) = self.emit_drops_for_exit(1, Some(ret_reg)) {
                    self.errors.push(Error::new(ErrorCode::CodegenError, self.current_span, msg));
                    return Err(self.errors.clone());
                }
                self.block_locals_stack.pop();
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
        self.current_fn_name = saved_fn_name;
        self.compiler_region_depth = saved_compiler_region_depth;
        self.fn_compiler_depth_baseline = saved_fn_baseline;
        self.block_locals_stack = saved_block_locals_stack;
        self.fn_block_baseline = saved_fn_block_baseline;
        self.handler_table_stack = saved_handler_table_stack;
        self.fn_handler_baseline = saved_fn_handler_baseline;
        self.remaining_uses = saved_remaining_uses;

        Ok(chunk)
    }

    fn register_builtins(&mut self) {
        // Compiler-internal — invoked by string `+` and `"{x}"` interp; not
        // typed, not user-callable by name.
        let cid = self.register_native_chunk("__concat", 2);
        self.concat_fn_id = Some(cid);
        let tid = self.register_native_chunk("__to_str", 1);
        self.to_str_fn_id = Some(tid);

        // myriad::builtins::register_default_builtins.
        let s = TyType::String;
        let i = TyType::Int;
        let f = TyType::Float;
        let u = TyType::Unit;
        // Console
        self.register_typed_native("print",    vec![s.clone()],            u.clone(), 1);
        self.register_typed_native("println",  vec![s.clone()],            u.clone(), 1);
        // Clock
        self.register_typed_native("now",      vec![],                     i.clone(), 0);
        self.register_typed_native("sleep_ms", vec![i.clone()],            u.clone(), 1);
        // Random
        self.register_typed_native("rand",     vec![],                     f.clone(), 0);
        self.register_typed_native("srand",    vec![f.clone()],            u.clone(), 1);
        // Math
        self.register_typed_native("abs",      vec![i.clone()],            i.clone(), 1);
        self.register_typed_native("ceil",     vec![f.clone()],            i.clone(), 1);
        self.register_typed_native("flr",      vec![f.clone()],            i.clone(), 1);
        self.register_typed_native("cos",      vec![f.clone()],            f.clone(), 1);
        self.register_typed_native("sin",      vec![f.clone()],            f.clone(), 1);
        self.register_typed_native("sqrt",     vec![f.clone()],            f.clone(), 1);
        self.register_typed_native("max",      vec![i.clone(), i.clone()], i.clone(), 2);
        self.register_typed_native("min",      vec![i.clone(), i.clone()], i.clone(), 2);
        // System
        self.register_typed_native("halt",     vec![i.clone()],            u.clone(), 1);
        self.register_typed_native("abort",    vec![s.clone()],            u.clone(), 1);
    }

    fn register_native_chunk(&mut self, name: &str, param_count: usize) -> usize {
        use crate::bytecode::NativeChunk;
        let id = self.functions.len();
        self.func_map.insert(name.into(), id);
        self.functions.push(Chunk::Native(NativeChunk {
            name: name.into(),
            param_count,
        }));
        id
    }

    fn register_typed_native(
        &mut self,
        name: &str,
        params: Vec<TyType>,
        ret: TyType,
        param_count: usize,
    ) {
        self.register_native_chunk(name, param_count);
        self.builtin_types.insert(name.into(), (params, ret));
    }
}
