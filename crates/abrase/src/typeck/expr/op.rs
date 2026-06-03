use crate::ast;
use crate::ast::Spanned;
use crate::ty::Type;
use super::super::*;

impl Checker {
    pub(super) fn infer_unary(&mut self, op: &ast::UnaryOp, right: &Spanned<ast::Expr>, span: ast::Span) -> Type {
        self.context_stack.push(format!("In unary operation {:?}", op));
        let result = match op {
            ast::UnaryOp::Ref => self.infer_borrow(right, false, span),
            ast::UnaryOp::RefMut => self.infer_borrow(right, true, span),
            ast::UnaryOp::Not => {
                let r_ty = self.infer_expr(right);
                if r_ty == Type::Bool || r_ty == Type::Unknown { Type::Bool } else { self.report_error("Expected Bool".into(), right.span) }
            }
            ast::UnaryOp::Neg => {
                let r_ty = self.infer_expr(right);
                if r_ty == Type::Int || r_ty == Type::Float || r_ty == Type::Unknown { r_ty } else { self.report_error("Expected numeric".into(), right.span) }
            }
            ast::UnaryOp::Deref => {
                let r_ty = self.infer_expr(right);
                match r_ty {
                    Type::Reference { inner, .. } => *inner,
                    Type::Shared { inner, .. } => *inner,
                    Type::Generic { name, args } if name == "Shared" => {
                        args.into_iter().next().unwrap_or(Type::Unknown)
                    }
                    _ => self.report_error("Expected reference".into(), right.span),
                }
            }
        };
        self.context_stack.pop();
        result
    }

    fn is_scalar(ty: &Type) -> bool {
        matches!(ty, Type::Int | Type::Float | Type::Bool | Type::Char | Type::Unit)
    }

    fn infer_borrow(&mut self, place: &Spanned<ast::Expr>, is_mut: bool, span: ast::Span) -> Type {
        if let ast::Expr::Identifier(name) = &place.node {
            let res = if is_mut { self.try_mut_borrow(name, span) } else { self.try_immut_borrow(name, span) };
            return match res {
                Ok(()) => {
                    let ty = self.get_var(name, true, span);
                    if is_mut && Self::is_scalar(&ty) {
                        return self.report_error("cannot take `&mut` of a scalar value; it has no stable address, so a write through the borrow would not persist. Pass it by value, or wrap it in a record.".into(), place.span);
                    }
                    Type::Reference { is_mut, inner: Box::new(ty) }
                }
                Err(msg) => self.report_error(msg, span),
            };
        }
        if matches!(&place.node,
            ast::Expr::FieldAccess { .. } | ast::Expr::Index { .. }
            | ast::Expr::Unary { op: ast::UnaryOp::Deref, .. }
        ) {
            let inner = self.infer_expr(place);
            if is_mut {
                if Self::is_scalar(&inner) {
                    return self.report_error("cannot take `&mut` of a scalar place; it has no stable address, so a write through the borrow would not persist. Use a record field of record type, or pass by value.".into(), place.span);
                }
                if let ast::Expr::Unary { op: ast::UnaryOp::Deref, right } = &place.node {
                    if !matches!(self.infer_expr(right), Type::Reference { is_mut: true, .. } | Type::Unknown) {
                        return self.report_error("cannot mutably borrow through an immutable reference; the source must be a `&mut`".into(), place.span);
                    }
                } else if let Some(root) = Self::root_ident(place) {
                    let mutable = self.scopes.iter().rev().find_map(|s| s.vars.get(&root).map(|m| {
                        m.is_mut || matches!(&m.ty, Type::Reference { is_mut: true, .. })
                    }));
                    if let Some(false) = mutable {
                        return self.report_error(
                            format!("cannot mutably borrow through immutable binding '{}'; use `let mut {}` or a `&mut` parameter", root, root),
                            place.span,
                        );
                    }
                }
            }
            return Type::Reference { is_mut, inner: Box::new(inner) };
        }
        let msg = if is_mut { "Cannot mutably borrow temporary" } else { "Cannot borrow temporary" };
        self.report_error(msg.into(), place.span)
    }

