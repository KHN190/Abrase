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
pub mod licm;
pub mod builtins;

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
    pub compiler_depth_at_entry: usize,
    // block_locals_stack.len() at loop entry.
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
    pub(super) const_mask_bits: Vec<bool>,
    pub(super) string_constants: Vec<String>,
    pub(super) code: Vec<OpCode>,
    pub(super) next_reg: u16,
    pub(super) max_reg: u16,
    // Compile-time tracking: which regs may currently hold a heap handle.
    // Updated by `emit()` based on opcode kind. Used by reclaim to skip
    // Drop emission for regs that provably do not hold a handle.
    pub(super) reg_holds_handle: Vec<bool>,
    pub(super) var_to_reg: HashMap<String, Register>,
    pub(super) var_types: HashMap<String, ast::Type>,
    pub(super) var_bound_at_region: HashMap<String, usize>,
    pub(super) current_self_fn_id: Option<u16>,
    pub(super) tail_call_spans: std::collections::HashSet<ast::Span>,
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
    pub(super) arm_resume_in_tail: HashMap<String, bool>,
    pub(super) op_call_to_arm: HashMap<ast::Span, String>,
    pub(super) return_arm_by_handle: HashMap<ast::Span, String>,
    pub(super) arm_captures: HashMap<String, Vec<closures::CaptureInfo>>,
    pub(super) arm_env_stack: Vec<HashMap<String, Register>>,
    pub method_dispatch: HashMap<(String, String), String>,
    pub(super) closure_by_span: HashMap<ast::Span, closures::ClosureInfo>,
    pub(super) closure_by_var: HashMap<String, closures::ClosureInfo>,
    pub(super) loop_stack: Vec<LoopCtx>,
    pub(super) concat_fn_id: Option<usize>,
    pub(super) to_str_fn_id: Option<usize>,
    pub(super) host_fns: HashMap<String, HostFnDecl>,
    pub(super) builtin_types: HashMap<String, (Vec<TyType>, TyType)>,
    pub(super) fn_signatures: HashMap<usize, (Vec<TyType>, TyType)>,
    pub(super) current_closure_layout: HashMap<String, usize>,
    pub(super) current_span: ast::Span,
    pub(super) compiler_region_depth: usize,
    pub(super) fn_compiler_depth_baseline: usize,
    pub(super) block_locals_stack: Vec<Vec<Register>>,
    pub(super) fn_block_baseline: usize,
    pub(super) handler_table_stack: Vec<Register>,
    pub(super) fn_handler_baseline: usize,
    pub errors: Vec<Error>,
    pub source: String,
    pub(super) debug_sink: Option<debug::CompileDebugSink>,
    pub(super) remaining_uses: HashMap<String, usize>,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            constants: Vec::new(),
            const_mask_bits: Vec::new(),
            string_constants: Vec::new(),
            code: Vec::new(),
            next_reg: 0,
            max_reg: 0,
            reg_holds_handle: Vec::new(),
            var_to_reg: HashMap::new(),
            var_types: HashMap::new(),
            var_bound_at_region: HashMap::new(),
            current_self_fn_id: None,
            tail_call_spans: std::collections::HashSet::new(),
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
            arm_resume_in_tail: HashMap::new(),
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
            fn_signatures: HashMap::new(),
            current_closure_layout: HashMap::new(),
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
            constants: self.constants.iter().map(|v| v.raw()).collect(),
            const_mask: pack_mask_bits(&self.const_mask_bits),
            string_constants: self.string_constants.clone(),
            reg_count: self.max_reg as usize,
            param_count: 0,
        }))
    }

    pub fn compile_module(&mut self, ast: &[ast::Decl]) -> Result<Module, Vec<Error>> {
        self.register_builtins();
        let mut checker = crate::typeck::Checker::new();
        self.register_builtins_to_checker(&mut checker);

        checker.check_program(ast);
        if !checker.errors.is_empty() {
            self.errors.extend(checker.errors.iter().map(|te| Error::new(
                ErrorCode::TypeError,
                te.span,
                te.message.clone(),
            )));
            return Err(self.errors.clone());
        }

        let mut impl_lowering = impls::ImplLowering::new();
        Self::seed_builtin_method_dispatch(&mut impl_lowering.method_dispatch);
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

        let owned = match mono::monomorphize_with_methods(decls_with_impls, self.method_dispatch.clone()) {
            Ok(o) => o,
            Err(es) => {
                self.errors.extend(es);
                return Err(self.errors.clone());
            }
        };
        let ast: &[ast::Decl] = &owned;

        let mut closure_lowering = closures::ClosureLowering::new();
        closure_lowering.lower(ast);
        self.closure_by_span = closure_lowering.by_span;
        let mut decls_with_closures: Vec<ast::Decl> = ast.to_vec();
        for fd in closure_lowering.synthetic_fns {
            decls_with_closures.push(ast::Decl::Fn(fd));
        }
        let ast: &[ast::Decl] = &decls_with_closures;

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
        self.arm_resume_in_tail = handler_lowering.arm_resume_in_tail;
        self.effect_ids = handler_lowering.effect_ids;
        self.op_ids = handler_lowering.op_ids;
        self.effect_op_counts = handler_lowering.effect_op_counts;

        let mut fn_decls = Vec::new();

        for decl in ast {
            match decl {
                ast::Decl::Fn(fn_decl) => {
                    if matches!(fn_decl.name.as_str(), "device_in" | "device_out")
                        || self.host_fns.contains_key(&fn_decl.name)
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
                    register_user_fn_signature(&mut self.fn_signatures, idx, fn_decl);
                    fn_decls.push((idx, fn_decl.clone()));
                }
                ast::Decl::Type { name, body, .. } => {
                    lower::register_type_decl(&mut self.layouts, name, body);
                }
                _ => {}
            }
        }

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
        })
    }

    fn compile_fn(&mut self, fn_decl: &ast::FnDecl) -> Result<Chunk, Vec<Error>> {
        let saved_code = std::mem::take(&mut self.code);
        let saved_constants = std::mem::take(&mut self.constants);
        let saved_const_mask_bits = std::mem::take(&mut self.const_mask_bits);
        let saved_string_constants = std::mem::take(&mut self.string_constants);
        let saved_next_reg = self.next_reg;
        let saved_max_reg = self.max_reg;
        let saved_reg_holds_handle = std::mem::take(&mut self.reg_holds_handle);
        let saved_var_to_reg = std::mem::take(&mut self.var_to_reg);
        let saved_var_types = std::mem::take(&mut self.var_types);
        let saved_var_bound_at_region = std::mem::take(&mut self.var_bound_at_region);
        let saved_current_self_fn_id = self.current_self_fn_id.take();
        let saved_tail_call_spans = std::mem::take(&mut self.tail_call_spans);
        let saved_fallible = self.current_fn_fallible;
        let saved_fn_name = std::mem::take(&mut self.current_fn_name);
        let saved_compiler_region_depth = self.compiler_region_depth;
        let saved_fn_baseline = self.fn_compiler_depth_baseline;
        let saved_block_locals_stack = std::mem::take(&mut self.block_locals_stack);
        let saved_fn_block_baseline = self.fn_block_baseline;
        let saved_handler_table_stack = std::mem::take(&mut self.handler_table_stack);
        let saved_fn_handler_baseline = self.fn_handler_baseline;
        let saved_remaining_uses = std::mem::take(&mut self.remaining_uses);
        let saved_closure_layout = std::mem::take(&mut self.current_closure_layout);
        if let Some(info) = self.closure_by_span.values().find(|i| i.lifted_fn == fn_decl.name).cloned() {
            self.current_closure_layout = info.captures.iter().enumerate()
                .map(|(i, c)| (c.name.clone(), i))
                .collect();
        }

        self.remaining_uses = liveness::count_uses(&fn_decl.body);
        self.current_fn_fallible = effects::fn_is_fallible(fn_decl);
        self.current_fn_name = fn_decl.name.clone();
        self.current_self_fn_id = self.func_map.get(&fn_decl.name)
            .and_then(|id| u16::try_from(*id).ok());
        self.tail_call_spans = collect_tail_self_calls(&fn_decl.body, &fn_decl.name);
        self.next_reg = 0;
        self.max_reg = 0;
        self.compiler_region_depth = 0;
        self.fn_compiler_depth_baseline = 0;
        self.fn_block_baseline = 0;
        self.fn_handler_baseline = 0;
        // Params layer ensures param regs are Dropped on exit (prevents handle leaks).
        self.block_locals_stack.push(Vec::new());

        for param in &fn_decl.params {
            if let ast::Param::Named { pattern, ty } = param {
                if let ast::Pattern::Bind(name) = &pattern.node {
                    match self.alloc_register() {
                        Ok(reg) => {
                            self.var_to_reg.insert(name.clone(), reg);
                            self.var_types.insert(name.clone(), ty.clone());
                            if let Some(top) = self.block_locals_stack.last_mut() {
                                top.push(reg);
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
                // Drop params layer, skip ret to match abnormal exit paths.
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

        let reg_count = self.max_reg as usize;
        let taken_constants = std::mem::take(&mut self.constants);
        let taken_mask_bits = std::mem::take(&mut self.const_mask_bits);
        let chunk = Chunk::Bytecode(BytecodeChunk {
            code: std::mem::take(&mut self.code),
            constants: taken_constants.iter().map(|v| v.raw()).collect(),
            const_mask: pack_mask_bits(&taken_mask_bits),
            string_constants: std::mem::take(&mut self.string_constants),
            reg_count,
            param_count,
        });

        self.code = saved_code;
        self.constants = saved_constants;
        self.const_mask_bits = saved_const_mask_bits;
        self.string_constants = saved_string_constants;
        self.next_reg = saved_next_reg;
        self.max_reg = saved_max_reg;
        self.reg_holds_handle = saved_reg_holds_handle;
        self.var_to_reg = saved_var_to_reg;
        self.var_types = saved_var_types;
        self.var_bound_at_region = saved_var_bound_at_region;
        self.current_self_fn_id = saved_current_self_fn_id;
        self.tail_call_spans = saved_tail_call_spans;
        self.current_fn_fallible = saved_fallible;
        self.current_fn_name = saved_fn_name;
        self.compiler_region_depth = saved_compiler_region_depth;
        self.fn_compiler_depth_baseline = saved_fn_baseline;
        self.block_locals_stack = saved_block_locals_stack;
        self.fn_block_baseline = saved_fn_block_baseline;
        self.handler_table_stack = saved_handler_table_stack;
        self.fn_handler_baseline = saved_fn_handler_baseline;
        self.remaining_uses = saved_remaining_uses;
        self.current_closure_layout = saved_closure_layout;

        Ok(chunk)
    }

}

fn register_user_fn_signature(
    sigs: &mut HashMap<usize, (Vec<TyType>, TyType)>,
    fn_id: usize,
    fn_decl: &ast::FnDecl,
) {
    let params: Vec<TyType> = fn_decl.params.iter().filter_map(|p| {
        if let ast::Param::Named { ty, .. } = p {
            Some(ast_type_to_ty(ty))
        } else { None }
    }).collect();
    let ret = fn_decl.return_type.as_ref()
        .map(ast_type_to_ty)
        .unwrap_or(TyType::Unit);
    sigs.insert(fn_id, (params, ret));
}

fn ast_type_to_ty(t: &ast::Type) -> TyType {
    match t {
        ast::Type::Named(n) => match n.as_str() {
            "Int"    => TyType::Int,
            "Float"  => TyType::Float,
            "Bool"   => TyType::Bool,
            "Char"   => TyType::Char,
            "String" => TyType::String,
            "Unit"   => TyType::Unit,
            "Never"  => TyType::Never,
            _ => TyType::Named(n.clone()),
        },
        ast::Type::Tuple(items) if items.is_empty() => TyType::Unit,
        _ => TyType::Unknown,
    }
}

pub(super) fn pack_mask_bits(bits: &[bool]) -> Vec<u64> {
    let nwords = (bits.len() + 63) / 64;
    let mut out = vec![0u64; nwords];
    for (i, &b) in bits.iter().enumerate() {
        if b { out[i / 64] |= 1u64 << (i % 64); }
    }
    out
}

fn collect_tail_self_calls(block: &ast::Block, self_name: &str) -> std::collections::HashSet<ast::Span> {
    let mut out = std::collections::HashSet::new();
    walk_tail_block(block, self_name, &mut out);
    out
}

fn walk_tail_block(block: &ast::Block, self_name: &str, out: &mut std::collections::HashSet<ast::Span>) {
    if let Some(ret) = &block.ret {
        walk_tail_expr(ret, self_name, out);
    }
}

fn walk_tail_expr(expr: &ast::Spanned<ast::Expr>, self_name: &str, out: &mut std::collections::HashSet<ast::Span>) {
    match &expr.node {
        ast::Expr::Call { callee, .. } => {
            if let ast::Expr::Identifier(n) = &callee.node {
                if n == self_name {
                    out.insert(expr.span);
                }
            }
        }
        ast::Expr::If { consequence, alternative, .. } => {
            walk_tail_expr(consequence, self_name, out);
            if let Some(a) = alternative.as_deref() {
                walk_tail_expr(a, self_name, out);
            }
        }
        ast::Expr::Match { arms, .. } => {
            for arm in arms {
                walk_tail_expr(&arm.body, self_name, out);
            }
        }
        ast::Expr::Block(block) => walk_tail_block(block, self_name, out),
        _ => {}
    }
}
