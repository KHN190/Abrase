// `Expr::Call` dispatcher. The dispatcher runs five intent-named heuristics
// in sequence; whichever recognises the call shape emits the right opcode
// and returns. The fallback path is a plain by-id call.

use crate::ast;
use crate::bytecode::{OpCode, Register};
use crate::compiler::Compiler;
use crate::vm::Value;

impl Compiler {
    pub(in crate::compiler) fn compile_call(
        &mut self,
        callee: &ast::Spanned<ast::Expr>,
        args: &[ast::Spanned<ast::Expr>],
    ) -> Result<Register, String> {
        if let Some(r) = self.try_compile_env_load(callee, args)? { return Ok(r); }
        if let Some(r) = self.try_compile_closure_call(callee, args)? { return Ok(r); }
        if let Some(r) = self.try_compile_effect_op_call(callee, args)? { return Ok(r); }
        if let Some(r) = self.try_compile_method_call(callee, args)? { return Ok(r); }
        if let Some(r) = self.try_compile_host_or_ctor(callee, args)? { return Ok(r); }
        self.compile_plain_call(callee, args)
    }

    /// Synthetic env load inside a lifted closure body, emitted by the
    /// closure pre-pass: `__env_load(__env, idx)` -> `ld dst, env, idx`.
    fn try_compile_env_load(
        &mut self,
        callee: &ast::Spanned<ast::Expr>,
        args: &[ast::Spanned<ast::Expr>],
    ) -> Result<Option<Register>, String> {
        let ast::Expr::Identifier(name) = &callee.node else { return Ok(None) };
        if name != "__env_load" || args.len() != 2 { return Ok(None); }
        let (env_name, idx) = match (&args[0].node, &args[1].node) {
            (ast::Expr::Identifier(env_name), ast::Expr::Literal(ast::Literal::Int(idx))) => (env_name, idx),
            _ => return Ok(None),
        };
        let env_reg = *self.var_to_reg.get(env_name)
            .ok_or_else(|| format!(
                "internal: env binding '{}' not in scope", env_name
            ))?;
        let dest = self.alloc_register()?;
        self.emit(OpCode::Ld(dest, env_reg, *idx as u16));
        Ok(Some(dest))
    }

    /// Direct call to a lifted closure fn when the callee is a bare local
    /// binding produced by a closure literal in the same fn body.
    fn try_compile_closure_call(
        &mut self,
        callee: &ast::Spanned<ast::Expr>,
        args: &[ast::Spanned<ast::Expr>],
    ) -> Result<Option<Register>, String> {
        let ast::Expr::Identifier(name) = &callee.node else { return Ok(None) };
        let Some(info) = self.closure_by_var.get(name).cloned() else { return Ok(None) };

        let func_id = *self.func_map.get(&info.lifted_fn)
            .ok_or_else(|| format!(
                "internal: lifted closure fn '{}' not in fn table",
                info.lifted_fn
            ))?;
        let env_reg = *self.var_to_reg.get(name)
            .ok_or_else(|| format!(
                "internal: closure binding '{}' has no register", name
            ))?;
        let mut arg_srcs = Vec::with_capacity(args.len());
        for arg in args {
            arg_srcs.push(self.compile_expr(arg)?);
        }
        let pos = self.code.len();
        self.emit(OpCode::Copy(Register(0), env_reg));
        self.pending_arg_patches.push((pos, 0));
        for (i, src) in arg_srcs.iter().enumerate() {
            let pos = self.code.len();
            self.emit(OpCode::Copy(Register(0), *src));
            self.pending_arg_patches.push((pos, (i + 1) as u8));
        }
        let dest = self.alloc_register()?;
        self.emit(OpCode::Call(dest, func_id as u16));
        Ok(Some(dest))
    }

    /// Effect-op call: reroute `Foo.op(args)` to the lifted arm fn.
    fn try_compile_effect_op_call(
        &mut self,
        callee: &ast::Spanned<ast::Expr>,
        args: &[ast::Spanned<ast::Expr>],
    ) -> Result<Option<Register>, String> {
        let ast::Expr::FieldAccess { base, field } = &callee.node else { return Ok(None) };
        let ast::Expr::Identifier(eff_name) = &base.node else { return Ok(None) };
        let key = (eff_name.clone(), field.clone());
        let Some(arm_name) = self.effect_op_to_arm.get(&key).cloned() else { return Ok(None) };

        let func_id = *self.func_map.get(&arm_name)
            .ok_or_else(|| format!(
                "internal: arm '{}' missing from fn table", arm_name
            ))?;
        let mut arg_srcs = Vec::with_capacity(args.len());
        for arg in args {
            arg_srcs.push(self.compile_expr(arg)?);
        }
        for (i, src) in arg_srcs.iter().enumerate() {
            if i > u8::MAX as usize {
                return Err("Too many arguments (>255)".to_string());
            }
            let pos = self.code.len();
            self.emit(OpCode::Copy(Register(0), *src));
            self.pending_arg_patches.push((pos, i as u8));
        }
        let dest = self.alloc_register()?;
        self.emit(OpCode::Call(dest, func_id as u16));
        Ok(Some(dest))
    }

