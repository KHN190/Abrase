// Codegen for effect-related forms: `throw`, `?`, `resume`, and `handle`.
use crate::ast;
use crate::bytecode::{OpCode, Register};
use crate::compiler::Compiler;
use crate::compiler::effects;
use crate::bytecode::Value;

impl Compiler {
    pub(in crate::compiler) fn compile_throw(
        &mut self,
        inner: &ast::Spanned<ast::Expr>,
    ) -> Result<Register, String> {
        if !self.current_fn_fallible {
            return Err("`throw` outside <exn> function".to_string());
        }
        let err_val = self.compile_expr(inner)?;
        let wrapped = self.wrap_err(err_val)?;
        // abnormal-exit order: forget_typed → drops → pops → Ret.
        self.emit_region_forget(wrapped)?;
        self.emit_drops_to_exit_fn(Some(wrapped))?;
        self.emit_handler_pops_to_exit_fn()?;
        self.emit_pops_to_exit_fn()?;
        self.emit(OpCode::Ret(wrapped));
        Ok(wrapped)
    }

    pub(in crate::compiler) fn compile_question(
        &mut self,
        inner: &ast::Spanned<ast::Expr>,
    ) -> Result<Register, String> {
        if !self.current_fn_fallible {
            return Err("`?` outside <exn> function".to_string());
        }
        let res = self.compile_expr(inner)?;
        let tag = self.alloc_register()?;
        self.emit(OpCode::Ld(tag, res, 0));
        let err_tag = self.alloc_register()?;
        let idx = self.add_constant(Value::from_int(effects::ERR_TAG as i64))?;
        self.emit(OpCode::PushConst(err_tag, idx));
        let is_err = self.alloc_register()?;
        self.emit(OpCode::Eq(is_err, tag, err_tag));
        let jz_idx = self.code.len();
        self.emit(OpCode::Jz(is_err, 0));
        // Err path: same shape as throw — forget the wrapped value, drop
        // every binder above fn baseline, pop every region, Ret.
        self.emit_region_forget(res)?;
        self.emit_drops_to_exit_fn(Some(res))?;
        self.emit_handler_pops_to_exit_fn()?;
        self.emit_pops_to_exit_fn()?;
        self.emit(OpCode::Ret(res));
        let after = self.code.len();
        self.patch_jz_at(jz_idx, after)?;
        let val = self.alloc_register()?;
        self.emit(OpCode::Ld(val, res, 1));
        Ok(val)
    }

    pub(in crate::compiler) fn compile_resume(
        &mut self,
        arg: Option<&ast::Spanned<ast::Expr>>,
    ) -> Result<Register, String> {
        let inferred_ty = arg.and_then(|e| self.infer_expr_type(e));
        let val_reg = if let Some(e) = arg {
            self.compile_expr(e)?
        } else {
            let r = self.alloc_register()?;
            let idx = self.add_constant(Value::UNIT)?;
            self.emit(OpCode::PushConst(r, idx));
            r
        };

        let resume_count = self.arm_resume_counts
            .get(&self.current_fn_name)
            .copied()
            .unwrap_or(0);
        let is_multishot = resume_count > 1;

        if is_multishot {
            let dest = self.alloc_register()?;
            self.emit(OpCode::Resume(dest, val_reg));
            Ok(dest)
        } else {
            if let Some(ty) = &inferred_ty {
                self.emit_region_forget_typed(val_reg, ty)?;
            } else {
                self.emit_region_forget(val_reg)?;
            }
            self.emit_drops_to_exit_fn(Some(val_reg))?;
            self.emit_handler_pops_to_exit_fn()?;
            self.emit_pops_to_exit_fn()?;
            self.emit(OpCode::Ret(val_reg));
            Ok(val_reg)
        }
    }

