use crate::ast;
use crate::ast::Spanned;
use crate::ty::Type;
use super::*;

pub(self) fn types_assignable(expected: &Type, actual: &Type) -> bool {
    if expected == &Type::Unknown || actual == &Type::Unknown { return true; }
    if expected == &Type::Never || actual == &Type::Never { return true; }
    match (expected, actual) {
        (Type::Shared { inner: ei, region: er }, Type::Shared { inner: ai, region: ar }) => {
            if !types_assignable(ei, ai) { return false; }
            match (er, ar) {
                (None, _) | (_, None) => false,
                (Some(a), Some(b)) => a == b,
            }
        }
        _ => expected == actual,
    }
}

pub mod call;
pub mod r#match;
pub mod closure;
pub mod handle;
pub mod escape;
pub mod op;
pub mod r#loop;
pub mod data;

impl Checker {

    pub(super) fn type_contains_shared(&self, ty: &Type) -> bool {
        let mut visited = std::collections::HashSet::new();
        self.type_contains_shared_inner(ty, &mut visited)
    }

    fn type_contains_shared_inner(
        &self,
        ty: &Type,
        visited: &mut std::collections::HashSet<String>,
    ) -> bool {
        match ty {
            Type::Shared { .. } => true,
            Type::Generic { name, .. } if name == "Shared" => true,
            Type::Generic { args, .. } => args.iter().any(|t| self.type_contains_shared_inner(t, visited)),
            Type::Tuple(elems) => elems.iter().any(|t| self.type_contains_shared_inner(t, visited)),
            Type::Reference { inner, .. } => self.type_contains_shared_inner(inner, visited),
            Type::Function { params, ret, .. } => {
                params.iter().any(|t| self.type_contains_shared_inner(t, visited))
                    || self.type_contains_shared_inner(ret, visited)
            }
            Type::Named(name) => {
                if !visited.insert(name.clone()) { return false; }
                match self.type_registry.get(name) {
                    Some(ast::TypeBody::Record(fields)) => fields.iter().any(|f| {
                        let ft = self.convert_type(&f.ty);
                        self.type_contains_shared_inner(&ft, visited)
                    }),
                    Some(ast::TypeBody::Variant(cases)) => cases.iter().any(|c| match c {
                        ast::VariantCase::Unit(_) => false,
                        ast::VariantCase::Tuple(_, tys) => tys.iter().any(|t| {
                            let tt = self.convert_type(t);
                            self.type_contains_shared_inner(&tt, visited)
                        }),
                        ast::VariantCase::Record(_, fields) => fields.iter().any(|f| {
                            let ft = self.convert_type(&f.ty);
                            self.type_contains_shared_inner(&ft, visited)
                        }),
                    }),
                    None => false,
                }
            }
            _ => false,
        }
    }

    fn check_assignment(&mut self, _op: &ast::BinaryOp, left: &Spanned<ast::Expr>, right: &Spanned<ast::Expr>) -> Type {
        let lhs_name = if let ast::Expr::Identifier(n) = &left.node { Some(n.clone()) } else { None };
        let l_ty = lhs_name.as_ref().and_then(|n| self.peek_var(n)).unwrap_or_else(|| self.infer_expr(left));
        let r_ty = self.infer_expr(right);
        if l_ty != Type::Unknown && r_ty != Type::Unknown && l_ty != Type::Never && r_ty != Type::Never && l_ty != r_ty {
            self.report_error(format!("Type mismatch: expected {:?}, found {:?}", l_ty, r_ty), right.span);
        }
        if let Some(name) = lhs_name {
            let is_mut = self.scopes.iter().rev()
                .find_map(|s| s.vars.get(&name).map(|m| m.is_mut))
                .or_else(|| self.static_vars.contains(&name).then_some(false));
            if let Some(false) = is_mut {
                let kind = if self.static_vars.contains(&name) { "static" } else { "binding" };
                self.report_error(
                    format!("Cannot assign to immutable {} '{}'; use `let mut {}` to allow mutation", kind, name, name),
                    left.span,
                );
            }
            for scope in self.scopes.iter_mut().rev() {
                if let Some(meta) = scope.vars.get_mut(&name) { meta.is_moved = false; meta.moved_at = None; break; }
            }
        } else if matches!(&left.node, ast::Expr::Index { .. } | ast::Expr::FieldAccess { .. }) {
            if let Some(root) = Self::root_ident(left) {
                let is_mut = self.scopes.iter().rev().find_map(|s| s.vars.get(&root).map(|m| {
                    m.is_mut || matches!(&m.ty, Type::Reference { is_mut: true, .. })
                }));
                if let Some(false) = is_mut {
                    self.report_error(
                        format!("Cannot mutate through immutable binding '{}'; use `let mut {}` to allow mutation", root, root),
                        left.span,
                    );
                }
            }
        }
        Type::Unit
    }

    pub fn infer_expr(&mut self, expr: &Spanned<ast::Expr>) -> Type {
        let ty = self.infer_expr_inner(expr);
        if !matches!(ty, Type::Unknown) {
            let module = match self.current_module.split_first() {
                Some((head, rest)) if head == "root" => rest.to_vec(),
                _ => self.current_module.clone(),
            };
            self.expr_types.insert((module, expr.span, std::mem::discriminant(&expr.node)), ty.clone());
        }
        ty
    }