    /// Method-call dispatch: `base.method(args)` lowers to a direct call to
    /// the synthesised `Trait__Type__method` fn, with `base` as the first arg.
    fn try_compile_method_call(
        &mut self,
        callee: &ast::Spanned<ast::Expr>,
        args: &[ast::Spanned<ast::Expr>],
    ) -> Result<Option<Register>, String> {
        let ast::Expr::FieldAccess { base, field } = &callee.node else { return Ok(None) };
        let Some(rname) = self.receiver_type_name(base) else { return Ok(None) };
        let key = (rname, field.clone());
        let Some(mangled) = self.method_dispatch.get(&key).cloned() else { return Ok(None) };

        let func_id = *self.func_map.get(&mangled)
            .ok_or_else(|| format!(
                "internal: method '{}' missing from fn table", mangled
            ))?;
        if func_id > u16::MAX as usize {
            return Err(format!("Function id {} exceeds u16 range", func_id));
        }
        let base_src = self.compile_expr(base)?;
        let mut arg_srcs = Vec::with_capacity(args.len() + 1);
        arg_srcs.push(base_src);
        for arg in args {
            arg_srcs.push(self.compile_expr(arg)?);
        }
        for (i, src) in arg_srcs.iter().enumerate() {
            if i > u8::MAX as usize {
                return Err("Too many arguments (>255)".to_string());
            }
            let pos = self.code.len();
            self.emit(OpCode::Copy(Register(0), *src));
            self.pending_arg_patches.push((pos, i as u8));
        }
        let dest = self.alloc_register()?;
        self.emit(OpCode::Call(dest, func_id as u16));
        Ok(Some(dest))
    }

    /// Host built-in `Shared(x)` or a variant constructor call `Foo(a,b)`.
    fn try_compile_host_or_ctor(
        &mut self,
        callee: &ast::Spanned<ast::Expr>,
        args: &[ast::Spanned<ast::Expr>],
    ) -> Result<Option<Register>, String> {
        let ast::Expr::Identifier(name) = &callee.node else { return Ok(None) };

        if name == "Shared" && args.len() == 1 {
            let src = self.compile_expr(&args[0])?;
            let dest = self.alloc_register()?;
            self.emit(OpCode::Alloc(dest, 1));
            self.emit(OpCode::St(src, dest, 0));
            return Ok(Some(dest));
        }

        let Some(info) = self.layouts.variants.get(name).cloned() else { return Ok(None) };
        let dest = self.alloc_register()?;
        let payload = args.len();
        self.emit(OpCode::Alloc(dest, (payload + 1) as u16));
        let tag_reg = self.alloc_register()?;
        let ti = self.add_constant(Value::Int(info.tag as i64))?;
        self.emit(OpCode::PushConst(tag_reg, ti));
        self.emit(OpCode::St(tag_reg, dest, 0));
        for (i, arg) in args.iter().enumerate() {
            let v = self.compile_expr(arg)?;
            self.emit(OpCode::St(v, dest, (i + 1) as u16));
        }
        Ok(Some(dest))
    }

    /// Fallback: a plain by-id call to a top-level fn.
    fn compile_plain_call(
        &mut self,
        callee: &ast::Spanned<ast::Expr>,
        args: &[ast::Spanned<ast::Expr>],
    ) -> Result<Register, String> {
        let ast::Expr::Identifier(name) = &callee.node else {
            return Err("Call target must be a function identifier".to_string());
        };
        let func_id = self.func_map.get(name).copied()
            .ok_or_else(|| format!("Undefined function: {}", name))?;
        if func_id > u16::MAX as usize {
            return Err(format!("Function id {} exceeds u16 range", func_id));
        }

        let mut arg_srcs = Vec::with_capacity(args.len());
        for arg in args {
            arg_srcs.push(self.compile_expr(arg)?);
        }
        for (i, src) in arg_srcs.iter().enumerate() {
            if i > u8::MAX as usize {
                return Err("Too many arguments (>255)".to_string());
            }
            let pos = self.code.len();
            self.emit(OpCode::Copy(Register(0), *src));
            self.pending_arg_patches.push((pos, i as u8));
        }
        let dest = self.alloc_register()?;
        self.emit(OpCode::Call(dest, func_id as u16));
        Ok(dest)
    }
}
