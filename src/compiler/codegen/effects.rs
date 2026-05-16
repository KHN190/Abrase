// Codegen for effect-related forms: `throw`, `?`, `resume`, and `handle`.
use crate::ast;
use crate::bytecode::{OpCode, Register};
use crate::compiler::Compiler;
use crate::compiler::effects;
use crate::vm::Value;

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
        let idx = self.add_constant(Value::Int(effects::ERR_TAG as i64))?;
        self.emit(OpCode::PushConst(err_tag, idx));
        let is_err = self.alloc_register()?;
        self.emit(OpCode::Eq(is_err, tag, err_tag));
        let jz_idx = self.code.len();
        self.emit(OpCode::Jz(is_err, 0));
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
        // Tail-position only for now: `resume(v)` becomes `Ret(v)`.
        let reg = if let Some(e) = arg {
            self.compile_expr(e)?
        } else {
            let r = self.alloc_register()?;
            let idx = self.add_constant(Value::Unit)?;
            self.emit(OpCode::PushConst(r, idx));
            r
        };
        self.emit(OpCode::Ret(reg));
        Ok(reg)
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

        let body_reg = self.compile_expr(body)?;
        let arm_envs = self.arm_env_stack.pop()
            .ok_or_else(|| "internal: arm_env_stack underflow at compile_handle".to_string())?;

        let ret_arm_name = self.return_arm_by_handle.get(&handle_span).cloned()
            .ok_or_else(|| format!(
                "internal: no return arm registered for handle at {:?}", handle_span
            ))?;
        let func_id = *self.func_map.get(&ret_arm_name)
            .ok_or_else(|| format!("internal: return arm '{}' not in fn table", ret_arm_name))?;
        let env_reg = arm_envs.get(&ret_arm_name).copied()
            .ok_or_else(|| format!("internal: no env packed for return arm '{}'", ret_arm_name))?;

        // Call convention: (env, body_result).
        let pos = self.code.len();
        self.emit(OpCode::Copy(Register(0), env_reg));
        self.pending_arg_patches.push((pos, 0));
        let pos = self.code.len();
        self.emit(OpCode::Copy(Register(0), body_reg));
        self.pending_arg_patches.push((pos, 1));

        let dest = self.alloc_register()?;
        self.emit(OpCode::Call(dest, func_id as u16));
        Ok(dest)
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
        for arm in arms {
            if let ast::HandleArmKind::Effect(path) = &arm.kind {
                if path.len() >= 2 {
                    if let Some(op) = path.last().cloned() {
                        let eff = path[..path.len()-1].join(".");
                        if let Some(name) = self.effect_op_to_arm.get(&(eff, op)) {
                            out.push(name.clone());
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
            // alloc at least one slot
            self.emit(OpCode::Alloc(env_reg, n.max(1) as u16));
            for (i, cap) in captures.iter().enumerate() {
                let src = *self.var_to_reg.get(&cap.name)
                    .ok_or_else(|| format!(
                        "internal: handler arm '{}' captures '{}', not in scope at handle site",
                        name, cap.name
                    ))?;
                self.emit(OpCode::St(src, env_reg, i as u16));
            }
            envs.insert(name.clone(), env_reg);
        }
        Ok(envs)
    }
}
