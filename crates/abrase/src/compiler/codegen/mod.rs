pub mod scaffold;
pub mod inference;
pub mod data;
pub mod arith;
pub mod control;
pub mod match_arm;
pub mod calls;
pub mod effects;
pub mod closure_expr;

pub(in crate::compiler) use inference::{is_move_type, is_share_type};

use crate::ast;
use crate::bytecode::{OpCode, Register};
use crate::compiler::Compiler;
use crate::bytecode::Value;

impl Compiler {
    pub(in crate::compiler) fn compile_expr(
        &mut self,
        expr: &ast::Spanned<ast::Expr>,
    ) -> Result<Register, String> {
        self.current_span = expr.span;
        if matches!(expr.node, ast::Expr::Unary { .. } | ast::Expr::Binary { .. }) {
            if let Some(lit) = self.try_const_fold(expr) {
                return self.compile_literal(&lit);
            }
        }
        match &expr.node {
            ast::Expr::Error => Err(
                "Compilation aborted: parser error was not recovered; fix parser errors first"
                    .to_string()
            ),
            ast::Expr::Literal(ast::Literal::StringInterp(parts)) => self.compile_string_interp(parts, expr.span),
            ast::Expr::Literal(lit)            => self.compile_literal(lit),
            ast::Expr::Identifier(name)        => self.compile_identifier(name),
            ast::Expr::Unary { op, right }     => self.compile_unary(op, right),
            ast::Expr::Binary { op, left, right } => self.compile_binary(op, left, right),
            ast::Expr::If { condition, consequence, alternative } => {
                self.compile_if(condition, consequence, alternative.as_deref())
            }
            ast::Expr::While { condition, body } => self.compile_while(condition, body),
            ast::Expr::Block(block)            => self.compile_block(block),
            ast::Expr::Match { scrutinee, arms } => self.compile_match(scrutinee, arms),
            ast::Expr::Call { callee, args }   => self.compile_call(callee, args, expr.span),
            ast::Expr::Return(opt_expr)        => self.compile_return(opt_expr.as_deref()),
            ast::Expr::Throw(inner)            => self.compile_throw(inner),
            ast::Expr::Question(inner)         => self.compile_question(inner),
            ast::Expr::Record { ty, fields }   => self.compile_record(ty, fields),
            ast::Expr::Variant { ty, args }    => self.compile_variant_expr(ty, args),
            ast::Expr::FieldAccess { base, field } => self.compile_field_access(base, field),
            ast::Expr::Array(items)            => self.compile_array(items),
            ast::Expr::Index { base, index }   => self.compile_index(base, index),
            ast::Expr::Resume(arg)             => self.compile_resume(arg.as_deref()),
            ast::Expr::Handle { expr: body, arms } => self.compile_handle(body, expr.span, arms),
            ast::Expr::Closure { .. }          => self.compile_closure(expr.span),
            ast::Expr::For { pattern, iter, body } => self.compile_for(pattern, iter, body),
            ast::Expr::Loop { body }     => self.compile_loop(body),
            ast::Expr::Break(val)        => self.compile_break(val.as_deref()),
            ast::Expr::Continue          => self.compile_continue(),
            ast::Expr::Tuple(items)      => self.compile_tuple(items),
            ast::Expr::ArrayRepeat { elem, count } => self.compile_array_repeat(elem, count),
            ast::Expr::Range { start, end, inclusive } => {
                self.compile_range(start.as_deref(), end.as_deref(), *inclusive)
            }
            ast::Expr::Region { body, .. } => {
                let body_ty = body.ret.as_ref().and_then(|e| self.infer_expr_type(e));
                self.emit_region_push()?;
                let result = self.compile_block(body)?;
                if let Some(ty) = &body_ty {
                    self.emit_region_forget_typed(result, ty)?;
                } else {
                    self.emit_region_forget(result)?;
                }
                self.emit_region_pop()?;
                Ok(result)
            }
        }
    }

    pub(in crate::compiler) fn compile_stmt(
        &mut self,
        stmt: &ast::Spanned<ast::Stmt>,
    ) -> Result<(), String> {
        let mark = self.snapshot_register_high_water();
        let r = self.compile_stmt_inner(stmt);
        self.reclaim_temp_regs_above(mark);
        r
    }

    // Rewind next_reg but keep regs in long-lived structures.
    pub(in crate::compiler) fn reclaim_temp_regs_above(&mut self, mark: u16) {
        let mut protect = mark;
        for reg in self.var_to_reg.values() {
            protect = protect.max(reg.0 as u16 + 1);
        }
        for reg in &self.handler_table_stack {
            protect = protect.max(reg.0 as u16 + 1);
        }
        for ctx in &self.loop_stack {
            protect = protect.max(ctx.result_reg.0 as u16 + 1);
        }
        for envs in &self.arm_env_stack {
            for reg in envs.values() {
                protect = protect.max(reg.0 as u16 + 1);
            }
        }
        let high = self.snapshot_register_high_water();
        for r in protect..high {
            if self.reg_holds_handle.get(r as usize).copied().unwrap_or(false) {
                self.emit(OpCode::Drop(Register(r as u8)));
            }
        }
        self.restore_register_high_water(protect);
    }

