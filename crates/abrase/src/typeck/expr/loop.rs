use crate::ast;
use crate::ast::Spanned;
use crate::ty::Type;
use super::super::*;

impl Checker {
    pub(super) fn infer_for(&mut self, pattern: &Spanned<ast::Pattern>, iter: &Spanned<ast::Expr>, body: &ast::Block, span: ast::Span) -> Type {
        self.context_stack.push("In for loop".into());
        let iter_ty = self.infer_expr(iter);
        self.enter_scope();
        self.loop_depth += 1;
        self.loop_break_types.push(None);
        let element_ty = self.extract_iterable_element_type(&iter_ty, iter.span);
        if let ast::Pattern::Bind(name) = &pattern.node {
            self.insert_var(name.clone(), element_ty, false, pattern.span);
        }
        self.push_region(format!("for_body_{}", self.region_stack.len()));
        self.loop_body_region_depth.push(self.region_stack.len());
        let _body_ty = self.infer_block(body);
        self.loop_body_region_depth.pop();
        self.check_borrow_barrier("for loop exit", span);
        self.pop_region();
        self.loop_depth -= 1;
        let break_ty = self.loop_break_types.pop().flatten();
        self.exit_scope();
        self.context_stack.pop();
        break_ty.unwrap_or(Type::Unit)
    }

    pub(super) fn infer_while(&mut self, condition: &Spanned<ast::Expr>, body: &ast::Block, span: ast::Span) -> Type {
        self.context_stack.push("In while loop".into());
        let cond_ty = self.infer_expr(condition);
        if cond_ty != Type::Bool && cond_ty != Type::Unknown {
            self.report_error("While condition must be Bool".into(), condition.span);
        }
        self.loop_depth += 1;
        self.loop_break_types.push(None);
        self.push_region(format!("while_body_{}", self.region_stack.len()));
        self.loop_body_region_depth.push(self.region_stack.len());
        let _body_ty = self.infer_block(body);
        self.loop_body_region_depth.pop();
        self.check_borrow_barrier("while loop exit", span);
        self.pop_region();
        self.loop_depth -= 1;
        let break_ty = self.loop_break_types.pop().flatten();
        self.context_stack.pop();
        break_ty.unwrap_or(Type::Unit)
    }

    pub(super) fn infer_loop(&mut self, body: &ast::Block, span: ast::Span) -> Type {
        self.context_stack.push("In loop".into());
        self.loop_depth += 1;
        self.loop_break_types.push(None);
        self.push_region(format!("loop_body_{}", self.region_stack.len()));
        self.loop_body_region_depth.push(self.region_stack.len());
        let _body_ty = self.infer_block(body);
        self.loop_body_region_depth.pop();
        self.check_borrow_barrier("loop exit", span);
        self.pop_region();
        self.loop_depth -= 1;
        let break_ty = self.loop_break_types.pop().flatten();
        self.context_stack.pop();
        break_ty.unwrap_or(Type::Never)
    }

    pub(super) fn infer_break(&mut self, break_val: &Option<Box<Spanned<ast::Expr>>>, span: ast::Span) -> Type {
        if self.loop_depth == 0 {
            self.report_error("Break outside of loop".into(), span);
            return Type::Never;
        }
        if let Some(val) = break_val {
            if let Some((root, s)) = self.check_break_escape(val) {
                self.report_error(
                    format!("borrow of '{}' cannot escape the loop body; it would dangle past loop exit", root),
                    s,
                );
            }
            let val_ty = self.infer_expr(val);
            let existing = self.loop_break_types.last().and_then(|s| s.clone());
            match existing {
                None => { if let Some(slot) = self.loop_break_types.last_mut() { *slot = Some(val_ty); } }
                Some(ref ex_ty) => {
                    if !self.types_compatible(ex_ty, &val_ty) && val_ty != Type::Unknown {
                        self.report_error(
                            format!("Break value type mismatch: expected {:?}, got {:?}", ex_ty, val_ty),
                            span,
                        );
                    }
                }
            }
        }
        Type::Never
    }

    pub(super) fn infer_resume(&mut self, arg: &Option<Box<Spanned<ast::Expr>>>, span: ast::Span) -> Type {
        if !self.in_handler_arm {
            self.report_error("'resume' is only valid inside a handler arm body".into(), span);
        }
        if let Some(a) = arg { let _ = self.infer_expr(a); }
        Type::Never
    }

    pub(super) fn infer_region(&mut self, label: &Option<String>, body: &ast::Block, span: ast::Span) -> Type {
        self.context_stack.push(format!("In region{}", label.as_ref().map(|l| format!(" '{}'", l)).unwrap_or_default()));
        let region_name = label.as_ref().cloned()
            .unwrap_or_else(|| format!("region_{}", self.region_stack.len()));
        self.push_region(region_name.clone());
        self.effect_stack.push(self.active_effects.clone());
        let body_ty = self.infer_block(body);
        self.effect_stack.pop();
        self.check_borrow_barrier("region exit", span);
        self.pop_region();
        self.context_stack.pop();
        if self.type_contains_shared(&body_ty) {
            self.report_error(
                format!("region '{}' result type {:?} contains `Shared<T>` — a Shared cell cannot escape its enclosing region", region_name, body_ty),
                span,
            );
        }
        body_ty
    }
}
