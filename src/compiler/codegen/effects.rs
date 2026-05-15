// Codegen for effect-related forms: `throw`, `?`, `resume`, and `handle`.
//
// The current `resume`/`handle` lowering is the simplified MVP path:
//
//   * `resume(v)` lowers to a plain `Ret(v)`, so it only works when it
//     appears in tail position of the arm body.
//   * `handle BODY { arms }` compiles BODY directly, then calls the
//     synthesised return-arm fn with the result. No `OpCode::Handle` frame
//     is installed, so multi-shot resumes and nested-handler dispatch are
//     not yet supported.
//
// Removing these limitations is the next compiler-side task (heap-cell
// continuations + per-handle dispatch tables).

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
    ) -> Result<Register, String> {
        // The pre-pass already lifted arm bodies to top-level fns. Compile
        // the protected body, then call the return-arm with its result.
        let body_reg = self.compile_expr(body)?;
        let ret_arm_name = self.return_arm_by_handle.get(&handle_span).cloned()
            .ok_or_else(|| format!(
                "internal: no return arm registered for handle at {:?}", handle_span
            ))?;
        let func_id = *self.func_map.get(&ret_arm_name)
            .ok_or_else(|| format!("internal: return arm '{}' not in fn table", ret_arm_name))?;

        let pos = self.code.len();
        self.emit(OpCode::Copy(Register(0), body_reg));
        self.pending_arg_patches.push((pos, 0));

        let dest = self.alloc_register()?;
        self.emit(OpCode::Call(dest, func_id as u16));
        Ok(dest)
    }
}
