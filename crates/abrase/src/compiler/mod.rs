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
    pub effects: Vec<ast::EffectItem>,
}

pub struct Compiler {
    pub(super) constants: Vec<Value>,
    pub(super) const_mask_bits: Vec<bool>,
    pub(super) string_constants: Vec<String>,
    pub(super) code: Vec<OpCode>,
    pub(super) next_reg: u16,
    pub(super) max_reg: u16,
    pub(super) module_table_reg: Option<Register>,
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
    pub(super) current_fn_module: Vec<String>,
    pub(super) fn_origin: HashMap<usize, Vec<String>>,
    pub(super) module_imports: HashMap<Vec<String>, HashMap<String, Vec<String>>>,
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
    pub(super) cell_vars: std::collections::HashSet<String>,
    pub(super) cell_bindings: std::collections::HashSet<String>,
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
    pub(super) current_closure_capture_types: HashMap<usize, ast::Type>,
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
    pub(super) int32_mode: bool,
    pub(super) no_built_in: bool,
    pub(super) const_values: HashMap<String, codegen::inference::ConstValue>,
    pub(super) static_offsets: HashMap<String, u16>,
    pub(super) static_types: HashMap<String, ast::Type>,
    pub(super) static_mut_set: std::collections::HashSet<String>,
    pub(super) typeck_expr_types: HashMap<(Vec<String>, ast::Span, std::mem::Discriminant<ast::Expr>), crate::ty::Type>,
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
            module_table_reg: None,
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
            current_fn_module: Vec::new(),
            fn_origin: HashMap::new(),
            module_imports: HashMap::new(),
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
            cell_vars: std::collections::HashSet::new(),
            cell_bindings: std::collections::HashSet::new(),
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
            current_closure_capture_types: HashMap::new(),
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
            int32_mode: false,
            no_built_in: false,
            const_values: HashMap::new(),
            static_offsets: HashMap::new(),
            static_types: HashMap::new(),
            static_mut_set: std::collections::HashSet::new(),
            typeck_expr_types: HashMap::new(),
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

    pub fn with_int32_mode(mut self, on: bool) -> Self {
        self.int32_mode = on;
        self
    }

    pub fn with_no_built_in(mut self, on: bool) -> Self {
        self.no_built_in = on;
        self
    }

