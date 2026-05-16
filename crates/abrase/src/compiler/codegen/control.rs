// Control-flow forms: if/else, while, and early returns.

use crate::ast;
use crate::bytecode::{OpCode, Register};
use crate::compiler::{Compiler, LoopCtx};
use crate::bytecode::Value;

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
        // Per-iteration region: cond check sits before the push so each iter
        // gets a fresh region. break/continue emit their own pop before Jmp
        // (see compile_break / compile_continue) — that keeps push/pop balanced.
        let result_reg = self.alloc_register()?;
        let unit_idx = self.add_constant(Value::UNIT)?;
        self.emit(OpCode::PushConst(result_reg, unit_idx));

        let loop_top = self.code.len();
        let cond_reg = self.compile_expr(condition)?;
        let jz_idx = self.code.len();
        self.emit(OpCode::Jz(cond_reg, 0));

        self.emit_region_push()?;

        self.loop_stack.push(LoopCtx {
            result_reg,
            continue_target: loop_top,
            break_patches: Vec::new(),
            continue_patches: Vec::new(),
        });

        self.compile_block(body)?;

        // Normal iter end: pop the region, jump back to cond check.
        self.emit_region_pop()?;
        let back_idx = self.code.len();
        let back_off = self.rel_offset(loop_top, back_idx)?;
        self.emit(OpCode::Jmp(back_off));

        let ctx = self.loop_stack.pop().expect("loop_stack");
        // continue: already popped in compile_continue, just route to cond check.
        for pidx in ctx.continue_patches {
            self.patch_jmp_at(pidx, loop_top)?;
        }
        let exit_addr = self.code.len();
        self.patch_jz_at(jz_idx, exit_addr)?;
        for pidx in ctx.break_patches {
            self.patch_jmp_at(pidx, exit_addr)?;
        }
        Ok(result_reg)
    }

    pub(in crate::compiler) fn compile_loop(
        &mut self,
        body: &ast::Block,
    ) -> Result<Register, String> {
        let result_reg = self.alloc_register()?;
        let unit_idx = self.add_constant(Value::UNIT)?;
        self.emit(OpCode::PushConst(result_reg, unit_idx));

        // Per-iteration region: back-jump lands BEFORE the push so each iter
        // gets a fresh region. break/continue emit their own pop before Jmp.
        let loop_top = self.code.len();
        self.emit_region_push()?;

        self.loop_stack.push(LoopCtx {
            result_reg,
            continue_target: loop_top,
            break_patches: Vec::new(),
            continue_patches: Vec::new(),
        });

        self.compile_block(body)?;

        // Normal iter end: pop, then back-jump to top (which re-pushes).
        self.emit_region_pop()?;
        let back_idx = self.code.len();
        let back_off = self.rel_offset(loop_top, back_idx)?;
        self.emit(OpCode::Jmp(back_off));

        let ctx = self.loop_stack.pop().expect("loop_stack");
        for pidx in ctx.continue_patches {
            self.patch_jmp_at(pidx, loop_top)?;
        }
        let exit_addr = self.code.len();
        for pidx in ctx.break_patches {
            self.patch_jmp_at(pidx, exit_addr)?;
        }
        Ok(result_reg)
    }

    pub(in crate::compiler) fn compile_break(
        &mut self,
        value: Option<&ast::Spanned<ast::Expr>>,
    ) -> Result<Register, String> {
        if self.loop_stack.is_empty() {
            return Err("`break` outside of loop".to_string());
        }
        if let Some(v) = value {
            let r = self.compile_expr(v)?;
            let dest = self.loop_stack.last().unwrap().result_reg;
            self.emit(OpCode::Copy(dest, r));
        }
        // Pop the current iteration's region before jumping out: each iter
        // pushes one region (see compile_loop/while/for), and break bypasses
        // the normal end-of-iter pop.
        self.emit_region_pop()?;
        let pidx = self.code.len();
        self.emit(OpCode::Jmp(0));
        self.loop_stack.last_mut().unwrap().break_patches.push(pidx);

        let r = self.alloc_register()?;
        let idx = self.add_constant(Value::UNIT)?;
        self.emit(OpCode::PushConst(r, idx));
        Ok(r)
    }

    pub(in crate::compiler) fn compile_continue(&mut self) -> Result<Register, String> {
        if self.loop_stack.is_empty() {
            return Err("`continue` outside of loop".to_string());
        }
        // Pop the current iteration's region before jumping. The patch target
        // (set by compile_loop/while/for) routes back to the loop top (which
        // re-pushes) or to the for-increment block.
        self.emit_region_pop()?;
        let pidx = self.code.len();
        self.emit(OpCode::Jmp(0));
        self.loop_stack
            .last_mut()
            .unwrap()
            .continue_patches
            .push(pidx);

        let r = self.alloc_register()?;
        let idx = self.add_constant(Value::UNIT)?;
        self.emit(OpCode::PushConst(r, idx));
        Ok(r)
    }

    pub(in crate::compiler) fn compile_for(
        &mut self,
        pattern: &ast::Spanned<ast::Pattern>,
        iter: &ast::Spanned<ast::Expr>,
        body: &ast::Block,
    ) -> Result<Register, String> {
        // Three forms: inline Range expr (fast path, inclusivity known at
        // compile time), Range value (read 3-field heap cell), or any other
        // iter type (error).
        let (start_reg, end_reg, inclusive_kind) = match &iter.node {
            ast::Expr::Range { start, end, inclusive } => {
                let s = match start {
                    Some(e) => self.compile_expr(e)?,
                    None => {
                        let r = self.alloc_register()?;
                        let i = self.add_constant(Value::from_int(0))?;
                        self.emit(OpCode::PushConst(r, i));
                        r
                    }
                };
                let e = match end {
                    Some(e) => self.compile_expr(e)?,
                    None => return Err("for loop requires bounded range".into()),
                };
                (s, e, InclusiveKind::Static(*inclusive))
            }
            _ => {
                // Range value: 3-field heap cell {0: start, 1: end, 2: inclusive flag}.
                let handle = self.compile_expr(iter)?;
                let s = self.alloc_register()?;
                self.emit(OpCode::Ld(s, handle, 0));
                let e = self.alloc_register()?;
                self.emit(OpCode::Ld(e, handle, 1));
                let inc = self.alloc_register()?;
                self.emit(OpCode::Ld(inc, handle, 2));
                (s, e, InclusiveKind::Dynamic(inc))
            }
        };

        let counter = self.alloc_register()?;
        self.emit(OpCode::Copy(counter, start_reg));

        let result_reg = self.alloc_register()?;
        let unit_idx = self.add_constant(Value::UNIT)?;
        self.emit(OpCode::PushConst(result_reg, unit_idx));

        // Per-iteration region: cond check + Jz live BEFORE the push so the
        // exit path on a false cond doesn't owe a pop. break/continue emit
        // their own pop before Jmp.
        let cond_addr = self.code.len();
        let cmp_reg = self.alloc_register()?;
        match inclusive_kind {
            InclusiveKind::Static(true) => self.emit(OpCode::Lte(cmp_reg, counter, end_reg)),
            InclusiveKind::Static(false) => self.emit(OpCode::Lt(cmp_reg, counter, end_reg)),
            InclusiveKind::Dynamic(inc_reg) => {
                // (counter < end + inc) where inc is 0 or 1 collapses
                // exclusive (inc=0) and inclusive (inc=1) into a single Lt.
                let eff_end = self.alloc_register()?;
                self.emit(OpCode::Add(eff_end, end_reg, inc_reg));
                self.emit(OpCode::Lt(cmp_reg, counter, eff_end));
            }
        }
        let jz_idx = self.code.len();
        self.emit(OpCode::Jz(cmp_reg, 0));

        self.emit_region_push()?;

        self.loop_stack.push(LoopCtx {
            result_reg,
            continue_target: 0,
            break_patches: Vec::new(),
            continue_patches: Vec::new(),
        });

        if let ast::Pattern::Bind(name) = &pattern.node {
            let var_reg = self.alloc_register()?;
            self.emit(OpCode::Copy(var_reg, counter));
            self.var_to_reg.insert(name.clone(), var_reg);
            self.var_types.insert(name.clone(), ast::Type::Named("Int".into()));
        }

        self.compile_block(body)?;

        if let ast::Pattern::Bind(name) = &pattern.node {
            self.var_to_reg.remove(name);
            self.var_types.remove(name);
        }

        // Normal iter end: pop, then run the increment step, then back-jump.
        // `continue` jumps to `continue_addr` (the increment step) — it has
        // already emitted its own pop in compile_continue.
        self.emit_region_pop()?;
        let continue_addr = self.code.len();

        let one_reg = self.alloc_register()?;
        let one_idx = self.add_constant(Value::from_int(1))?;
        self.emit(OpCode::PushConst(one_reg, one_idx));
        let next = self.alloc_register()?;
        self.emit(OpCode::Add(next, counter, one_reg));
        self.emit(OpCode::Copy(counter, next));

        let back_idx = self.code.len();
        let back_off = self.rel_offset(cond_addr, back_idx)?;
        self.emit(OpCode::Jmp(back_off));

        let ctx = self.loop_stack.pop().expect("loop_stack");
        for pidx in ctx.continue_patches {
            self.patch_jmp_at(pidx, continue_addr)?;
        }
        let exit_addr = self.code.len();
        self.patch_jz_at(jz_idx, exit_addr)?;
        for pidx in ctx.break_patches {
            self.patch_jmp_at(pidx, exit_addr)?;
        }

        Ok(result_reg)
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
        // Return unwinds every enclosing loop's per-iteration region. break
        // only escapes one loop (single pop, handled in compile_break), but
        // return jumps clear out of the function, so it must pop them all.
        self.emit_pops_to_exit_fn()?;
        self.emit(OpCode::Ret(ret_reg));
        Ok(r)
    }

    // Emit one region_pop per enclosing loop currently active in the codegen
    // state. Used by `return` and `throw`, which leave the function and so
    // bypass every loop's normal end-of-iter pop.
    pub(in crate::compiler) fn emit_pops_to_exit_fn(&mut self) -> Result<(), String> {
        for _ in 0..self.loop_stack.len() {
            self.emit_region_pop()?;
        }
        Ok(())
    }
}

enum InclusiveKind {
    Static(bool),
    Dynamic(Register),
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