    fn infer_expr_inner(&mut self, expr: &Spanned<ast::Expr>) -> Type {
        let prop = self.exn_prop;
        self.exn_prop = false;
        match &expr.node {
            ast::Expr::Error                                => Type::Unknown,
            ast::Expr::Paren(inner)                        => { self.exn_prop = prop; self.infer_expr(inner) }
            ast::Expr::Literal(lit)                        => self.infer_literal(lit, expr.span),
            ast::Expr::Identifier(name)                    => self.get_var(name, false, expr.span),
            ast::Expr::Unary { op, right }                 => self.infer_unary(op, right, expr.span),
            ast::Expr::Binary { op, left, right }          => self.infer_binary(op, left, right, expr.span),
            ast::Expr::Block(block)                        => { self.exn_prop = prop; self.infer_block_expr(block) }
            ast::Expr::If { condition, consequence, alternative } => { self.exn_prop = prop; self.infer_if(condition, consequence, alternative, expr.span) }
            ast::Expr::Match { scrutinee, arms }           => { self.exn_prop = prop; self.infer_match(scrutinee, arms, expr.span) }
            ast::Expr::For { pattern, iter, body }         => self.infer_for(pattern, iter, body, expr.span),
            ast::Expr::While { condition, body }           => self.infer_while(condition, body, expr.span),
            ast::Expr::Loop { body }                       => self.infer_loop(body, expr.span),
            ast::Expr::Break(break_val)                    => self.infer_break(break_val, expr.span),
            ast::Expr::Continue                            => {
                if self.loop_depth == 0 { self.report_error("Continue outside of loop".into(), expr.span); }
                Type::Never
            }
            ast::Expr::Return(ret_val) => {
                if let Some(val) = ret_val {
                    if let Some((root, span)) = self.check_return_escape(val) {
                        self.report_error(
                            format!("borrow of '{}' cannot escape via return; it would dangle past loop / function exit", root),
                            span,
                        );
                    }
                    let _val_ty = self.infer_expr(val);
                }
                Type::Never
            }
            ast::Expr::Throw(expr_val) => {
                if let Some((root, span)) = self.check_return_escape(expr_val) {
                    self.report_error(
                        format!("borrow of '{}' cannot escape via throw; it would dangle past loop / function exit", root),
                        span,
                    );
                }
                let ex_ty = self.infer_expr(expr_val);
                self.add_required_effect(crate::ty::Effect::Exn(Box::new(ex_ty)));
                Type::Never
            }
            ast::Expr::Call { callee, args }               => { self.exn_prop = prop; self.infer_call(callee, args, expr.span) }
            ast::Expr::Tuple(elems)                        => self.infer_tuple(elems),
            ast::Expr::Array(elems)                        => self.infer_array(elems),
            ast::Expr::ArrayRepeat { elem, count }         => self.infer_array_repeat(elem, count),
            ast::Expr::Index { base, index }               => self.infer_index(base, index),
            ast::Expr::FieldAccess { base, field }         => self.infer_field_access(base, field),
            ast::Expr::Closure { is_move, params, effects, return_type, body } => self.infer_closure(*is_move, params, effects, return_type, body),
            ast::Expr::Record { ty, fields }               => self.infer_record(ty, fields, expr.span),
            ast::Expr::Variant { ty, args }                => self.infer_variant(ty, args),
            ast::Expr::Range { start, end, inclusive: _ }  => self.infer_range(start, end),
            ast::Expr::Question(inner)                     => self.infer_question(inner),
            ast::Expr::Resume(arg)                         => self.infer_resume(arg, expr.span),
            ast::Expr::Region { label, body }              => self.infer_region(label, body, expr.span),
            ast::Expr::Handle { expr: handler_expr, arms } => self.infer_handle(handler_expr, arms, expr.span),
        }
    }

    // A `let r = &x` / `let r = &mut x` binding keeps its borrow alive past the
    // statement (until scope exit); every other statement's borrows are temporary.
    fn stmt_binds_named_borrow(stmt: &Spanned<ast::Stmt>) -> bool {
        if let ast::Stmt::Let { value, .. } = &stmt.node {
            if let ast::Expr::Unary { op: ast::UnaryOp::Ref | ast::UnaryOp::RefMut, right } = &value.node {
                return matches!(&right.node, ast::Expr::Identifier(_));
            }
        }
        false
    }

    pub fn infer_block(&mut self, block: &ast::Block) -> Type {
        let prop = self.exn_prop;
        self.enter_scope();
        self.exn_prop = false;
        for stmt in &block.stmts {
            let mark = self.borrow_stack.len();
            self.check_stmt(stmt);
            if !Self::stmt_binds_named_borrow(stmt) {
                self.release_borrows_to(mark);
            }
        }
        let ty = if let Some(ret_expr) = &block.ret {
            self.exn_prop = prop;
            self.infer_expr(ret_expr)
        } else { Type::Unit };
        self.exit_scope();
        ty
    }
}
