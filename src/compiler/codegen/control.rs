// Control-flow forms: if/else, while, and early returns.

use crate::ast;
use crate::bytecode::{OpCode, Register};
use crate::compiler::Compiler;
use crate::myriad::Value;

impl Compiler {
    pub(in crate::compiler) fn compile_if(
        &mut self,
        condition: &ast::Spanned<ast::Expr>,
        consequence: &ast::Spanned<ast::Expr>,
        alternative: Option<&ast::Spanned<ast::Expr>>,
    ) -> Result<Register, String> {
        let cond_reg = self.compile_expr(condition)?;
        let jz_idx = self.code.len();
        self.emit(OpCode::Jz(cond_reg, 0));

        let result_reg = self.alloc_register()?;

        let cons_reg = self.compile_expr(consequence)?;
        // Peephole only on leaf expressions; phi-joining forms can't be redirected.
        if !is_leaf_for_peephole(&consequence.node)
            || !self.try_redirect_last_dest(cons_reg, result_reg)
        {
            self.emit(OpCode::Copy(result_reg, cons_reg));
        }

        let jmp_idx = self.code.len();
        self.emit(OpCode::Jmp(0));

        let else_addr = self.code.len();
        self.patch_jz_at(jz_idx, else_addr)?;

        let (alt_reg, alt_leaf) = if let Some(alt) = alternative {
            (self.compile_expr(alt)?, is_leaf_for_peephole(&alt.node))
        } else {
            let r = self.alloc_register()?;
            let idx = self.add_constant(Value::UNIT)?;
            self.emit(OpCode::PushConst(r, idx));
            (r, true)
        };
        if !alt_leaf || !self.try_redirect_last_dest(alt_reg, result_reg) {
            self.emit(OpCode::Copy(result_reg, alt_reg));
        }

        let end_addr = self.code.len();
        self.patch_jmp_at(jmp_idx, end_addr)?;

        Ok(result_reg)
    }

    pub(in crate::compiler) fn compile_while(
        &mut self,
        condition: &ast::Spanned<ast::Expr>,
        body: &ast::Block,
    ) -> Result<Register, String> {
        let loop_addr = self.code.len();
        let cond_reg = self.compile_expr(condition)?;
        let jz_idx = self.code.len();
        self.emit(OpCode::Jz(cond_reg, 0));

        self.compile_block(body)?;
        let back_idx = self.code.len();
        let back_off = self.rel_offset(loop_addr, back_idx)?;
        self.emit(OpCode::Jmp(back_off));

        let exit_addr = self.code.len();
        self.patch_jz_at(jz_idx, exit_addr)?;

        let r = self.alloc_register()?;
        let idx = self.add_constant(Value::UNIT)?;
        self.emit(OpCode::PushConst(r, idx));
        Ok(r)
    }

    pub(in crate::compiler) fn compile_return(
        &mut self,
        opt_expr: Option<&ast::Spanned<ast::Expr>>,
    ) -> Result<Register, String> {
        let r = if let Some(expr) = opt_expr {
            self.compile_expr(expr)?
        } else {
            let reg = self.alloc_register()?;
            let idx = self.add_constant(Value::UNIT)?;
            self.emit(OpCode::PushConst(reg, idx));
            reg
        };
        let ret_reg = if self.current_fn_fallible { self.wrap_ok(r)? } else { r };
        self.emit(OpCode::Ret(ret_reg));
        Ok(r)
    }
}

// Phi-joining forms (Match, If, Block, Handle, While) write the result reg
// from multiple sites and can't be redirected by a single last-emit rewrite.
fn is_leaf_for_peephole(expr: &ast::Expr) -> bool {
    matches!(
        expr,
        ast::Expr::Literal(_)
        | ast::Expr::Identifier(_)
        | ast::Expr::Binary { .. }
        | ast::Expr::Unary { .. }
        | ast::Expr::Call { .. }
        | ast::Expr::FieldAccess { .. }
        | ast::Expr::Index { .. }
        | ast::Expr::Record { .. }
        | ast::Expr::Variant { .. }
        | ast::Expr::Array(_)
        | ast::Expr::Closure { .. }
    )
}
