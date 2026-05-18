use crate::ast;
use crate::bytecode::{OpCode, Register};
use crate::compiler::Compiler;
use crate::bytecode::Value;

pub(in crate::compiler) enum CallEnv {
    None,
    Reg(Register)
}

pub(in crate::compiler) enum CallTarget<'a> {
    Function { func_id: u16, env: CallEnv },
    Method   { func_id: u16, receiver: &'a ast::Spanned<ast::Expr> },
    EnvLoad  { env_reg: Register, idx: u16 },
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
        if let Some(t) = self.resolve_closure_call(callee)? { return Ok(t); }
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
        let func_id = if self.fn_overloads.contains_key(name) {
            self.resolve_overload_fn_id(name, args)?
        } else {
            self.func_map.get(name).copied()
                .ok_or_else(|| format!("Undefined function: {}", name))?
        };
        let fid = super::scaffold::to_u16(func_id, &format!("Function id for '{}'", name))?;
        Ok(CallTarget::Function { func_id: fid, env: CallEnv::None })
    }

    fn resolve_overload_fn_id(
        &self,
        name: &str,
        args: &[ast::Spanned<ast::Expr>],
    ) -> Result<usize, String> {
        let arg_tys: Vec<Option<ast::Type>> =
            args.iter().map(|a| self.infer_expr_type(a)).collect();
        let mut candidates: Vec<usize> = Vec::new();
        if let Some(&primary) = self.func_map.get(name) { candidates.push(primary); }
        if let Some(extras) = self.fn_overloads.get(name) { candidates.extend(extras); }
        for fid in &candidates {
            let Some((params, _)) = self.fn_signatures.get(fid) else { continue };
            if params.len() != arg_tys.len() { continue; }
            let ok = params.iter().zip(&arg_tys).all(|(p, a)| match a {
                Some(t) => Self::ty_matches_ast_type(p, t),
                None => false,
            });
            if ok { return Ok(*fid); }
        }
        candidates.first().copied()
            .ok_or_else(|| format!("Undefined function: {}", name))
    }

    fn ty_matches_ast_type(ty: &crate::ty::Type, ast_ty: &ast::Type) -> bool {
        use crate::ty::Type as T;
        match (ty, ast_ty) {
            (T::Int,    ast::Type::Named(n)) if n == "Int"    => true,
            (T::Float,  ast::Type::Named(n)) if n == "Float"  => true,
            (T::Bool,   ast::Type::Named(n)) if n == "Bool"   => true,
            (T::Char,   ast::Type::Named(n)) if n == "Char"   => true,
            (T::String, ast::Type::Named(n)) if n == "String" => true,
            (T::Unit,   ast::Type::Tuple(v)) if v.is_empty()  => true,
            _ => false,
        }
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
            return Ok(Some(CallTarget::HostFn { fn_id: host.fn_id }));
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

    fn emit_shared_ctor(&mut self, args: &[ast::Spanned<ast::Expr>]) -> Result<Register, String> {
        let src = self.compile_expr(&args[0])?;
        let dest = self.alloc_register()?;
        self.emit(OpCode::Alloc(dest, 1));
        self.emit(OpCode::St(src, dest, 0));
        Ok(dest)
    }

    fn emit_host_fn_call(&mut self, fn_id: u16, args: &[ast::Spanned<ast::Expr>]) -> Result<Register, String> {
        let arg_port = self.alloc_register()?;
        let arg_port_idx = self.add_constant(Value::from_int(0xF018))?;
        self.emit(OpCode::PushConst(arg_port, arg_port_idx));
        for a in args {
            let v = self.compile_expr(a)?;
            self.emit(OpCode::Deo(v, arg_port));
        }
        let trigger_reg = self.alloc_register()?;
        let trigger_val_idx = self.add_constant(Value::from_int(fn_id as i64))?;
        self.emit(OpCode::PushConst(trigger_reg, trigger_val_idx));
        let trigger_port = self.alloc_register()?;
        let trigger_port_idx = self.add_constant(Value::from_int(0xF01F))?;
        self.emit(OpCode::PushConst(trigger_port, trigger_port_idx));
        self.emit(OpCode::Deo(trigger_reg, trigger_port));
        let result_port = self.alloc_register()?;
        let result_port_idx = self.add_constant(Value::from_int(0xF01E))?;
        self.emit(OpCode::PushConst(result_port, result_port_idx));
        let result = self.alloc_register()?;
        self.emit(OpCode::Dei(result, result_port));
        self.device_mask[0xF0 / 8] |= 1 << (0xF0 % 8);
        Ok(result)
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

        //   PushConst(key_reg, key_idx)
        //   PushConst(lookup_port_reg, lookup_port_idx)
        //   Deo(key_reg, lookup_port_reg)      # Request handler fn_id
        //   Dei(fn_id_reg, lookup_port_reg)    # Receive fn_id
        //   PushConst(env_port_reg, env_port_idx)
        //   Dei(env_reg, env_port_reg)         # Receive arm env
        //   PushConst(return_env_reg, 0)       # Dummy return_env
        //   <stage args with env, return_env, user_args>
        //   CallReg(dest, fn_id_reg)           # Call dispatched handler

        let key = ((effect_id as i64) << 8) | (op_id as i64);
        let lookup_port = ((crate::bytecode::DISPATCH_ID as i64) << 8)
            | (crate::bytecode::DISPATCH_PORT_LOOKUP as i64);
        let env_port = ((crate::bytecode::DISPATCH_ID as i64) << 8)
            | (crate::bytecode::DISPATCH_PORT_ENV as i64);

        let key_reg = self.alloc_register()?;
        let key_idx = self.add_constant(Value::from_int(key))?;
        self.emit(OpCode::PushConst(key_reg, key_idx));

        let lookup_port_reg = self.alloc_register()?;
        let lookup_port_idx = self.add_constant(Value::from_int(lookup_port))?;
        self.emit(OpCode::PushConst(lookup_port_reg, lookup_port_idx));
        self.emit(OpCode::Deo(key_reg, lookup_port_reg));

        let fn_id_reg = self.alloc_register()?;
        self.emit(OpCode::Dei(fn_id_reg, lookup_port_reg));

        let env_port_reg = self.alloc_register()?;
        let env_port_idx = self.add_constant(Value::from_int(env_port))?;
        self.emit(OpCode::PushConst(env_port_reg, env_port_idx));
        let env_reg = self.alloc_register()?;
        self.emit(OpCode::Dei(env_reg, env_port_reg));

        let return_env_reg = self.alloc_register()?;
        let zero_idx = self.add_constant(Value::from_int(0))?;
        self.emit(OpCode::PushConst(return_env_reg, zero_idx));

        let mut staged: Vec<(Register, bool)> = vec![(env_reg, false), (return_env_reg, false)];
        for arg in args {
            let r = self.compile_expr(arg)?;
            staged.push((r, self.arg_should_move(arg)));
        }
        self.stage_call_args(&staged)?;
        let dest = self.alloc_register()?;
        self.emit(OpCode::CallReg(dest, fn_id_reg));
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
        let env_to_pass = match env {
            CallEnv::None => None,
            CallEnv::Reg(r) => Some(r)
        };
        let mut staged: Vec<(Register, bool)> = env_to_pass.into_iter().map(|r| (r, true)).collect();
        for arg in args {
            let r = self.compile_expr(arg)?;
            staged.push((r, self.arg_should_move(arg)));
        }
        self.stage_call_args(&staged)?;
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
        let r = self.compile_expr(receiver)?;
        let mut staged = vec![(r, self.arg_should_move(receiver))];
        for arg in args {
            let r = self.compile_expr(arg)?;
            staged.push((r, self.arg_should_move(arg)));
        }
        self.stage_call_args(&staged)?;
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