    pub(in crate::compiler) fn compile_handle(
        &mut self,
        body: &ast::Spanned<ast::Expr>,
        handle_span: ast::Span,
        arms: &[ast::HandleArm],
    ) -> Result<Register, String> {

        let arm_names = self.collect_handle_arm_names(handle_span, arms);
        let envs = self.pack_arm_envs(&arm_names)?;
        self.arm_env_stack.push(envs);

        let installed_frame = self.emit_handle_install(handle_span, arms)?;

        let body_reg = self.compile_expr(body)?;
        let arm_envs = self.arm_env_stack.pop()
            .ok_or_else(|| "internal: arm_env_stack underflow at compile_handle".to_string())?;

        if installed_frame {
            self.emit_handle_pop()?;
        }

        let ret_arm_name = self.return_arm_by_handle.get(&handle_span).cloned()
            .ok_or_else(|| format!(
                "internal: no return arm registered for handle at {:?}", handle_span
            ))?;
        let func_id = *self.func_map.get(&ret_arm_name)
            .ok_or_else(|| format!("internal: return arm '{}' not in fn table", ret_arm_name))?;
        let env_reg = arm_envs.get(&ret_arm_name).copied()
            .ok_or_else(|| format!("internal: no env packed for return arm '{}'", ret_arm_name))?;

        //   PushConst(r7, 0)           # Load 0 into return_env_reg
        //   Copy(r0, r5)               # arg 0: __env → r0
        //   Copy(r0, r7)               # arg 1: __return_env → r0
        //   Copy(r0, r6)               # arg 2: body_result → r0
        //   Call(r8, 6)                # Call function_id=6, result → r8

        let pos = self.code.len();
        self.emit(OpCode::Copy(Register(0), env_reg));
        self.pending_arg_patches.push((pos, 0));

        let return_env_reg = self.alloc_register()?;
        let zero_idx = self.add_constant(Value::from_int(0))?;
        self.emit(OpCode::PushConst(return_env_reg, zero_idx));
        let pos = self.code.len();
        self.emit(OpCode::Copy(Register(0), return_env_reg));
        self.pending_arg_patches.push((pos, 1));

        let pos = self.code.len();
        self.emit(OpCode::Copy(Register(0), body_reg));
        self.pending_arg_patches.push((pos, 2));

        let dest = self.alloc_register()?;
        let fid = super::scaffold::to_u16(func_id, "Handler arm fn_id")?;
        self.emit(OpCode::Call(dest, fid));
        Ok(dest)
    }

    fn emit_handle_install(&mut self, handle_span: ast::Span, arms: &[ast::HandleArm]) -> Result<bool, String> {
        let eff_name = arms.iter().find_map(|arm| match &arm.kind {
            ast::HandleArmKind::Effect(path) if path.len() >= 2 => {
                Some(path[..path.len()-1].join("."))
            }
            _ => None,
        });
        if let Some(sink) = &mut self.debug_sink {
            sink(&format!("[emit_handle_install] eff_name={:?}", eff_name));
        }
        let eff = match eff_name { Some(n) => n, None => {
            if let Some(sink) = &mut self.debug_sink {
                sink("[emit_handle_install] no effect found, returning false");
            }
            return Ok(false)
        } };
        let effect_id = match self.effect_ids.get(&eff).copied() {
            Some(id) => id,
            None => {
                if let Some(sink) = &mut self.debug_sink {
                    sink("[emit_handle_install] effect_id not found");
                }
                return Ok(false)
            },
        };
        let op_count = self.effect_op_counts.get(&eff).copied().unwrap_or(0) as usize;
        if let Some(sink) = &mut self.debug_sink {
            sink(&format!("[emit_handle_install] effect_id={}, op_count={}", effect_id, op_count));
        }
        if op_count == 0 {
            if let Some(sink) = &mut self.debug_sink {
                sink("[emit_handle_install] op_count is 0, returning false");
            }
            return Ok(false);
        }

        let local_arms = self.effect_arms_by_handle.get(&handle_span).cloned().unwrap_or_default();

        let table_reg = self.alloc_register()?;
        let alloc_size = super::scaffold::to_u16((op_count * 2).max(1), "Dispatch table size")?;
        self.emit(OpCode::Alloc(table_reg, alloc_size));

        let current_envs = self.arm_env_stack.last().cloned().unwrap_or_default();
        for arm in arms {
            if let ast::HandleArmKind::Effect(path) = &arm.kind {
                if path.len() < 2 { continue; }
                let op_name = path.last().cloned().unwrap();
                let key = (eff.clone(), op_name);
                let arm_name = match local_arms.get(&key).cloned() {
                    Some(n) => n,
                    None => match self.effect_op_to_arm.get(&key).cloned() {
                        Some(n) => n,
                        None => continue,
                    },
                };
                let op_id = self.op_ids.get(&key).copied().unwrap_or(0);
                let fn_id = *self.func_map.get(&arm_name).unwrap_or(&0);
                let fn_id_reg = self.alloc_register()?;
                let fn_id_idx = self.add_constant(crate::bytecode::Value::from_int(fn_id as i64))?;
                self.emit(OpCode::PushConst(fn_id_reg, fn_id_idx));
                let off = (op_id as u16).saturating_mul(2);
                self.emit(OpCode::St(fn_id_reg, table_reg, off));
                if let Some(env_reg) = current_envs.get(&arm_name).copied() {
                    let tmp = self.alloc_register()?;
                    self.emit(OpCode::Copy(tmp, env_reg));
                    self.emit(OpCode::St(tmp, table_reg, off + 1));
                }
            }
        }
        let eid_u16 = effect_id;
        let dest_reg = self.alloc_register()?;
        self.emit(OpCode::Handle(dest_reg, table_reg, eid_u16));
        self.handler_table_stack.push(table_reg);
        Ok(true)
    }