    pub(super) fn infer_binary(&mut self, op: &ast::BinaryOp, left: &Spanned<ast::Expr>, right: &Spanned<ast::Expr>, span: ast::Span) -> Type {
        self.context_stack.push("In binary expression".into());
        if matches!(op,
            ast::BinaryOp::Assign | ast::BinaryOp::AddAssign | ast::BinaryOp::SubAssign |
            ast::BinaryOp::MulAssign | ast::BinaryOp::DivAssign | ast::BinaryOp::ModAssign
        ) {
            let ret = self.check_assignment(op, left, right);
            self.context_stack.pop();
            return ret;
        }
        let l_ty = self.infer_expr(left);
        let r_ty = self.infer_expr(right);

        let result = if l_ty == Type::Unknown || r_ty == Type::Unknown {
            Type::Unknown
        } else if l_ty == Type::Never || r_ty == Type::Never {
            Type::Never
        } else if l_ty != r_ty {
            self.report_error(format!("Type mismatch: expected {:?}, found {:?}", l_ty, r_ty), right.span)
        } else {
            match op {
                ast::BinaryOp::Add | ast::BinaryOp::Sub | ast::BinaryOp::Mul | ast::BinaryOp::Div | ast::BinaryOp::Mod => {
                    if l_ty == Type::Int || l_ty == Type::Float { l_ty } else { self.report_error("Expected numeric types".into(), span) }
                }
                ast::BinaryOp::Eq | ast::BinaryOp::Neq | ast::BinaryOp::Lt | ast::BinaryOp::Gt | ast::BinaryOp::Lte | ast::BinaryOp::Gte => {
                    Type::Bool
                }
                ast::BinaryOp::And | ast::BinaryOp::Or => {
                    if l_ty == Type::Bool { Type::Bool } else { self.report_error("Expected Bool".into(), span) }
                }
                ast::BinaryOp::BitAnd | ast::BinaryOp::BitOr | ast::BinaryOp::BitXor | ast::BinaryOp::Shl | ast::BinaryOp::Shr => {
                    if l_ty == Type::Int { Type::Int } else { self.report_error("Expected Int for bitwise op".into(), span) }
                }
                ast::BinaryOp::Assign
                | ast::BinaryOp::AddAssign | ast::BinaryOp::SubAssign
                | ast::BinaryOp::MulAssign | ast::BinaryOp::DivAssign | ast::BinaryOp::ModAssign => unreachable!(),
            }
        };
        self.context_stack.pop();
        result
    }

    pub(super) fn infer_block_expr(&mut self, block: &ast::Block) -> Type {
        let prop = self.exn_prop;
        self.enter_scope();
        self.exn_prop = false;
        for stmt in &block.stmts {
            self.check_stmt(stmt);
        }
        let ty = if let Some(ret_expr) = &block.ret {
            self.exn_prop = prop;
            self.infer_expr(ret_expr)
        } else {
            Type::Unit
        };
        self.exit_scope();
        ty
    }

    pub(super) fn infer_if(&mut self, condition: &Spanned<ast::Expr>, consequence: &Spanned<ast::Expr>, alternative: &Option<Box<Spanned<ast::Expr>>>, _span: ast::Span) -> Type {
        let prop = self.exn_prop;
        self.exn_prop = false;
        self.context_stack.push("In if condition".into());
        let cond_ty = self.infer_expr(condition);
        self.context_stack.pop();

        if cond_ty != Type::Bool && cond_ty != Type::Unknown {
            self.report_error("Condition must be Bool".into(), condition.span);
        }

        let snapshot = self.scopes.clone();
        self.exn_prop = prop;
        let cons_ty = self.infer_expr(consequence);
        if let Some(alt) = alternative {
            self.scopes = snapshot;
            self.exn_prop = prop;
            let alt_ty = self.infer_expr(alt);
            let compatible = cons_ty == alt_ty
                || cons_ty == Type::Unknown || alt_ty == Type::Unknown
                || cons_ty == Type::Never || alt_ty == Type::Never;
            if !compatible {
                self.report_error("If branch types do not match".into(), alt.span);
            }
            if prop && cons_ty != Type::Never && alt_ty != Type::Never
                && self.tail_yields_result(consequence) != self.tail_yields_result(alt)
            {
                self.report_error(
                    "in a fallible tail, both `if` branches must be uniformly fallible or \
                     uniformly plain values; add `?` to the fallible branch".into(),
                    alt.span,
                );
            }
            if cons_ty == Type::Never { alt_ty } else { cons_ty }
        } else {
            if cons_ty != Type::Unit && cons_ty != Type::Unknown && cons_ty != Type::Never {
                self.report_error(
                    format!("`if` without `else` must have Unit consequence, got {:?}", cons_ty),
                    consequence.span,
                );
            }
            Type::Unit
        }
    }
}