    pub fn static_names_by_offset(&self) -> Vec<String> {
        let n = self.static_offsets.len();
        let mut out = vec![String::new(); n];
        for (name, &offset) in &self.static_offsets {
            if (offset as usize) < n {
                out[offset as usize] = name.clone();
            }
        }
        out
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

    pub(super) fn fqn(module: &[String], name: &str) -> String {
        if module.is_empty() { name.into() }
        else { format!("{}__{}", module.join("_"), name) }
    }

    pub(in crate::compiler) fn resolve_static_offset(&self, name: &str) -> Option<u16> {
        if !self.current_fn_module.is_empty() {
            let local = Self::fqn(&self.current_fn_module, name);
            if let Some(&o) = self.static_offsets.get(&local) { return Some(o); }
        }
        if let Some(imports) = self.module_imports.get(&self.current_fn_module) {
            if let Some(mod_path) = imports.get(name) {
                let imp = Self::fqn(mod_path, name);
                if let Some(&o) = self.static_offsets.get(&imp) { return Some(o); }
            }
        }
        self.static_offsets.get(name).copied()
    }

    pub(in crate::compiler) fn resolve_static_type(&self, name: &str) -> Option<&ast::Type> {
        if !self.current_fn_module.is_empty() {
            let local = Self::fqn(&self.current_fn_module, name);
            if let Some(t) = self.static_types.get(&local) { return Some(t); }
        }
        if let Some(imports) = self.module_imports.get(&self.current_fn_module) {
            if let Some(mod_path) = imports.get(name) {
                let imp = Self::fqn(mod_path, name);
                if let Some(t) = self.static_types.get(&imp) { return Some(t); }
            }
        }
        self.static_types.get(name)
    }

    pub(in crate::compiler) fn resolve_fn_callee(&self, name: &str) -> Option<usize> {
        if !self.current_fn_module.is_empty() {
            let local = Self::fqn(&self.current_fn_module, name);
            if let Some(&id) = self.func_map.get(&local) { return Some(id); }
        }
        if let Some(imports) = self.module_imports.get(&self.current_fn_module) {
            if let Some(mod_path) = imports.get(name) {
                let imp = Self::fqn(mod_path, name);
                if let Some(&id) = self.func_map.get(&imp) { return Some(id); }
            }
        }
        self.func_map.get(name).copied()
    }

    pub fn register_host_fn(
        &mut self,
        name: &str,
        params: Vec<TyType>,
        ret: TyType,
        effects: Vec<ast::EffectItem>,
    ) -> Result<u16, String> {
        if self.host_fns.contains_key(name) || self.func_map.contains_key(name) {
            return Err(format!("name '{}' already registered", name));
        }
        let param_count = params.len();
        let id = self.functions.len();
        let fn_id = u16::try_from(id)
            .map_err(|_| format!("function table overflow registering '{}'", name))?;
        self.func_map.insert(name.into(), id);
        self.functions.push(Chunk::Native(crate::bytecode::NativeChunk {
            name: name.into(),
            param_count,
        }));
        self.fn_signatures.insert(id, (params.clone(), ret.clone()));
        self.host_fns.insert(name.into(), HostFnDecl {
            name: name.into(), params, ret, effects,
        });
        Ok(fn_id)
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
        if !self.no_built_in { self.register_builtins(); }
        self.run_typeck(ast)?;
        let decls_with_impls = self.run_impl_lift(ast)?;
        let owned = self.run_mono(decls_with_impls)?;
        let decls_with_closures = self.run_closure_lift(&owned);
        let handler_synthetic_fns = self.run_handler_lift(&decls_with_closures);
        let ast: &[ast::Decl] = &decls_with_closures;

        let mut fn_decls = self.seed_decls(ast);
        for arm_fn in handler_synthetic_fns {
            let idx = self.functions.len();
            self.func_map.insert(arm_fn.name.clone(), idx);
            self.functions.push(Chunk::Bytecode(BytecodeChunk::default()));
            fn_decls.push((idx, arm_fn));
        }

        let entry = match self.func_map.get("main").copied() {
            Some(idx) => idx,
            None => {
                self.errors.push(Error::new(
                    ErrorCode::CodegenError, ast::Span::new(0, 0), "No main function found",
                ));
                return Err(self.errors.clone());
            }
        };

        self.compile_all_fns(fn_decls)?;
        let module_init_id = self.compile_static_init(ast)?;
        self.emit_debug_dump();

        let mut flags = 0u16;
        if self.int32_mode { flags |= crate::bytecode::CART_FLAG_INT32_SAFE; }
        let exports = self.build_exports(ast, entry, module_init_id);
        Ok(Module { functions: self.functions.clone(), entry, flags, exports })
    }

    fn seed_decls(&mut self, ast: &[ast::Decl]) -> Vec<(usize, ast::FnDecl)> {
        let mut fn_decls = Vec::new();
        let mut module_stack: Vec<Vec<String>> = Vec::new();
        for decl in ast {
            match decl {
                ast::Decl::ModEnter(p) => { module_stack.push(p.clone()); }
                ast::Decl::ModExit => { module_stack.pop(); }
                ast::Decl::Use { path, items } => {
                    let owner = module_stack.last().cloned().unwrap_or_default();
                    let bucket = self.module_imports.entry(owner).or_default();
                    for item in items {
                        let alias = item.alias.clone().unwrap_or_else(|| item.name.clone());
                        bucket.insert(alias, path.clone());
                    }
                }
                ast::Decl::Fn(fn_decl) => {
                    let mod_path = module_stack.last().cloned().unwrap_or_default();
                    let mangled = Self::fqn(&mod_path, &fn_decl.name);
                    if matches!(fn_decl.name.as_str(), "device_in" | "device_out")
                        || self.host_fns.contains_key(&mangled)
                        || self.func_map.contains_key(&mangled)
                    {
                        self.errors.push(Error::new(
                            ErrorCode::TypeError, ast::Span::new(0, 0),
                            format!("cannot redefine fn `{}` — name is reserved or already declared", fn_decl.name),
                        ));
                        continue;
                    }
                    let idx = self.functions.len();
                    self.func_map.insert(mangled, idx);
                    self.fn_origin.insert(idx, mod_path);
                    self.functions.push(Chunk::Bytecode(BytecodeChunk::default()));
                    register_user_fn_signature(&mut self.fn_signatures, idx, fn_decl);
                    fn_decls.push((idx, fn_decl.clone()));
                }
                ast::Decl::Type { name, body, .. } => {
                    lower::register_type_decl(&mut self.layouts, name, body);
                }
                ast::Decl::Const { name, value, is_fn, .. } if !*is_fn => {
                    match self.try_const_fold(value) {
                        Some(cv) => { self.const_values.insert(name.clone(), cv); }
                        None => self.errors.push(Error::new(
                            ErrorCode::CodegenError, value.span,
                            format!("const `{}` value is not a compile-time constant", name),
                        )),
                    }
                }
                ast::Decl::Static { name, ty, is_mut, .. } => {
                    let mod_path = module_stack.last().cloned().unwrap_or_default();
                    let mangled = Self::fqn(&mod_path, name);
                    let offset = self.static_offsets.len() as u16;
                    self.static_offsets.insert(mangled.clone(), offset);
                    self.static_types.insert(mangled.clone(), ty.clone());
                    if *is_mut { self.static_mut_set.insert(mangled); }
                }
                _ => {}
            }
        }
        fn_decls
    }

    fn compile_all_fns(&mut self, fn_decls: Vec<(usize, ast::FnDecl)>) -> Result<(), Vec<Error>> {
        for (idx, fn_decl) in fn_decls {
            if self.debug_sink.is_some() {
                self.emit_debug(&format!("[COMPILE] idx={}, name={}", idx, fn_decl.name));
            }
            let saved_module = std::mem::take(&mut self.current_fn_module);
            self.current_fn_module = self.fn_origin.get(&idx).cloned().unwrap_or_default();
            let chunk = match self.compile_fn(&fn_decl) {
                Ok(c) => c,
                Err(mut es) => {
                    for e in &mut es {
                        if e.module.is_empty() { e.module = self.current_fn_module.clone(); }
                    }
                    self.current_fn_module = saved_module;
                    return Err(es);
                }
            };
            self.current_fn_module = saved_module;
            self.functions[idx] = chunk;
        }
        Ok(())
    }

    fn compile_static_init(&mut self, ast: &[ast::Decl]) -> Result<Option<u16>, Vec<Error>> {
        if self.static_offsets.is_empty() { return Ok(None); }
        let mut ordered: Vec<(String, u16)> = self.static_offsets.iter()
            .map(|(n, o)| (n.clone(), *o)).collect();
        ordered.sort_by_key(|(_, o)| *o);
        let mut init_stack: Vec<Vec<String>> = Vec::new();
        let static_values: Vec<(Vec<String>, ast::Spanned<ast::Expr>)> = ast.iter().filter_map(|d| {
            match d {
                ast::Decl::ModEnter(p) => { init_stack.push(p.clone()); None }
                ast::Decl::ModExit => { init_stack.pop(); None }
                ast::Decl::Static { name, value, .. } => {
                    let mod_path = init_stack.last().cloned().unwrap_or_default();
                    let key = Self::fqn(&mod_path, name);
                    if self.static_offsets.contains_key(&key) {
                        Some((mod_path, value.clone()))
                    } else { None }
                }
                _ => None,
            }
        }).collect();
        let table_size = ordered.len() as u16;
        let init_id = self.functions.len();
        self.func_map.insert("__module_init".into(), init_id);
        self.functions.push(Chunk::Bytecode(BytecodeChunk::default()));
        let chunk = self.compile_module_init(table_size, &ordered, &static_values)?;
        self.functions[init_id] = chunk;
        Ok(Some(init_id as u16))
    }

    fn emit_debug_dump(&mut self) {
        let Some(sink) = &mut self.debug_sink else { return };
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

    fn build_exports(&self, ast: &[ast::Decl], entry: usize, module_init_id: Option<u16>)
        -> Vec<crate::bytecode::Export>
    {
        let mut exports: Vec<crate::bytecode::Export> = Vec::new();
        let mut export_stack: Vec<Vec<String>> = Vec::new();
        for decl in ast {
            match decl {
                ast::Decl::ModEnter(p) => { export_stack.push(p.clone()); }
                ast::Decl::ModExit => { export_stack.pop(); }
                ast::Decl::Fn(fn_decl) if export_stack.is_empty() && fn_decl.is_pub => {
                    if let Some(&idx) = self.func_map.get(&fn_decl.name) {
                        if let Ok(id) = u16::try_from(idx) {
                            exports.push(crate::bytecode::Export { name: fn_decl.name.clone(), fn_id: id });
                        }
                    }
                }
                _ => {}
            }
        }
        if let Ok(id) = u16::try_from(entry) {
            if !exports.iter().any(|e| e.name == "main") {
                exports.push(crate::bytecode::Export { name: "main".into(), fn_id: id });
            }
        }
        if let Some(id) = module_init_id {
            exports.push(crate::bytecode::Export { name: "__module_init".into(), fn_id: id });
        }
        exports
    }

    fn run_typeck(&mut self, ast: &[ast::Decl]) -> Result<(), Vec<Error>> {
        let mut checker = crate::typeck::Checker::new();
        if !self.no_built_in { self.register_builtins_to_checker(&mut checker); }
        checker.check_program(ast);
        self.typeck_expr_types = std::mem::take(&mut checker.expr_types);
        if !checker.errors.is_empty() {
            self.errors.extend(checker.errors.iter().map(|te| Error::new(
                ErrorCode::TypeError, te.span, te.message.clone(),
            ).with_module(te.module.clone())));
            return Err(self.errors.clone());
        }
        Ok(())
    }

    fn run_impl_lift(&mut self, ast: &[ast::Decl]) -> Result<Vec<ast::Decl>, Vec<Error>> {
        let mut impl_lowering = impls::ImplLowering::new();
        Self::seed_builtin_method_dispatch(&mut impl_lowering.method_dispatch);
        impl_lowering.lower(ast);
        if !impl_lowering.errors.is_empty() {
            for msg in impl_lowering.errors {
                self.errors.push(Error::new(ErrorCode::TypeError, ast::Span::new(0, 0), msg));
            }
            return Err(self.errors.clone());
        }
        self.method_dispatch = impl_lowering.method_dispatch;
        let mut decls = ast.to_vec();
        for fd in impl_lowering.synthetic_fns { decls.push(ast::Decl::Fn(fd)); }
        Ok(decls)
    }

    fn run_mono(&mut self, decls: Vec<ast::Decl>) -> Result<Vec<ast::Decl>, Vec<Error>> {
        let builtin_returns: std::collections::HashMap<String, ast::Type> = self.builtin_types.iter()
            .filter_map(|(name, (_, ret))| codegen::inference::ty_to_ast(ret).map(|t| (name.clone(), t)))
            .collect();
        mono::monomorphize_with_methods_and_builtins(
            decls, self.method_dispatch.clone(), builtin_returns,
        ).map_err(|es| { self.errors.extend(es); self.errors.clone() })
    }

    fn run_closure_lift(&mut self, ast: &[ast::Decl]) -> Vec<ast::Decl> {
        let mut closure_lowering = closures::ClosureLowering::new();
        closure_lowering.lower(ast);
        self.closure_by_span = closure_lowering.by_span;
        let mut decls = ast.to_vec();
        for fd in closure_lowering.synthetic_fns { decls.push(ast::Decl::Fn(fd)); }
        decls
    }

    fn run_handler_lift(&mut self, ast: &[ast::Decl]) -> Vec<ast::FnDecl> {
        let mut h = handlers::HandleLowering::new();
        if self.debug_sink.is_some() {
            h = h.with_debug_sink(debug::stderr_sink());
        }
        h.lower(ast);
        for err in h.errors {
            self.errors.push(Error::new(ErrorCode::CodegenError, ast::Span::new(0, 0), err));
        }
        self.effect_op_to_arm = h.effect_op_to_arm;
        self.op_call_to_arm = h.op_call_to_arm;
        self.return_arm_by_handle = h.return_arm_by_handle;
        self.effect_arms_by_handle = h.effect_arms_by_handle;
        self.arm_captures = h.arm_captures;
        self.cell_vars = h.cell_vars;
        self.arm_resume_counts = h.arm_resume_counts;
        self.arm_resume_in_tail = h.arm_resume_in_tail;
        self.effect_ids = h.effect_ids;
        self.op_ids = h.op_ids;
        self.effect_op_counts = h.effect_op_counts;
        h.synthetic_fns
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
        let saved_closure_capture_types = std::mem::take(&mut self.current_closure_capture_types);
        let saved_cell_bindings = std::mem::take(&mut self.cell_bindings);
        if let Some(info) = self.closure_by_span.values().find(|i| i.lifted_fn == fn_decl.name).cloned() {
            self.current_closure_layout = info.captures.iter().enumerate()
                .map(|(i, c)| (c.name.clone(), i))
                .collect();
            self.current_closure_capture_types = info.captures.iter().enumerate()
                .map(|(i, c)| (i, c.ty.clone()))
                .collect();
        }

        self.remaining_uses = liveness::count_uses(&fn_decl.body);
        self.current_fn_fallible = effects::fn_is_fallible(fn_decl);
        self.current_fn_name = fn_decl.name.clone();
        let self_key = Self::fqn(&self.current_fn_module, &fn_decl.name);
        self.current_self_fn_id = self.func_map.get(&self_key)
            .or_else(|| self.func_map.get(&fn_decl.name))
            .and_then(|id| u16::try_from(*id).ok());
        self.tail_call_spans = collect_tail_self_calls(&fn_decl.body, &fn_decl.name);
        self.next_reg = 0;
        self.max_reg = 0;
        self.module_table_reg = None;
        self.compiler_region_depth = 0;
        self.fn_compiler_depth_baseline = 0;
        self.fn_block_baseline = 0;
        self.fn_handler_baseline = 0;
        // Params layer ensures param regs are Dropped on exit (prevents handle leaks).
        self.block_locals_stack.push(Vec::new());

        for param in &fn_decl.params {
            if let ast::Param::Named { pattern, ty } = param {
                let reg = match self.alloc_register() {
                    Ok(r) => r,
                    Err(_) => {
                        self.errors.push(Error::new(
                            ErrorCode::CodegenError,
                            pattern.span,
                            "Failed to allocate register for parameter",
                        ));
                        continue;
                    }
                };
                if let Some(top) = self.block_locals_stack.last_mut() {
                    top.push(reg);
                }
                match &pattern.node {
                    ast::Pattern::Bind(name) => {
                        self.var_to_reg.insert(name.clone(), reg);
                        self.var_types.insert(name.clone(), ty.clone());
                    }
                    _ => {
                        if let Err(msg) = self.compile_destructure(pattern, reg, Some(ty)) {
                            self.errors.push(Error::new(
                                ErrorCode::CodegenError,
                                pattern.span,
                                msg,
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
        self.current_closure_capture_types = saved_closure_capture_types;
        self.cell_bindings = saved_cell_bindings;

        Ok(chunk)
    }

    fn compile_module_init(
        &mut self,
        table_size: u16,
        _ordered: &[(String, u16)],
        static_values: &[(Vec<String>, ast::Spanned<ast::Expr>)],
    ) -> Result<Chunk, Vec<Error>> {
        let saved_code = std::mem::take(&mut self.code);
        let saved_constants = std::mem::take(&mut self.constants);
        let saved_const_mask_bits = std::mem::take(&mut self.const_mask_bits);
        let saved_string_constants = std::mem::take(&mut self.string_constants);
        let saved_next_reg = self.next_reg;
        let saved_max_reg = self.max_reg;
        let saved_reg_holds_handle = std::mem::take(&mut self.reg_holds_handle);
        let saved_var_to_reg = std::mem::take(&mut self.var_to_reg);
        let saved_var_types = std::mem::take(&mut self.var_types);
        let saved_fn_name = std::mem::take(&mut self.current_fn_name);
        self.next_reg = 0;
        self.max_reg = 0;
        self.current_fn_name = "__module_init".into();

        macro_rules! bail {
            ($span:expr, $msg:expr) => {{
                self.errors.push(Error::new(ErrorCode::CodegenError, $span, $msg));
                return Err(self.errors.clone());
            }};
        }

        let table = match self.alloc_register() {
            Ok(r) => r,
            Err(m) => bail!(ast::Span::new(0, 0), m),
        };
        self.emit(OpCode::Alloc(table, table_size));
        let saved_init_module = std::mem::take(&mut self.current_fn_module);
        for (i, (mod_path, value)) in static_values.iter().enumerate() {
            let off = i as u16;
            let mark = self.snapshot_register_high_water();
            self.current_fn_module = mod_path.clone();
            let v = match self.compile_expr(value) {
                Ok(r) => r,
                Err(m) => bail!(value.span, m),
            };
            self.emit(OpCode::St(v, table, off));
            self.restore_register_high_water(mark);
        }
        self.current_fn_module = saved_init_module;
        // Publish: Deo(table -> MODULE_ID:MODULE_PORT_TABLE).
        let port_reg = match self.alloc_register() {
            Ok(r) => r,
            Err(m) => bail!(ast::Span::new(0, 0), m),
        };
        let port_val = ((crate::bytecode::MODULE_ID as i64) << 8)
            | (crate::bytecode::MODULE_PORT_TABLE as i64);
        let port_idx = match self.add_constant(Value::from_int(port_val)) {
            Ok(i) => i,
            Err(m) => bail!(ast::Span::new(0, 0), m),
        };
        self.emit(OpCode::PushConst(port_reg, port_idx));
        self.emit(OpCode::Deo(table, port_reg));
        // Return Unit (a non-handle scalar; the result is discarded).
        let unit = match self.alloc_register() {
            Ok(r) => r,
            Err(m) => bail!(ast::Span::new(0, 0), m),
        };
        let unit_idx = match self.add_constant(Value::from_int(0)) {
            Ok(i) => i,
            Err(m) => bail!(ast::Span::new(0, 0), m),
        };
        self.emit(OpCode::PushConst(unit, unit_idx));
        self.emit(OpCode::Ret(unit));

        if let Err(msg) = self.finalize_arg_patches() {
            bail!(ast::Span::new(0, 0), msg);
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
            param_count: 0,
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
        self.current_fn_name = saved_fn_name;
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
        ast::Type::Tuple(items) => TyType::Tuple(items.iter().map(ast_type_to_ty).collect()),
        ast::Type::Generic { name, args } => TyType::Generic {
            name: name.clone(),
            args: args.iter().map(ast_type_to_ty).collect(),
        },
        ast::Type::Array { elem, .. } => TyType::Generic {
            name: "Array".into(),
            args: vec![ast_type_to_ty(elem)],
        },
        ast::Type::Reference { is_mut, inner, .. } => TyType::Reference {
            is_mut: *is_mut,
            inner: Box::new(ast_type_to_ty(inner)),
        },
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