    fn emit_handle_pop(&mut self) -> Result<(), String> {
        let table_reg = self.handler_table_stack.pop()
            .ok_or_else(|| "internal: emit_handle_pop without matching install".to_string())?;
        self.emit_handler_pop_one(table_reg)
    }

    // Gather the arm fn names belonging to a particular `handle` expression
    fn collect_handle_arm_names(
        &self,
        handle_span: ast::Span,
        arms: &[ast::HandleArm],
    ) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(name) = self.return_arm_by_handle.get(&handle_span) {
            out.push(name.clone());
        }
        let local = self.effect_arms_by_handle.get(&handle_span);
        for arm in arms {
            if let ast::HandleArmKind::Effect(path) = &arm.kind {
                if path.len() >= 2 {
                    if let Some(op) = path.last().cloned() {
                        let eff = path[..path.len()-1].join(".");
                        let key = (eff, op);
                        let name = local.and_then(|m| m.get(&key).cloned())
                            .or_else(|| self.effect_op_to_arm.get(&key).cloned());
                        if let Some(name) = name {
                            out.push(name);
                        }
                    }
                }
            }
        }
        out
    }

    // Allocate one env heap object per arm fn and store its captures from
    // the current scope. Returns `arm fn name -> env register`.
    fn pack_arm_envs(
        &mut self,
        arm_names: &[String],
    ) -> Result<std::collections::HashMap<String, Register>, String> {
        let mut envs = std::collections::HashMap::new();
        for name in arm_names {
            let captures = self.arm_captures.get(name).cloned().unwrap_or_default();
            let env_reg = self.alloc_register()?;
            let n = captures.len();
            let alloc_size = super::scaffold::to_u16(n.max(1), &format!("Handler arm '{}' env size", name))?;
            self.emit(OpCode::Alloc(env_reg, alloc_size));
            for (i, cap) in captures.iter().enumerate() {
                let offset = super::scaffold::to_u16(i, "Handler env offset")?;
                let src = *self.var_to_reg.get(&cap.name)
                    .ok_or_else(|| format!(
                        "internal: handler arm '{}' captures '{}', not in scope at handle site",
                        name, cap.name
                    ))?;
                self.emit(OpCode::St(src, env_reg, offset));
            }
            envs.insert(name.clone(), env_reg);
        }
        Ok(envs)
    }
}
