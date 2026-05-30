use crate::ast;
use crate::bytecode::{OpCode, Register};
use crate::compiler::Compiler;
use crate::bytecode::Value;

pub(in crate::compiler) enum CallEnv {
    None,
    Reg(Register),
    EnvLoadOffset(usize),
}

pub(in crate::compiler) enum CallTarget<'a> {
    Function { func_id: u16, env: CallEnv },
    Method   { func_id: u16, receiver: &'a ast::Spanned<ast::Expr> },
    EnvLoad  { env_reg: Register, idx: u16 },
    CellLoad { env_reg: Register, idx: u16 },
    CellStore { env_reg: Register, idx: u16, value: &'a ast::Spanned<ast::Expr> },
    SharedCtor,
    HostFn { fn_id: u16 },
    VariantCtor { tag: u32 },
    UnresolvedMethod { receiver: String, field: String },
    DeviceIn,
    DeviceOut,
    EffectOpDispatch { effect_id: u16, op_id: u8 },
}

impl Compiler {
    pub(in crate::compiler) fn compile_call(
        &mut self,
        callee: &ast::Spanned<ast::Expr>,
        args: &[ast::Spanned<ast::Expr>],
        call_span: ast::Span,
    ) -> Result<Register, String> {
        if let Some(sink) = &mut self.debug_sink {
            sink(&format!("[CALL] fn={}, callee={:?}", self.current_fn_name, callee.node));
        }
        let target = self.resolve_call(callee, args, call_span)?;
        if let Some(sink) = &mut self.debug_sink {
            let kind = match &target {
                CallTarget::EffectOpDispatch { .. } => "EffectOpDispatch",
                CallTarget::Function { .. } => "Function",
                CallTarget::Method { .. } => "Method",
                _ => "Other",
            };
            sink(&format!("[CALL] resolved to {:?}", kind));
        }
        match target {
            CallTarget::EnvLoad { env_reg, idx } => self.emit_env_load(env_reg, idx),
            CallTarget::CellLoad { env_reg, idx } => self.emit_cell_load(env_reg, idx),
            CallTarget::CellStore { env_reg, idx, value } => self.emit_cell_store(env_reg, idx, value),
            CallTarget::SharedCtor => self.emit_shared_ctor(args),
            CallTarget::HostFn { fn_id } => self.emit_host_fn_call(fn_id, args),
            CallTarget::VariantCtor { tag } => self.emit_variant_ctor(tag, args),
            CallTarget::Function { func_id, env: CallEnv::None }
                if Some(func_id) == self.current_self_fn_id
                && self.tail_call_spans.contains(&call_span) => {
                self.emit_tail_self_call(args)
            }
            CallTarget::Function { func_id, env } => self.emit_func_call(func_id, env, args),
            CallTarget::Method { func_id, receiver } => self.emit_method_call(func_id, receiver, args),
            CallTarget::DeviceIn => self.emit_device_in(args),
            CallTarget::DeviceOut => self.emit_device_out(args),
            CallTarget::EffectOpDispatch { effect_id, op_id } => {
                self.emit_effect_op_dispatch(effect_id, op_id, args)
            }
            CallTarget::UnresolvedMethod { receiver, field } => Err(format!(
                "No method '{}' on type '{}' (or receiver type could not be inferred)",
                field, receiver
            )),
        }
    }

    fn resolve_call<'a>(
        &self,
        callee: &'a ast::Spanned<ast::Expr>,
        args: &'a [ast::Spanned<ast::Expr>],
        call_span: ast::Span,
    ) -> Result<CallTarget<'a>, String> {
        // Language primitives win over everything else — including user
        // shadowing. compile_module rejects user fns that try to redefine
        // these names, but we double-gate here so the intrinsic always fires.
        if let Some(t) = self.resolve_primitive(callee, args)? { return Ok(t); }
        if let Some(t) = self.resolve_env_load(callee, args)? { return Ok(t); }
        if let Some(t) = self.resolve_cell_builtin(callee, args)? { return Ok(t); }
        if let Some(t) = self.resolve_closure_call(callee)? { return Ok(t); }
        if let Some(t) = self.resolve_env_load_closure_call(callee)? { return Ok(t); }
        if let Some(t) = self.resolve_effect_op_call(callee, call_span)? { return Ok(t); }
        if let Some(t) = self.resolve_method_call(callee)? { return Ok(t); }
        if let Some(t) = self.resolve_host_or_ctor(callee, args)? { return Ok(t); }
        if let ast::Expr::FieldAccess { base, field } = &callee.node {
            let Some(recv) = self.receiver_type_name(base) else {
                return Err(format!(
                    "Cannot infer receiver type for method call '.{}'; annotate the base expression",
                    field
                ));
            };
            return Ok(CallTarget::UnresolvedMethod { receiver: recv, field: field.clone() });
        }
        let ast::Expr::Identifier(name) = &callee.node else {
            return Err("Call target must be a function identifier".to_string());
        };
        let func_id = self.resolve_fn_callee(name)
            .ok_or_else(|| format!("Undefined function: {}", name))?;
        let fid = super::scaffold::to_u16(func_id, &format!("Function id for '{}'", name))?;
        Ok(CallTarget::Function { func_id: fid, env: CallEnv::None })
    }

    fn resolve_env_load<'a>(
        &self,
        callee: &'a ast::Spanned<ast::Expr>,
        args: &'a [ast::Spanned<ast::Expr>],
    ) -> Result<Option<CallTarget<'a>>, String> {
        let ast::Expr::Identifier(name) = &callee.node else { return Ok(None) };
        if name != "__env_load" || args.len() != 2 { return Ok(None); }
        let (env_name, idx) = match (&args[0].node, &args[1].node) {
            (ast::Expr::Identifier(env_name), ast::Expr::Literal(ast::Literal::Int(idx))) => (env_name, idx),
            _ => return Ok(None),
        };
        let env_reg = *self.var_to_reg.get(env_name)
            .ok_or_else(|| format!("internal: env binding '{}' not in scope", env_name))?;
        if *idx < 0 { return Err(format!("Env-load index must be non-negative, got {}", idx)); }
        let env_idx = super::scaffold::to_u16(*idx as usize, "Env-load index")?;
        Ok(Some(CallTarget::EnvLoad { env_reg, idx: env_idx }))
    }

    fn resolve_closure_call<'a>(
        &self,
        callee: &'a ast::Spanned<ast::Expr>,
    ) -> Result<Option<CallTarget<'a>>, String> {
        let ast::Expr::Identifier(name) = &callee.node else { return Ok(None) };
        let Some(info) = self.closure_by_var.get(name) else { return Ok(None) };
        let func_id = *self.func_map.get(&info.lifted_fn)
            .ok_or_else(|| format!("internal: lifted closure fn '{}' not in fn table", info.lifted_fn))?;
        let env_reg = *self.var_to_reg.get(name)
            .ok_or_else(|| format!("internal: closure binding '{}' has no register", name))?;
        let fid = super::scaffold::to_u16(func_id, &format!("Closure fn_id for '{}'", name))?;
        Ok(Some(CallTarget::Function { func_id: fid, env: CallEnv::Reg(env_reg) }))
    }

    fn resolve_env_load_closure_call<'a>(
        &self,
        callee: &'a ast::Spanned<ast::Expr>,
    ) -> Result<Option<CallTarget<'a>>, String> {
        let ast::Expr::Call { callee: inner_callee, args: inner_args } = &callee.node else { return Ok(None) };
        let ast::Expr::Identifier(inner_name) = &inner_callee.node else { return Ok(None) };
        if inner_name != "__env_load" { return Ok(None); }
        if inner_args.len() != 2 { return Ok(None); }
        let ast::Expr::Literal(ast::Literal::Int(off)) = &inner_args[1].node else { return Ok(None) };
        let offset = *off as usize;
        let captured_name = self.current_closure_layout.iter()
            .find_map(|(n, &i)| if i == offset { Some(n.clone()) } else { None });
        let Some(captured_name) = captured_name else { return Ok(None); };
        let Some(info) = self.closure_by_var.get(&captured_name) else { return Ok(None) };
        let func_id = *self.func_map.get(&info.lifted_fn)
            .ok_or_else(|| format!("internal: lifted closure fn '{}' not in fn table", info.lifted_fn))?;
        let fid = super::scaffold::to_u16(func_id, &format!("Closure fn_id for '{}'", captured_name))?;
        Ok(Some(CallTarget::Function { func_id: fid, env: CallEnv::EnvLoadOffset(offset) }))
    }

    fn resolve_effect_op_call<'a>(
        &self,
        callee: &'a ast::Spanned<ast::Expr>,
        _call_span: ast::Span,
    ) -> Result<Option<CallTarget<'a>>, String> {
        let ast::Expr::FieldAccess { base, field } = &callee.node else { return Ok(None) };
        let ast::Expr::Identifier(eff_name) = &base.node else { return Ok(None) };
        let key = (eff_name.clone(), field.clone());
        if !self.effect_op_to_arm.contains_key(&key) { return Ok(None); }
        let effect_id = match self.effect_ids.get(eff_name).copied() {
            Some(id) => id,
            None => return Ok(None),
        };
        let op_id = self.op_ids.get(&key).copied().unwrap_or(0);
        Ok(Some(CallTarget::EffectOpDispatch { effect_id, op_id }))
    }

    fn resolve_method_call<'a>(
        &self,
        callee: &'a ast::Spanned<ast::Expr>,
    ) -> Result<Option<CallTarget<'a>>, String> {
        let ast::Expr::FieldAccess { base, field } = &callee.node else { return Ok(None) };
        let Some(rname) = self.receiver_type_name(base) else { return Ok(None) };
        let Some(mangled) = self.method_dispatch.get(&(rname, field.clone())).cloned() else { return Ok(None) };
        let func_id = *self.func_map.get(&mangled)
            .ok_or_else(|| format!("internal: method '{}' missing from fn table", mangled))?;
        let fid = super::scaffold::to_u16(func_id, &format!("Method fn_id for '{}'", mangled))?;
        Ok(Some(CallTarget::Method { func_id: fid, receiver: base }))
    }

    // Resolve `device_in`/`device_out` by name.
    fn resolve_primitive<'a>(
        &self,
        callee: &'a ast::Spanned<ast::Expr>,
        args: &'a [ast::Spanned<ast::Expr>],
    ) -> Result<Option<CallTarget<'a>>, String> {
        let ast::Expr::Identifier(name) = &callee.node else { return Ok(None) };
        match name.as_str() {
            "device_in" => {
                if args.len() != 2 {
                    return Err(format!(
                        "device_in expects (port, data); got {} arg(s)", args.len()
                    ));
                }
                Ok(Some(CallTarget::DeviceIn))
            }
            "device_out" => {
                if args.len() != 1 {
                    return Err(format!(
                        "device_out expects (port); got {} arg(s)", args.len()
                    ));
                }
                Ok(Some(CallTarget::DeviceOut))
            }
            _ => Ok(None),
        }
    }

    fn resolve_host_or_ctor<'a>(
        &self,
        callee: &'a ast::Spanned<ast::Expr>,
        args: &'a [ast::Spanned<ast::Expr>],
    ) -> Result<Option<CallTarget<'a>>, String> {
        let ast::Expr::Identifier(name) = &callee.node else { return Ok(None) };
        if name == "Shared" && args.len() == 1 { return Ok(Some(CallTarget::SharedCtor)); }
        // Host fns rejects shadowing. No `func_map` precedence check.
        if let Some(host) = self.host_fns.get(name) {
            if host.params.len() != args.len() {
                return Err(format!(
                    "host fn '{}' expects {} arg(s), got {}",
                    name, host.params.len(), args.len()
                ));
            }
            let id = *self.func_map.get(name)
                .ok_or_else(|| format!("internal: host fn '{}' missing from fn table", name))?;
            let fn_id = super::scaffold::to_u16(id, &format!("host fn id for '{}'", name))?;
            return Ok(Some(CallTarget::HostFn { fn_id }));
        }
        if let Some(info) = self.layouts.variants.get(name) {
            return Ok(Some(CallTarget::VariantCtor { tag: info.tag }));
        }
        Ok(None)
    }

    fn emit_env_load(&mut self, env_reg: Register, idx: u16) -> Result<Register, String> {
        let dest = self.alloc_register()?;
        self.emit(OpCode::Ld(dest, env_reg, idx));
        Ok(dest)
    }

    fn resolve_cell_builtin<'a>(
        &self,
        callee: &'a ast::Spanned<ast::Expr>,
        args: &'a [ast::Spanned<ast::Expr>],
    ) -> Result<Option<CallTarget<'a>>, String> {
        let ast::Expr::Identifier(name) = &callee.node else { return Ok(None) };
        let is_load = name == "__cell_load" && args.len() == 2;
        let is_store = name == "__cell_store" && args.len() == 3;
        if !is_load && !is_store { return Ok(None); }
        let (env_name, idx) = match (&args[0].node, &args[1].node) {
            (ast::Expr::Identifier(en), ast::Expr::Literal(ast::Literal::Int(i))) => (en, *i),
            _ => return Ok(None),
        };
        let env_reg = *self.var_to_reg.get(env_name)
            .ok_or_else(|| format!("internal: env binding '{}' not in scope", env_name))?;
        if idx < 0 { return Err(format!("cell-builtin index must be non-negative, got {}", idx)); }
        let env_idx = super::scaffold::to_u16(idx as usize, "cell-builtin index")?;
        if is_load {
            Ok(Some(CallTarget::CellLoad { env_reg, idx: env_idx }))
        } else {
            Ok(Some(CallTarget::CellStore { env_reg, idx: env_idx, value: &args[2] }))
        }
    }

    fn emit_cell_load(&mut self, env_reg: Register, idx: u16) -> Result<Register, String> {
        let cell = self.alloc_register()?;
        self.emit(OpCode::Ld(cell, env_reg, idx));
        let dest = self.alloc_register()?;
        self.emit(OpCode::Ld(dest, cell, 0));
        Ok(dest)
    }

    fn emit_cell_store(
        &mut self,
        env_reg: Register,
        idx: u16,
        value: &ast::Spanned<ast::Expr>,
    ) -> Result<Register, String> {
        let v = self.compile_expr(value)?;
        let cell = self.alloc_register()?;
        self.emit(OpCode::Ld(cell, env_reg, idx));
        self.emit(OpCode::St(v, cell, 0));
        let unit = self.alloc_register()?;
        let i = self.add_constant(Value::UNIT)?;
        self.emit(OpCode::PushConst(unit, i));
        Ok(unit)
    }

    fn emit_shared_ctor(&mut self, args: &[ast::Spanned<ast::Expr>]) -> Result<Register, String> {
        let src = self.compile_expr(&args[0])?;
        let dest = self.alloc_register()?;
        self.emit(OpCode::Alloc(dest, 1));
        let tmp = self.alloc_register()?;
        self.emit(OpCode::Copy(tmp, src));
        self.emit(OpCode::St(tmp, dest, 0));
        Ok(dest)
    }

    fn emit_host_fn_call(&mut self, fn_id: u16, args: &[ast::Spanned<ast::Expr>]) -> Result<Register, String> {
        self.emit_func_call(fn_id, CallEnv::None, args)
    }

    // device_in(port, data) → Deo(data, port). Returns Unit register.
    fn emit_device_in(&mut self, args: &[ast::Spanned<ast::Expr>]) -> Result<Register, String> {
        let port = self.compile_expr(&args[0])?;
        let data = self.compile_expr(&args[1])?;
        self.emit(OpCode::Deo(data, port));
        let unit = self.alloc_register()?;
        let idx = self.add_constant(Value::UNIT)?;
        self.emit(OpCode::PushConst(unit, idx));
        Ok(unit)
    }

    // device_out(port) → Dei(dst, port). Returns the read value.
    fn emit_device_out(&mut self, args: &[ast::Spanned<ast::Expr>]) -> Result<Register, String> {
        let port = self.compile_expr(&args[0])?;
        let dest = self.alloc_register()?;
        self.emit(OpCode::Dei(dest, port));
        Ok(dest)
    }

    fn emit_effect_op_dispatch(
        &mut self,
        effect_id: u16,
        op_id: u8,
        args: &[ast::Spanned<ast::Expr>],
    ) -> Result<Register, String> {
        let key = ((effect_id as i64) << 8) | (op_id as i64);

        let key_reg = self.alloc_register()?;
        let key_idx = self.add_constant(Value::from_int(key))?;
        self.emit(OpCode::PushConst(key_reg, key_idx));

        let nargs = args.len();
        let args_base = if nargs == 0 {
            self.alloc_register()?
        } else {
            let first = self.alloc_register()?;
            for _ in 1..nargs {
                let _ = self.alloc_register()?;
            }
            first
        };
        for (i, arg) in args.iter().enumerate() {
            let r = self.compile_expr(arg)?;
            let slot = Register((args_base.0 as usize + i) as u8);
            if self.arg_should_move(arg) {
                self.emit(OpCode::Move(slot, r));
            } else {
                self.emit(OpCode::Copy(slot, r));
            }
        }

        let dest = self.alloc_register()?;
        self.emit(OpCode::Raise(dest, key_reg, args_base));
        Ok(dest)
    }

    fn emit_variant_ctor(&mut self, tag: u32, args: &[ast::Spanned<ast::Expr>]) -> Result<Register, String> {
        let dest = self.alloc_register()?;
        let alloc_size = super::scaffold::to_u16(args.len() + 1, "Variant ctor payload size")?;
        self.emit(OpCode::Alloc(dest, alloc_size));
        let tag_reg = self.alloc_register()?;
        let ti = self.add_constant(Value::from_int(tag as i64))?;
        self.emit(OpCode::PushConst(tag_reg, ti));
        self.emit(OpCode::St(tag_reg, dest, 0));
        for (i, arg) in args.iter().enumerate() {
            let offset = super::scaffold::to_u16(i + 1, "Variant ctor offset")?;
            let want_move = self.arg_should_move(arg);
            let v = self.compile_expr(arg)?;
            self.emit_store_field(v, want_move, dest, offset)?;
        }
        Ok(dest)
    }

    fn emit_func_call(
        &mut self,
        func_id: u16,
        env: CallEnv,
        args: &[ast::Spanned<ast::Expr>],
    ) -> Result<Register, String> {
        let mark = self.snapshot_register_high_water();
        let env_to_pass = match env {
            CallEnv::None => None,
            CallEnv::Reg(r) => Some(r),
            CallEnv::EnvLoadOffset(off) => {
                let outer_env = *self.var_to_reg.get("__env")
                    .ok_or_else(|| "internal: env-load call site outside closure body".to_string())?;
                let tmp = self.alloc_register()?;
                let off16 = super::scaffold::to_u16(off, "outer env offset")?;
                self.emit(OpCode::Ld(tmp, outer_env, off16));
                Some(tmp)
            }
        };
        let mut staged: Vec<(Register, bool)> = env_to_pass.into_iter().map(|r| (r, true)).collect();
        for arg in args {
            let r = self.compile_expr(arg)?;
            staged.push((r, self.arg_should_move(arg)));
        }
        self.stage_call_args(&staged)?;
        self.reclaim_temp_regs_above(mark);
        let dest = self.alloc_register()?;
        self.emit(OpCode::Call(dest, func_id));
        Ok(dest)
    }

    fn emit_tail_self_call(
        &mut self,
        args: &[ast::Spanned<ast::Expr>],
    ) -> Result<Register, String> {
        let mut temps: Vec<Register> = Vec::with_capacity(args.len());
        for arg in args {
            let src = self.compile_expr(arg)?;
            let want_move = self.arg_should_move(arg);
            let t = self.alloc_register()?;
            if want_move {
                self.emit(OpCode::Move(t, src));
            } else {
                self.emit(OpCode::Copy(t, src));
            }
            temps.push(t);
        }
        self.emit_drops_to_exit_fn(None)?;
        self.emit_pops_to_exit_fn()?;
        for (i, t) in temps.iter().enumerate() {
            let slot = super::scaffold::to_u8(i, "TCO param slot")?;
            self.emit(OpCode::Move(Register(slot), *t));
        }
        let p = self.code.len();
        let off = -((p as i64) + 1);
        if off < i16::MIN as i64 {
            return Err(format!("TCO jump offset {} out of i16 range", off));
        }
        self.emit(OpCode::Jmp(off as i16));
        let dummy = self.alloc_register()?;
        let idx = self.add_constant(Value::UNIT)?;
        self.emit(OpCode::PushConst(dummy, idx));
        Ok(dummy)
    }

    fn emit_method_call(
        &mut self,
        func_id: u16,
        receiver: &ast::Spanned<ast::Expr>,
        args: &[ast::Spanned<ast::Expr>],
    ) -> Result<Register, String> {
        let mark = self.snapshot_register_high_water();
        let r = self.compile_expr(receiver)?;
        let mut staged = vec![(r, self.arg_should_move(receiver))];
        for arg in args {
            let r = self.compile_expr(arg)?;
            staged.push((r, self.arg_should_move(arg)));
        }
        self.stage_call_args(&staged)?;
        self.reclaim_temp_regs_above(mark);
        let dest = self.alloc_register()?;
        self.emit(OpCode::Call(dest, func_id));
        Ok(dest)
    }

    pub(in crate::compiler) fn arg_should_move(&mut self, arg: &ast::Spanned<ast::Expr>) -> bool {
        match &arg.node {
            ast::Expr::Identifier(name) => {
                let n = name.clone();
                self.id_should_move(&n)
            }
            _ => true,
        }
    }

    pub(in crate::compiler) fn id_should_move(&mut self, name: &str) -> bool {
        let owned_ty = self.var_types.get(name).cloned();
        match &owned_ty {
            Some(ty) if super::is_move_type(ty) => true,
            Some(ty) if super::is_share_type(ty) => self.consume_use(name),
            _ => false,
        }
    }

    pub(in crate::compiler) fn emit_store_field(
        &mut self,
        src: Register,
        want_move: bool,
        dest: Register,
        offset: u16,
    ) -> Result<(), String> {
        let store_src = if want_move {
            src
        } else {
            let tmp = self.alloc_register()?;
            self.emit(OpCode::Copy(tmp, src));
            tmp
        };
        self.emit(OpCode::St(store_src, dest, offset));
        Ok(())
    }

    fn stage_call_args(&mut self, staged: &[(Register, bool)]) -> Result<(), String> {
        for (i, (src, want_move)) in staged.iter().enumerate() {
            let slot = super::scaffold::to_u8(i, "Argument slot")?;
            let pos = self.code.len();
            if *want_move {
                self.emit(OpCode::Move(Register(0), *src));
            } else {
                self.emit(OpCode::Copy(Register(0), *src));
            }
            self.pending_arg_patches.push((pos, slot));
        }
        Ok(())
    }
}