    fn compile_stmt_inner(
        &mut self,
        stmt: &ast::Spanned<ast::Stmt>,
    ) -> Result<(), String> {
        self.current_span = stmt.span;
        match &stmt.node {
            ast::Stmt::Let { pattern, value, ty, .. } => {
                if let ast::Pattern::Bind(name) = &pattern.node {
                    let inferred_ty = ty.clone().or_else(|| self.infer_expr_type(value));
                    let src_reg = self.compile_expr(value)?;
                    let bound_reg = if let ast::Expr::Identifier(src_name) = &value.node {
                        let dest = self.alloc_register()?;
                        let is_move = inferred_ty.as_ref().map(is_move_type).unwrap_or(false);
                        let is_share = !is_move && inferred_ty.as_ref().map(is_share_type).unwrap_or(false);
                        let src_name_owned = src_name.clone();
                        if is_move || (is_share && self.consume_use(&src_name_owned)) {
                            self.emit(OpCode::Move(dest, src_reg));
                            self.var_to_reg.remove(src_name);
                            self.var_types.remove(src_name);
                        } else {
                            self.emit(OpCode::Copy(dest, src_reg));
                        }
                        dest
                    } else {
                        src_reg
                    };
                    self.var_to_reg.insert(name.clone(), bound_reg);
                    self.var_bound_at_region.insert(name.clone(), self.compiler_region_depth);
                    if let Some(t) = inferred_ty {
                        self.var_types.insert(name.clone(), t);
                    }
                    if let ast::Expr::Closure { .. } = &value.node {
                        if let Some(info) = self.closure_by_span.get(&value.span).cloned() {
                            self.closure_by_var.insert(name.clone(), info);
                        }
                    }
                    if let Some(top) = self.block_locals_stack.last_mut() {
                        top.push(bound_reg);
                    }
                    Ok(())
                } else {
                    Err("Only simple bindings supported".to_string())
                }
            }
            ast::Stmt::Expr(expr) => {
                let is_block = matches!(&expr.node, ast::Expr::Block(_));
                if is_block { self.emit_region_push()?; }
                let reg = self.compile_expr(expr)?;
                let needs_drop = self.infer_expr_type(expr)
                    .as_ref()
                    .map(is_move_type)
                    .unwrap_or(false);
                if needs_drop {
                    self.emit(OpCode::Drop(reg));
                }
                if is_block { self.emit_region_pop()?; }
                Ok(())
            }
            ast::Stmt::Empty => Ok(()),
        }
    }

    pub(in crate::compiler) fn compile_block(
        &mut self,
        block: &ast::Block,
    ) -> Result<Register, String> {
        let pre_bindings: std::collections::HashSet<String> = self.var_to_reg.keys().cloned().collect();
        let mark = self.snapshot_register_high_water();
        self.block_locals_stack.push(Vec::new());
        for stmt in &block.stmts {
            self.compile_stmt(stmt)?;
        }
        let result = if let Some(ret) = &block.ret {
            self.compile_expr(ret)?
        } else {
            let reg = self.alloc_register()?;
            let idx = self.add_constant(Value::UNIT)?;
            self.emit(OpCode::PushConst(reg, idx));
            reg
        };
        self.emit_drops_for_exit(1, Some(result))?;
        self.block_locals_stack.pop();
        let new_bindings: Vec<String> = self.var_to_reg.keys()
            .filter(|k| !pre_bindings.contains(*k))
            .cloned()
            .collect();
        for name in &new_bindings {
            self.var_to_reg.remove(name);
            self.var_types.remove(name);
        }
        let floor = mark.max(result.0 as u16 + 1);
        self.reclaim_temp_regs_above(floor);
        Ok(result)
    }

    pub(in crate::compiler) fn finalize_arg_patches(&mut self) -> Result<(), String> {
        let reg_count = self.max_reg as usize;
        for (pos, slot) in std::mem::take(&mut self.pending_arg_patches) {
            let dst_idx = reg_count + slot as usize;
            if dst_idx > u8::MAX as usize {
                return Err(format!("Arg-passing register index {} exceeds u8 range", dst_idx));
            }
            let dst = Register(dst_idx as u8);
            match self.code[pos] {
                OpCode::Copy(_, src) => { self.code[pos] = OpCode::Copy(dst, src); }
                OpCode::Move(_, src) => { self.code[pos] = OpCode::Move(dst, src); }
                ref other => return Err(format!(
                    "internal: arg-patch site at pc {} is not Copy/Move (found {:?})",
                    pos, other
                )),
            }
        }
        Ok(())
    }
}
