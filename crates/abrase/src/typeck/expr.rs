use crate::ast;
use crate::ast::Spanned;
use crate::ty::Type;
use super::*;

fn types_assignable(expected: &Type, actual: &Type) -> bool {
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

impl Checker {

    // Overload resolution for a call site. If `name` has registered overloads,
    // pick the one whose params match arg types inferred from literals / known
    // variable bindings. Returns the matched Type::Function so the existing
    // arg-checking loop can use it.
    pub(super) fn resolve_call_overload(
        &self,
        name: &str,
        args: &[Spanned<ast::Expr>],
    ) -> Option<Type> {
        let sigs = self.fn_overloads.get(name)?;
        if sigs.is_empty() { return None; }
        let arg_tys: Vec<Option<Type>> =
            args.iter().map(|a| self.quick_arg_type(a)).collect();
        for (params, ret) in sigs {
            if params.len() != arg_tys.len() { continue; }
            let ok = params.iter().zip(&arg_tys).all(|(p, a)| match a {
                Some(t) => p == t || matches!(p, Type::Unknown) || matches!(t, Type::Unknown),
                None => false,
            });
            if ok {
                return Some(Type::Function {
                    params: params.clone(),
                    effects: vec![],
                    ret: Box::new(ret.clone()),
                });
            }
        }
        None
    }

    // Side-effect-free arg type peek. Covers literals, simple identifiers, and
    // arithmetic/comparison binops whose operand types are themselves peek-able.
    // Returns None for anything more complex; the caller then falls back to
    // full inference (which is allowed to report errors).
    fn quick_arg_type(&self, expr: &Spanned<ast::Expr>) -> Option<Type> {
        match &expr.node {
            ast::Expr::Literal(lit) => Some(match lit {
                ast::Literal::Int(_)    => Type::Int,
                ast::Literal::Float(_)  => Type::Float,
                ast::Literal::Bool(_)   => Type::Bool,
                ast::Literal::Char(_)   => Type::Char,
                ast::Literal::String(_) => Type::String,
                ast::Literal::Unit      => Type::Unit,
                _ => return None,
            }),
            ast::Expr::Identifier(name) => {
                for scope in self.scopes.iter().rev() {
                    if let Some(meta) = scope.vars.get(name) {
                        return Some(meta.ty.clone());
                    }
                }
                None
            }
            // Arithmetic: result type = operand type (when both peek-able and
            // agree). Comparisons / logical ops yield Bool regardless.
            ast::Expr::Binary { op, left, right } => {
                use ast::BinaryOp as B;
                match op {
                    B::Add | B::Sub | B::Mul | B::Div | B::Mod
                    | B::AddAssign | B::SubAssign | B::MulAssign
                    | B::DivAssign | B::ModAssign => {
                        let lt = self.quick_arg_type(left)?;
                        let rt = self.quick_arg_type(right)?;
                        if lt == rt { Some(lt) } else { None }
                    }
                    B::Eq | B::Neq | B::Lt | B::Gt | B::Lte | B::Gte
                    | B::And | B::Or => Some(Type::Bool),
                    B::Assign => None,
                }
            }
            ast::Expr::Unary { op, right } => {
                use ast::UnaryOp as U;
                match op {
                    U::Neg => self.quick_arg_type(right),
                    U::Not => Some(Type::Bool),
                    _ => None,
                }
            }
            _ => None,
        }
    }

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
                if !visited.insert(name.clone()) { return false; } // cycle guard
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

    fn check_assignment(
        &mut self,
        _op: &ast::BinaryOp,
        left: &Spanned<ast::Expr>,
        right: &Spanned<ast::Expr>,
    ) -> Type {
        let lhs_name = if let ast::Expr::Identifier(n) = &left.node { Some(n.clone()) } else { None };
        let l_ty = lhs_name.as_ref()
            .and_then(|n| self.peek_var(n))
            .unwrap_or_else(|| self.infer_expr(left));
        let r_ty = self.infer_expr(right);
        if l_ty != Type::Unknown && r_ty != Type::Unknown && l_ty != Type::Never && r_ty != Type::Never && l_ty != r_ty {
            self.report_error(format!("Type mismatch: expected {:?}, found {:?}", l_ty, r_ty), right.span);
        }
        if let Some(name) = lhs_name {
            let is_mut = self.scopes.iter().rev()
                .find_map(|s| s.vars.get(&name).map(|m| m.is_mut));
            if let Some(false) = is_mut {
                self.report_error(
                    format!("Cannot assign to immutable binding '{}'; \
                             use `let mut {}` to allow mutation", name, name),
                    left.span,
                );
            }
            for scope in self.scopes.iter_mut().rev() {
                if let Some(meta) = scope.vars.get_mut(&name) {
                    meta.is_moved = false;
                    meta.moved_at = None;
                    break;
                }
            }
        }
        Type::Unit
    }

    pub fn infer_expr(&mut self, expr: &Spanned<ast::Expr>) -> Type {
        match &expr.node {
            ast::Expr::Error => Type::Unknown,
            // single source of truth for literal typing.
            ast::Expr::Literal(lit) => self.infer_literal(lit, expr.span),
            ast::Expr::Identifier(name) => self.get_var(name, false, expr.span),
            ast::Expr::Unary { op, right } => {
                self.context_stack.push(format!("In unary operation {:?}", op));

                let result = match op {
                    ast::UnaryOp::Ref => {
                        if let ast::Expr::Identifier(name) = &right.node {
                            match self.try_immut_borrow(name, expr.span) {
                                Ok(()) => {
                                    let ty = self.get_var(name, true, expr.span);
                                    Type::Reference { is_mut: false, inner: Box::new(ty) }
                                }
                                Err(msg) => self.report_error(msg, expr.span)
                            }
                        } else {
                            self.report_error("Cannot borrow temporary".into(), right.span)
                        }
                    }
                    ast::UnaryOp::RefMut => {
                        if let ast::Expr::Identifier(name) = &right.node {
                            match self.try_mut_borrow(name, expr.span) {
                                Ok(()) => {
                                    let ty = self.get_var(name, true, expr.span);
                                    Type::Reference { is_mut: true, inner: Box::new(ty) }
                                }
                                Err(msg) => self.report_error(msg, expr.span)
                            }
                        } else {
                            self.report_error("Cannot mutably borrow temporary".into(), right.span)
                        }
                    }
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
                            // `Shared<T>` is a host-provided reference cell:
                            // `*cell` reads the inner T.
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
            ast::Expr::Binary { op, left, right } => {
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
                            if l_ty == Type::Int || l_ty == Type::Float { l_ty } else { self.report_error("Expected numeric types".into(), expr.span) }
                        }
                        ast::BinaryOp::Eq | ast::BinaryOp::Neq | ast::BinaryOp::Lt | ast::BinaryOp::Gt | ast::BinaryOp::Lte | ast::BinaryOp::Gte => {
                            Type::Bool
                        }
                        ast::BinaryOp::And | ast::BinaryOp::Or => {
                            if l_ty == Type::Bool { Type::Bool } else { self.report_error("Expected Bool".into(), expr.span) }
                        }
                        ast::BinaryOp::Assign
                        | ast::BinaryOp::AddAssign | ast::BinaryOp::SubAssign
                        | ast::BinaryOp::MulAssign | ast::BinaryOp::DivAssign
                        | ast::BinaryOp::ModAssign => unreachable!("handled by check_assignment"),
                    }
                };
                self.context_stack.pop();
                result
            }
            ast::Expr::Block(block) => {
                self.enter_scope();
                for stmt in &block.stmts {
                    self.check_stmt(stmt);
                }
                let ty = if let Some(ret_expr) = &block.ret {
                    self.infer_expr(ret_expr)
                } else {
                    Type::Unit
                };
                self.exit_scope();
                ty
            }
            ast::Expr::If { condition, consequence, alternative } => {
                self.context_stack.push("In if condition".into());
                let cond_ty = self.infer_expr(condition);
                self.context_stack.pop();

                if cond_ty != Type::Bool && cond_ty != Type::Unknown {
                    self.report_error("Condition must be Bool".into(), condition.span);
                }

                let snapshot = self.scopes.clone();
                let cons_ty = self.infer_expr(consequence);
                let result = if let Some(alt) = alternative {
                    self.scopes = snapshot;
                    let alt_ty = self.infer_expr(alt);
                    let compatible = cons_ty == alt_ty
                        || cons_ty == Type::Unknown || alt_ty == Type::Unknown
                        || cons_ty == Type::Never || alt_ty == Type::Never;
                    if !compatible {
                        self.report_error("If branch types do not match".into(), alt.span);
                    }
                    if cons_ty == Type::Never { alt_ty } else { cons_ty.clone() }
                } else {
                    if cons_ty != Type::Unit && cons_ty != Type::Never && cons_ty != Type::Unknown {
                        self.report_error(
                            format!("`if` without `else` must have () consequence, got {:?}", cons_ty),
                            consequence.span,
                        );
                    }
                    Type::Unit
                };
                result
            }
            ast::Expr::Match { scrutinee, arms } => {
                self.context_stack.push("In match expression".into());
                let required_before = self.fn_required_effects.clone();
                let scrutinee_ty = if let ast::Expr::Identifier(name) = &scrutinee.node {
                    self.get_var(name, true, scrutinee.span)
                } else {
                    self.infer_expr(scrutinee)
                };
                let exn_added: Vec<_> = self.fn_required_effects.iter()
                    .filter(|e| !required_before.iter().any(|b| self.effects_equal(b, e))
                        && matches!(e, crate::ty::Effect::Exn(_)))
                    .cloned()
                    .collect();

                // Pattern type checking and exhaustiveness analysis
                for arm in arms {
                    self.check_pattern(&arm.pattern, &scrutinee_ty, arm.pattern.span);
                }

                // If scrutinee produced an exn effect and arms cover Ok+Err, treat as handled
                if !exn_added.is_empty() && Self::arms_cover_ok_err(arms) {
                    self.fn_required_effects.retain(|e| !matches!(e, crate::ty::Effect::Exn(_)));
                    for e in required_before.iter() {
                        if matches!(e, crate::ty::Effect::Exn(_))
                            && !self.fn_required_effects.iter().any(|x| std::mem::discriminant(x) == std::mem::discriminant(e) && x == e)
                        {
                            self.fn_required_effects.push(e.clone());
                        }
                    }
                }

                // Check variant exhaustiveness for Named types
                if let Type::Named(type_name) = &scrutinee_ty {
                    let type_name = type_name.clone();
                    let (covered, has_wildcard) = Self::collect_arm_patterns(arms);
                    self.check_variant_exhaustiveness(&type_name, &covered, has_wildcard, expr.span);
                }

                let mut arm_types = Vec::new();
                // Mutually exclusive arms: snapshot scope/effects before each, union deltas.
                let pre_arm_snapshot = self.scopes.clone();
                let pre_arm_effects = self.fn_required_effects.clone();
                let mut arm_effects: Vec<crate::ty::Effect> = Vec::new();
                for arm in arms {
                    self.scopes = pre_arm_snapshot.clone();
                    self.fn_required_effects = pre_arm_effects.clone();
                    self.check_pattern(&arm.pattern, &scrutinee_ty, arm.pattern.span);
                    if let Some(guard) = &arm.guard {
                        let guard_ty = self.infer_expr(guard);
                        if guard_ty != Type::Bool && guard_ty != Type::Unknown {
                            self.report_error("Guard must be Bool".into(), guard.span);
                        }
                    }
                    let body_ty = self.infer_expr(&arm.body);
                    for e in self.fn_required_effects.iter() {
                        if !pre_arm_effects.iter().any(|p| self.effects_equal(p, e))
                            && !arm_effects.iter().any(|x| self.effects_equal(x, e))
                        {
                            arm_effects.push(e.clone());
                        }
                    }
                    arm_types.push(body_ty);
                }
                self.scopes = pre_arm_snapshot;
                self.fn_required_effects = pre_arm_effects;
                for e in arm_effects {
                    if !self.fn_required_effects.iter().any(|p| self.effects_equal(p, &e)) {
                        self.fn_required_effects.push(e);
                    }
                }

                // All arms must have same type
                if !arm_types.is_empty() {
                    let first = arm_types[0].clone();
                    for ty in arm_types.iter().skip(1) {
                        if *ty != first && first != Type::Unknown && *ty != Type::Unknown {
                            self.report_error("Match arm types do not match".into(), expr.span);
                        }
                    }
                }

                self.context_stack.pop();
                if arm_types.is_empty() { Type::Unknown } else { arm_types[0].clone() }
            }
            ast::Expr::For { pattern, iter, body } => {
                self.context_stack.push("In for loop".into());
                let iter_ty = self.infer_expr(iter);

                self.enter_scope();
                self.loop_depth += 1;
                self.loop_break_types.push(None);

                let element_ty = self.extract_iterable_element_type(&iter_ty, iter.span);
                if let ast::Pattern::Bind(name) = &pattern.node {
                    self.insert_var(name.clone(), element_ty, false, pattern.span);
                }

                // Body value is discarded — implicit region.
                self.push_region(format!("for_body_{}", self.region_stack.len()));
                self.loop_body_region_depth.push(self.region_stack.len());
                let _body_ty = self.infer_block(body);
                self.loop_body_region_depth.pop();

                self.check_borrow_barrier("for loop exit", expr.span);
                self.pop_region();

                self.loop_depth -= 1;
                let break_ty = self.loop_break_types.pop().flatten();
                self.exit_scope();
                self.context_stack.pop();
                break_ty.unwrap_or(Type::Unit)
            }
            ast::Expr::While { condition, body } => {
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

                self.check_borrow_barrier("while loop exit", expr.span);
                self.pop_region();
                self.loop_depth -= 1;
                let break_ty = self.loop_break_types.pop().flatten();

                self.context_stack.pop();
                break_ty.unwrap_or(Type::Unit)
            }
            ast::Expr::Loop { body } => {
                self.context_stack.push("In loop".into());
                self.loop_depth += 1;
                self.loop_break_types.push(None);
                // Loop body is an implicit escape-barrier region (wiki §5):
                // no reference may leave via break.
                self.push_region(format!("loop_body_{}", self.region_stack.len()));
                self.loop_body_region_depth.push(self.region_stack.len());
                let _body_ty = self.infer_block(body);
                self.loop_body_region_depth.pop();

                self.check_borrow_barrier("loop exit", expr.span);
                self.pop_region();
                self.loop_depth -= 1;
                let break_ty = self.loop_break_types.pop().flatten();
                self.context_stack.pop();
                // loop {} without break yields Never; loop { break x } yields T
                break_ty.unwrap_or(Type::Never)
            }
            ast::Expr::Break(break_val) => {
                if self.loop_depth == 0 {
                    self.report_error("Break outside of loop".into(), expr.span);
                    return Type::Never;
                }
                if let Some(val) = break_val {
                    if let Some((root, span)) = self.check_break_escape(val) {
                        self.report_error(
                            format!("borrow of '{}' cannot escape the loop body; \
                                     it would dangle past loop exit",
                                root),
                            span,
                        );
                    }
                    let val_ty = self.infer_expr(val);
                    // Unify with the innermost loop's break type
                    let existing = self.loop_break_types.last().and_then(|s| s.clone());
                    match existing {
                        None => {
                            if let Some(slot) = self.loop_break_types.last_mut() {
                                *slot = Some(val_ty);
                            }
                        }
                        Some(ref ex_ty) => {
                            if !self.types_compatible(ex_ty, &val_ty) && val_ty != Type::Unknown {
                                self.report_error(
                                    format!("Break value type mismatch: expected {:?}, got {:?}", ex_ty, val_ty),
                                    expr.span,
                                );
                            }
                        }
                    }
                }
                Type::Never
            }
            ast::Expr::Continue => {
                if self.loop_depth == 0 {
                    self.report_error("Continue outside of loop".into(), expr.span);
                }
                Type::Never
            }
            ast::Expr::Return(ret_val) => {
                if let Some(val) = ret_val {
                    if let Some((root, span)) = self.check_return_escape(val) {
                        self.report_error(
                            format!("borrow of '{}' cannot escape via return; \
                                     it would dangle past loop / function exit",
                                root),
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
                        format!("borrow of '{}' cannot escape via throw; \
                                 it would dangle past loop / function exit",
                            root),
                        span,
                    );
                }
                let ex_ty = self.infer_expr(expr_val);
                self.add_required_effect(crate::ty::Effect::Exn(Box::new(ex_ty)));
                Type::Never
            }
            ast::Expr::Call { callee, args } => {
                self.context_stack.push(format!("In function call"));

                // `Shared(x)` must be inside a region
                if let ast::Expr::Identifier(name) = &callee.node {
                    if name == "Shared" && args.len() == 1 {
                        let inner_ty = self.infer_expr(&args[0]);
                        let region = self.region_stack.last().cloned();
                        if region.is_none() {
                            self.report_error(
                                "`Shared(...)` must be constructed inside a region expression".into(),
                                expr.span,
                            );
                        }
                        self.context_stack.pop();
                        return Type::Shared { inner: Box::new(inner_ty), region };
                    }
                }

                if let ast::Expr::FieldAccess { base, field } = &callee.node {
                    if let ast::Expr::Identifier(eff_name) = &base.node {
                        if self.effect_registry.contains_key(eff_name) {
                            let op_key = format!("{}::{}", eff_name, field);
                            let op_ty = self.effect_ops_registry.get(&op_key).cloned();
                            let (params, ret): (Vec<Type>, Type) = match op_ty {
                                Some(Type::Function { params, ret, .. }) => (params, *ret),
                                _ => (vec![], Type::Unknown),
                            };
                            if args.len() != params.len() {
                                self.report_error(
                                    format!(
                                        "Effect op '{}' expects {} argument(s), got {}",
                                        op_key, params.len(), args.len()
                                    ),
                                    expr.span,
                                );
                            }
                            for (i, arg) in args.iter().enumerate() {
                                self.context_stack.push(format!("Argument {}", i + 1));
                                let arg_ty = self.infer_expr(arg);
                                self.context_stack.pop();
                                if i < params.len()
                                    && arg_ty != params[i]
                                    && arg_ty != Type::Unknown
                                    && params[i] != Type::Unknown
                                {
                                    self.report_error(
                                        format!("Argument {} type mismatch: expected {:?}, got {:?}",
                                            i, params[i], arg_ty),
                                        arg.span,
                                    );
                                }
                            }
                            self.check_borrow_barrier(&op_key, expr.span);
                            self.add_required_effect(crate::ty::Effect::UserEffect(eff_name.clone()));
                            self.context_stack.pop();
                            return ret;
                        }
                    }
                }

                // Method-call dispatch
                if let ast::Expr::FieldAccess { base, field } = &callee.node {
                    self.context_stack.push(format!("In method call '.{}'", field));
                    let base_ty = self.infer_expr(base);
                    self.context_stack.pop();
                    let receiver_name = match &base_ty {
                        Type::Int => Some("Int".to_string()),
                        Type::Float => Some("Float".to_string()),
                        Type::Bool => Some("Bool".to_string()),
                        Type::Char => Some("Char".to_string()),
                        Type::String => Some("String".to_string()),
                        Type::Unit => Some("Unit".to_string()),
                        Type::Named(n) => Some(n.clone()),
                        Type::Reference { inner, .. } => match inner.as_ref() {
                            Type::Int => Some("Int".to_string()),
                            Type::Float => Some("Float".to_string()),
                            Type::Bool => Some("Bool".to_string()),
                            Type::Char => Some("Char".to_string()),
                            Type::String => Some("String".to_string()),
                            Type::Named(n) => Some(n.clone()),
                            _ => None,
                        },
                        _ => None,
                    };

                    if let Some(rname) = receiver_name {
                        let receiver_ty: Type = match rname.as_str() {
                            "Int"    => Type::Int,
                            "Float"  => Type::Float,
                            "Bool"   => Type::Bool,
                            "Char"   => Type::Char,
                            "String" => Type::String,
                            "Unit"   => Type::Unit,
                            _        => Type::Named(rname.clone()),
                        };
                        let sub_self = |t: &Type| -> Type {
                            match t {
                                Type::Named(n) if n == "Self" => receiver_ty.clone(),
                                Type::Reference { is_mut, inner } => {
                                    let inner_new = if let Type::Named(n) = inner.as_ref() {
                                        if n == "Self" { Type::Named(rname.clone()) }
                                        else { (**inner).clone() }
                                    } else { (**inner).clone() };
                                    Type::Reference { is_mut: *is_mut, inner: Box::new(inner_new) }
                                }
                                other => other.clone(),
                            }
                        };

                        // Bounded generic var: `x: T` where `T: Show` declares `field`.
                        let bound_match = self.get_trait_bounds(&rname).and_then(|bounds| {
                            bounds.iter().find_map(|trait_name| {
                                self.get_trait_method_sig(trait_name, field)
                                    .map(|sig| (trait_name.clone(), sig))
                            })
                        });
                        if let Some((_trait_name, (sig_params, sig_ret))) = bound_match {
                            let expected_args: Vec<Type> = sig_params.iter().skip(1).map(&sub_self).collect();
                            if args.len() != expected_args.len() {
                                self.report_error(
                                    format!("Method '{}.{}' expects {} argument(s), got {}",
                                        rname, field, expected_args.len(), args.len()),
                                    expr.span,
                                );
                            }
                            for (i, arg) in args.iter().enumerate() {
                                let arg_ty = self.infer_expr(arg);
                                if i < expected_args.len()
                                    && arg_ty != expected_args[i]
                                    && arg_ty != Type::Unknown
                                    && expected_args[i] != Type::Unknown
                                {
                                    self.report_error(
                                        format!("Argument {} type mismatch in method '{}.{}': expected {:?}, got {:?}",
                                            i, rname, field, expected_args[i], arg_ty),
                                        arg.span,
                                    );
                                }
                            }
                            self.context_stack.pop();
                            return sub_self(&sig_ret);
                        }

                        // Concrete `impl Trait for <Type>` for the receiver's type.
                        match self.resolve_method_on_type(&rname, field) {
                            Ok(Some((trait_name, _mangled))) => {
                                let (sig_params, sig_ret) = self
                                    .get_trait_method_sig(&trait_name, field)
                                    .unwrap_or((vec![], Type::Unknown));
                                let expected_args: Vec<Type> = sig_params.iter().skip(1).map(&sub_self).collect();
                                if args.len() != expected_args.len() {
                                    self.report_error(
                                        format!("Method '{}.{}' expects {} argument(s), got {}",
                                            rname, field, expected_args.len(), args.len()),
                                        expr.span,
                                    );
                                }
                                for (i, arg) in args.iter().enumerate() {
                                    let arg_ty = self.infer_expr(arg);
                                    if i < expected_args.len()
                                        && arg_ty != expected_args[i]
                                        && arg_ty != Type::Unknown
                                        && expected_args[i] != Type::Unknown
                                    {
                                        self.report_error(
                                            format!("Argument {} type mismatch in method '{}.{}': expected {:?}, got {:?}",
                                                i, rname, field, expected_args[i], arg_ty),
                                            arg.span,
                                        );
                                    }
                                }
                                self.context_stack.pop();
                                return sub_self(&sig_ret);
                            }
                            Ok(None) => {
                                let is_record_with_field = matches!(
                                    self.type_registry.get(&rname),
                                    Some(ast::TypeBody::Record(fs)) if fs.iter().any(|f| &f.name == field)
                                );
                                if !is_record_with_field {
                                    self.report_error(
                                        format!("No method '{}' for type '{}'", field, rname),
                                        expr.span,
                                    );
                                    self.context_stack.pop();
                                    return Type::Unknown;
                                }
                            }
                            Err(traits) => {
                                self.report_error(
                                    format!("Ambiguous method call '{}.{}': implemented by traits {:?}",
                                        rname, field, traits),
                                    expr.span,
                                );
                                self.context_stack.pop();
                                return Type::Unknown;
                            }
                        }
                    }
                }

                let callee_generic_vars: Vec<String> = if let ast::Expr::Identifier(n) = &callee.node {
                    self.get_generic_params(n).unwrap_or_default()
                } else {
                    Vec::new()
                };
                // Overload resolution: if the callee is an identifier with a
                // registered overload set, pick the matching signature based on
                // arg types. Falls back to scope lookup if nothing matches.
                let overload_hit: Option<Type> = if let ast::Expr::Identifier(name) = &callee.node {
                    self.resolve_call_overload(name, args)
                } else {
                    None
                };
                let callee_ty = overload_hit.unwrap_or_else(|| self.infer_expr(callee));
                let result = if let Type::Function { params, effects, ret } = callee_ty {
                    if args.len() != params.len() {
                        self.report_error(
                            format!("Expected {} arguments, got {}", params.len(), args.len()),
                            expr.span
                        );
                    }

                    let mut arg_types = Vec::new();
                    for (i, arg) in args.iter().enumerate() {
                        self.context_stack.push(format!("Argument {}", i + 1));
                        let arg_ty = self.infer_expr(arg);
                        self.context_stack.pop();
                        arg_types.push(arg_ty);
                    }

                    let subst = self.build_substitution_map(&params, &arg_types);
                    for (i, (arg_ty, param_ty)) in arg_types.iter().zip(params.iter()).enumerate() {
                        // Skip strict type checking if parameter is a generic type variable
                        // (either Type::Generic, or Type::Named(n) where n is a generic param of the callee).
                        let is_param_generic = matches!(param_ty, Type::Generic { .. })
                            || matches!(param_ty, Type::Named(n) if callee_generic_vars.contains(n));
                        if !is_param_generic && arg_ty != param_ty && *arg_ty != Type::Unknown && *param_ty != Type::Unknown {
                            self.report_error(
                                format!("Argument {} type mismatch: expected {:?}, got {:?}", i, param_ty, arg_ty),
                                args[i].span
                            );
                        }
                    }

                    // Borrow barrier: effect calls are suspension points, reject live outer-region borrows.
                    if !effects.is_empty() {
                        let op_name = match &callee.node {
                            ast::Expr::Identifier(n) => n.clone(),
                            ast::Expr::FieldAccess { field, .. } => field.clone(),
                            _ => "<call>".into(),
                        };
                        self.check_borrow_barrier(&op_name, expr.span);
                    }

                    for effect in &effects {
                        self.add_required_effect(effect.clone());
                    }
                    // For `Type::Named(n)` parameters that are generic vars, also build
                    // a name-keyed substitution so the return type can be specialised.
                    let mut named_subst: std::collections::HashMap<String, Type> =
                        std::collections::HashMap::new();
                    fn collect_named(
                        param: &Type, arg: &Type,
                        gens: &[String],
                        subst: &mut std::collections::HashMap<String, Type>,
                    ) {
                        match (param, arg) {
                            (Type::Named(n), a) if gens.contains(n) => {
                                subst.entry(n.clone()).or_insert_with(|| a.clone());
                            }
                            (Type::Generic { name: pn, args: pa },
                             Type::Generic { name: an, args: aa })
                                if pn == an && pa.len() == aa.len() => {
                                for (p, a) in pa.iter().zip(aa.iter()) {
                                    collect_named(p, a, gens, subst);
                                }
                            }
                            (Type::Tuple(ps), Type::Tuple(as_)) if ps.len() == as_.len() => {
                                for (p, a) in ps.iter().zip(as_.iter()) {
                                    collect_named(p, a, gens, subst);
                                }
                            }
                            (Type::Reference { inner: pi, .. },
                             Type::Reference { inner: ai, .. }) => {
                                collect_named(pi, ai, gens, subst);
                            }
                            _ => {}
                        }
                    }
                    for (p, a) in params.iter().zip(arg_types.iter()) {
                        collect_named(p, a, &callee_generic_vars, &mut named_subst);
                    }
                    // Reject the call here (before the compiler runs) if any
                    // declared type parameter of the callee can't be inferred
                    if !callee_generic_vars.is_empty() {
                        let callee_name = if let ast::Expr::Identifier(n) = &callee.node {
                            n.clone()
                        } else {
                            "<call>".into()
                        };
                        for g in &callee_generic_vars {
                            if !named_subst.contains_key(g)
                                && !subst.contains_key(g)
                            {
                                self.report_error(
                                    format!("Cannot infer type parameter '{}' for call to '{}'",
                                        g, callee_name),
                                    expr.span,
                                );
                            }
                        }
                        // Verify any where-clause bounds declared on the callee
                        // are satisfied by the inferred concrete type args.
                        let type_args: Vec<(String, Type)> = callee_generic_vars.iter()
                            .filter_map(|g| named_subst.get(g)
                                .or_else(|| subst.get(g))
                                .map(|t| (g.clone(), t.clone())))
                            .collect();
                        if !self.check_all_trait_bounds(&callee_name, &type_args) {
                            for (param, arg) in &type_args {
                                if let Some(bounds) = self.get_trait_bounds(param) {
                                    for trait_name in &bounds {
                                        let ty_str = match arg {
                                            Type::Int    => "Int".to_string(),
                                            Type::Float  => "Float".to_string(),
                                            Type::Bool   => "Bool".to_string(),
                                            Type::Char   => "Char".to_string(),
                                            Type::String => "String".to_string(),
                                            Type::Unit   => "Unit".to_string(),
                                            Type::Named(n) => n.clone(),
                                            _ => format!("{:?}", arg),
                                        };
                                        if !self.has_impl(&ty_str, trait_name) {
                                            self.report_error(
                                                format!("Type '{}' does not satisfy bound '{}: {}' \
                                                         required by call to '{}'",
                                                    ty_str, param, trait_name, callee_name),
                                                expr.span,
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                    fn subst_named(ty: &Type, subst: &std::collections::HashMap<String, Type>) -> Type {
                        match ty {
                            Type::Named(n) => subst.get(n).cloned().unwrap_or_else(|| ty.clone()),
                            Type::Generic { name, args } => Type::Generic {
                                name: name.clone(),
                                args: args.iter().map(|a| subst_named(a, subst)).collect(),
                            },
                            Type::Shared { inner, region } => Type::Shared {
                                inner: Box::new(subst_named(inner, subst)),
                                region: region.clone(),
                            },
                            Type::Tuple(ts) => Type::Tuple(
                                ts.iter().map(|t| subst_named(t, subst)).collect()),
                            Type::Reference { is_mut, inner } => Type::Reference {
                                is_mut: *is_mut,
                                inner: Box::new(subst_named(inner, subst)),
                            },
                            Type::Function { params, effects, ret } => Type::Function {
                                params: params.iter().map(|p| subst_named(p, subst)).collect(),
                                effects: effects.clone(),
                                ret: Box::new(subst_named(ret, subst)),
                            },
                            _ => ty.clone(),
                        }
                    }
                    let ret_after_generic_subst = self.apply_substitution(&ret, &subst);
                    subst_named(&ret_after_generic_subst, &named_subst)

                } else {
                    self.report_error("Callee must be function type".into(), callee.span)
                };

                self.context_stack.pop();
                result
            }
            ast::Expr::Tuple(elems) => {
                self.context_stack.push("In tuple construction".into());
                let elem_types: Vec<_> = elems.iter().map(|e| self.infer_expr(e)).collect();
                self.context_stack.pop();
                Type::Tuple(elem_types)
            }
            ast::Expr::Array(elems) => {
                self.context_stack.push("In array construction".into());
                let result = if elems.is_empty() {
                    Type::Generic { name: "Array".into(), args: vec![Type::Unknown] }
                } else {
                    let first_ty = self.infer_expr(&elems[0]);
                    for elem in &elems[1..] {
                        let elem_ty = self.infer_expr(elem);
                        if elem_ty != first_ty && elem_ty != Type::Unknown && first_ty != Type::Unknown {
                            self.report_error("Array elements must have same type".into(), elem.span);
                        }
                    }
                    Type::Generic { name: "Array".into(), args: vec![first_ty] }
                };
                self.context_stack.pop();
                result
            }
            ast::Expr::ArrayRepeat { elem, count } => {
                self.context_stack.push("In array repeat".into());
                let elem_ty = self.infer_expr(elem);
                let count_ty = self.infer_expr(count);
                if count_ty != Type::Int && count_ty != Type::Unknown {
                    self.report_error("Array repeat count must be Int".into(), count.span);
                }
                self.context_stack.pop();
                Type::Generic { name: "Array".into(), args: vec![elem_ty] }
            }
            ast::Expr::Index { base, index } => {
                self.context_stack.push("In array indexing".into());
                let base_ty = self.infer_expr(base);
                let index_ty = self.infer_expr(index);

                if index_ty != Type::Int && index_ty != Type::Unknown {
                    self.report_error("Index must be Int".into(), index.span);
                }

                let result = match base_ty {
                    Type::Generic { ref name, ref args } if name == "Array" => {
                        args.get(0).cloned().unwrap_or(Type::Unknown)
                    }
                    Type::Tuple(ref elems) => {
                        if elems.is_empty() { Type::Unknown }
                        else { elems[0].clone() }
                    }
                    Type::Unknown => Type::Unknown,
                    _ => self.report_error("Can only index arrays or tuples".into(), base.span),
                };

                self.context_stack.pop();
                result
            }
            ast::Expr::FieldAccess { base, field } => {
                if let ast::Expr::Identifier(base_name) = &base.node {
                    if let Some(cases) = self.variant_registry.get(base_name) {
                        if cases.iter().any(|c| c == field) {
                            return Type::Named(base_name.clone());
                        }
                    }
                }
                self.context_stack.push(format!("In field access '{}'", field));
                // Field access borrows base; `p.x + p.y` doesn't trip move checker.
                let base_ty = if let ast::Expr::Identifier(name) = &base.node {
                    self.get_var(name, true, base.span)
                } else {
                    self.infer_expr(base)
                };
                let field_type = self.resolve_field_access(&base_ty, field, base.span);

                self.context_stack.pop();
                field_type
            }
            ast::Expr::Closure { is_move: _, params, effects, return_type, body } => {
                self.context_stack.push("In closure expression".into());

                // closure captures must be Move/Copy.
                {
                    use std::collections::HashSet;
                    let mut bound: HashSet<String> = HashSet::new();
                    for p in params {
                        if let ast::Pattern::Bind(n) = &p.pattern.node {
                            bound.insert(n.clone());
                        }
                    }
                    let mut seen = HashSet::new();
                    let mut frees = Vec::new();
                    crate::compiler::closures::collect_free_vars(body, &bound, &mut seen, &mut frees);
                    for name in &frees {
                        if let Some(ty) = self.peek_var(name) {
                            if matches!(ty, Type::Reference { .. }) {
                                self.report_error(
                                    format!(
                                        "closure cannot capture reference '{}'", name
                                    ),
                                    body.span,
                                );
                            }
                            if self.type_contains_shared(&ty) {
                                self.report_error(
                                    format!(
                                        "closure cannot capture Shared binding '{}' \
                                         from an enclosing region", name
                                    ),
                                    body.span,
                                );
                            }
                            if let Some(true) = self.peek_var_is_mut(name) {
                                self.report_error(
                                    format!(
                                        "mutable binding '{}' cannot be captured by a closure; \
                                         rebind as immutable, or model mutation through the `state` effect",
                                        name
                                    ),
                                    body.span,
                                );
                            }
                        }
                    }
                }

                self.enter_scope();

                // Set declared effects for the closure
                let declared_effects = self.convert_effect_items(effects);
                let saved_required = std::mem::take(&mut self.fn_required_effects);

                self.fn_declared_effects = declared_effects.clone();

                for param in params {
                    let converted_ty = param.ty.as_ref()
                        .map(|t| self.convert_type(t))
                        .unwrap_or(Type::Unknown);
                    if let ast::Pattern::Bind(n) = &param.pattern.node {
                        self.insert_var(n.clone(), converted_ty, false, param.pattern.span);
                    }
                }
                let body_ty = self.infer_expr(body);
                if let Some(expected_ret) = return_type {
                    let expected = self.convert_type(expected_ret);
                    if expected != body_ty && expected != Type::Unknown && body_ty != Type::Unknown {
                        self.report_error("Closure body type mismatch".into(), body.span);
                    }
                }

                // Validate closure effect declarations
                // If effects are declared, check that inferred effects match declaration
                if !declared_effects.is_empty() {
                    let inferred = self.fn_required_effects.clone();
                    let exceeds = self.inferred_effects_exceed_declared(&declared_effects, &inferred);
                    if !exceeds.is_empty() {
                        self.report_error(
                            format!("Closure body produces effects not in declared effect set"),
                            body.span
                        );
                    }
                }

                // Infer closure effects and clear context
                let inferred_effects = self.infer_closure_effects(&self.fn_required_effects);
                self.fn_declared_effects.clear();
                self.fn_required_effects = saved_required;

                self.exit_scope();

                // Return function type with inferred effects; missing annotations become Unknown.
                let param_tys: Vec<Type> = params.iter()
                    .map(|p| p.ty.as_ref().map(|t| self.convert_type(t)).unwrap_or(Type::Unknown))
                    .collect();
                let result = Type::Function {
                    params: param_tys,
                    effects: inferred_effects,
                    ret: Box::new(body_ty),
                };

                self.context_stack.pop();
                result
            }
            ast::Expr::Record { ty, fields } => {
                let type_name = ty.join(".");
                self.context_stack.push(format!("In record construction of '{}'", type_name));
                let declared_opt = self.type_registry.get(&type_name).cloned();
                let mut declared_tys: std::collections::HashMap<String, Type> = std::collections::HashMap::new();
                if let Some(ast::TypeBody::Record(declared)) = &declared_opt {
                    for f in declared {
                        declared_tys.insert(f.name.clone(), self.convert_type(&f.ty));
                    }
                }
                for field in fields {
                    if let Some(value) = &field.value {
                        let v_ty = self.infer_expr(value);
                        if let Some(expected) = declared_tys.get(&field.name) {
                            if !types_assignable(expected, &v_ty) {
                                self.report_error(
                                    format!("Record '{}' field '{}': expected {:?}, got {:?}",
                                            type_name, field.name, expected, v_ty),
                                    value.span,
                                );
                            }
                        }
                    }
                }
                if let Some(ast::TypeBody::Record(declared)) = &declared_opt {
                    let known: Vec<String> = declared.iter().map(|f| f.name.clone()).collect();
                    for field in fields {
                        if !known.iter().any(|n| n == &field.name) {
                            self.report_error(
                                format!("Record '{}' has no field '{}'; known fields: {:?}",
                                        type_name, field.name, known),
                                expr.span,
                            );
                        }
                    }
                    for declared_name in &known {
                        if !fields.iter().any(|f| &f.name == declared_name) {
                            self.report_error(
                                format!("Record '{}' missing required field '{}'", type_name, declared_name),
                                expr.span,
                            );
                        }
                    }
                }
                self.context_stack.pop();
                Type::Named(type_name)
            }
            ast::Expr::Variant { ty, args } => {
                let case_name = ty.last().cloned().unwrap_or_default();
                self.context_stack.push(format!("In variant construction of '{}'", ty.join(".")));
                let payload_tys: Vec<Type> = match self.lookup_variant_constructor(&case_name) {
                    Some(Type::Function { params, .. }) => params,
                    _ => Vec::new(),
                };
                for (i, arg) in args.iter().enumerate() {
                    let arg_ty = self.infer_expr(arg);
                    if let Some(expected) = payload_tys.get(i) {
                        if !types_assignable(expected, &arg_ty) {
                            self.report_error(
                                format!("Variant '{}' payload {}: expected {:?}, got {:?}",
                                        case_name, i, expected, arg_ty),
                                arg.span,
                            );
                        }
                    }
                }
                self.context_stack.pop();
                Type::Named(ty.join("."))
            }
            ast::Expr::Range { start, end, inclusive: _ } => {
                if let Some(s) = start {
                    let s_ty = self.infer_expr(s);
                    if s_ty != Type::Int && s_ty != Type::Unknown {
                        self.report_error("Range start must be Int".into(), s.span);
                    }
                }
                if let Some(e) = end {
                    let e_ty = self.infer_expr(e);
                    if e_ty != Type::Int && e_ty != Type::Unknown {
                        self.report_error("Range end must be Int".into(), e.span);
                    }
                }
                Type::Named("Range<Int>".into())
            }
            ast::Expr::Question(inner) => {
                let inner_ty = self.infer_expr(inner);
                let in_exn_fn = self.fn_declared_effects.iter()
                    .any(|e| matches!(e, crate::ty::Effect::Exn(_)));
                match &inner_ty {
                    Type::Generic { name, args } if name == "Result" => {
                        let ok_ty = args.first().cloned().unwrap_or(Type::Unknown);
                        let err_ty = args.get(1).cloned().unwrap_or(Type::Unknown);
                        self.add_required_effect(crate::ty::Effect::Exn(Box::new(err_ty)));
                        ok_ty
                    }
                    Type::Generic { name, args } if name == "Option" => {
                        let inner_t = args.first().cloned().unwrap_or(Type::Unknown);
                        self.add_required_effect(crate::ty::Effect::Exn(
                            Box::new(Type::Named("NoneError".into()))
                        ));
                        inner_t
                    }
                    Type::Unknown => Type::Unknown,
                    _ if in_exn_fn => inner_ty.clone(),
                    _ => {
                        self.report_error(
                            format!("'?' operator requires Result<T,E> or Option<T>, got {:?}", inner_ty),
                            inner.span,
                        );
                        Type::Unknown
                    }
                }
            }
            ast::Expr::Resume(arg) => {
                // resume(...) must occur inside a non-return handler arm body.
                if !self.in_handler_arm {
                    self.report_error(
                        "'resume' is only valid inside a handler arm body".into(),
                        expr.span,
                    );
                }
                if let Some(a) = arg { let _ = self.infer_expr(a); }
                Type::Never
            }
            ast::Expr::Region { label, body } => {
                self.context_stack.push(format!("In region{}",
                    label.as_ref().map(|l| format!(" '{}'", l)).unwrap_or_default()));

                let region_name = label.as_ref()
                    .map(|l| l.clone())
                    .unwrap_or_else(|| format!("region_{}", self.region_stack.len()));
                self.push_region(region_name.clone());

                self.effect_stack.push(self.active_effects.clone());
                let body_ty = self.infer_block(body);
                self.effect_stack.pop();

                // Enforce borrow barrier at region exit: no outer-region references should be live
                self.check_borrow_barrier("region exit", expr.span);

                self.pop_region();
                self.context_stack.pop();

                // A region's result type must not contain a Shared cell
                if self.type_contains_shared(&body_ty) {
                    self.report_error(
                        format!(
                            "region '{}' result type {:?} contains `Shared<T>` — \
                             a Shared cell cannot escape its enclosing region",
                            region_name, body_ty
                        ),
                        expr.span,
                    );
                }

                body_ty
            }
            ast::Expr::Handle { expr: handler_expr, arms } => {
                self.context_stack.push("In handle expression".into());
                let saved_handled = std::mem::take(&mut self.handled_effects);

                let required_before = self.fn_required_effects.clone();
                let _expr_ty = self.infer_expr(handler_expr);
                let required_from_inner: Vec<_> = self.fn_required_effects.iter()
                    .filter(|e| !required_before.iter().any(|b| self.effects_equal(b, e)))
                    .cloned()
                    .collect();

                let mut seen_effect: Option<String> = None;
                for arm in arms.iter() {
                    if let ast::HandleArmKind::Effect(path) = &arm.kind {
                        if path.len() >= 2 {
                            let eff = path[..path.len() - 1].join(".");
                            match &seen_effect {
                                None => seen_effect = Some(eff),
                                Some(prev) if prev == &eff => {}
                                Some(prev) => {
                                    self.report_error(
                                        format!("`handle` may only cover arms of a single effect; \
                                                 saw arms for both `{}` and `{}` (split into separate `handle` blocks)",
                                            prev, eff),
                                        expr.span,
                                    );
                                    break;
                                }
                            }
                        }
                    }
                }

                let mut arm_types = Vec::new();
                for (arm_idx, arm) in arms.iter().enumerate() {
                    // Validate arm pattern if present (introduces binder visible to body)
                    if let Some(pat) = &arm.pattern {
                        if let ast::Pattern::Bind(name) = &pat.node {
                            let pat_ty = if let ast::HandleArmKind::Effect(path) = &arm.kind {
                                if path.len() >= 2 {
                                    let op_key = format!("{}::{}", path[0], path[1]);
                                    match self.effect_ops_registry.get(&op_key).cloned() {
                                        Some(Type::Function { params, .. }) if params.len() == 1 => {
                                            params.into_iter().next().unwrap()
                                        }
                                        Some(Type::Function { params, .. }) if params.is_empty() => {
                                            Type::Unit
                                        }
                                        _ => Type::Unknown,
                                    }
                                } else {
                                    Type::Unknown
                                }
                            } else {
                                Type::Unknown
                            };
                            self.insert_var(name.clone(), pat_ty, false, pat.span);
                        }
                    }

                    // Non-return arm bodies are implicit regions and the body is a
                    // handler context where `resume` is valid.
                    let is_non_return = !matches!(arm.kind, ast::HandleArmKind::Return);
                    let saved_in_arm = self.in_handler_arm;
                    if is_non_return {
                        self.in_handler_arm = true;
                        let region_name = format!("handle_arm_{}", arm_idx);
                        self.push_region(region_name);
                    }

                    if let Some(nested_span) = Self::find_nested_handle(&arm.body) {
                        self.report_error(
                            "nested `handle` inside a handler arm body is not yet supported".into(),
                            nested_span,
                        );
                    }

                    let arm_ty = self.infer_expr(&arm.body);
                    arm_types.push(arm_ty);

                    if matches!(arm.kind, ast::HandleArmKind::Effect(_))
                        && !Self::arm_resumes_or_diverges(&arm.body)
                    {
                        self.report_error(
                            "effect handler arm must call `resume`/`return`/`throw` \
                             on every path; missing leaks the captured continuation"
                                .into(),
                            arm.body.span,
                        );
                    }

                    if is_non_return {
                        self.pop_region();
                        self.in_handler_arm = saved_in_arm;
                    }

                    match &arm.kind {
                        ast::HandleArmKind::Return => {
                            // Return handler doesn't remove an effect
                        }
                        ast::HandleArmKind::Exn => {
                            if !required_from_inner.iter().any(|e| matches!(e, crate::ty::Effect::Exn(_))) {
                                self.report_error(
                                    "Handling exn but inner expression produces no exn effect".into(),
                                    expr.span
                                );
                            }
                            self.mark_effect_handled("exn".into());
                        }
                        ast::HandleArmKind::Effect(effect_path) => {
                            if let Some(eff_name) = effect_path.first() {
                                self.mark_effect_handled(eff_name.clone());
                            }
                            self.mark_effect_handled(effect_path.join("."));
                        }
                    }
                }

                // Remove handled effects from fn_required_effects
                let required = self.fn_required_effects.clone();
                self.compute_unhandled_effects(&required);
                self.fn_required_effects = self.unhandled_effects.clone();

                let result_ty = arm_types.iter()
                    .find(|t| **t != Type::Never && **t != Type::Unknown)
                    .cloned()
                    .or_else(|| arm_types.first().cloned())
                    .unwrap_or(Type::Unknown);
                for ty in &arm_types {
                    if *ty != result_ty
                        && *ty != Type::Never
                        && *ty != Type::Unknown
                        && result_ty != Type::Unknown
                    {
                        self.report_error("Handle arm types do not match".into(), expr.span);
                    }
                }

                self.context_stack.pop();
                self.handled_effects = saved_handled;
                self.unhandled_effects.clear();
                result_ty
            }
        }
    }
    
    pub fn infer_block(&mut self, block: &ast::Block) -> Type {
        self.enter_scope();
        for stmt in &block.stmts {
            self.check_stmt(stmt);
        }
        let ty = if let Some(ret_expr) = &block.ret {
            self.infer_expr(ret_expr)
        } else {
            Type::Unit
        };
        self.exit_scope();
        ty
    }

    // Escape-barrier check: returns the (root, span) of a binding whose
    // borrow would dangle past `barrier_depth`. `break` uses the innermost
    // loop's depth; `return`/`throw` use the outermost loop's depth (they
    // unwind every enclosing loop on the way out of the function).
    //
    // The Ref cell itself is region_forgotten on break/return (see codegen),
    // so cells survive the pop. What matters is whether the bits stored in
    // the cell would dangle: a snapshot of a primitive is safe forever; a
    // snapshot of a heap handle is stale once the pointee's cell is force-
    // freed by the region pop. So we only reject when the borrowed root is
    // heap-typed AND inside the region barrier.
    fn check_escape_past(
        &self,
        val: &ast::Spanned<ast::Expr>,
        barrier_depth: usize,
    ) -> Option<(String, ast::Span)> {
        let root = match &val.node {
            ast::Expr::Unary { op: ast::UnaryOp::Ref, right }
            | ast::Expr::Unary { op: ast::UnaryOp::RefMut, right } => {
                Self::root_ident(right)?
            }
            ast::Expr::Identifier(n) => {
                let ty = self.resolve_var_in_scopes(n)?;
                if !matches!(ty, Type::Reference { .. }) { return None; }
                n.clone()
            }
            _ => return None,
        };
        for scope in self.scopes.iter().rev() {
            if let Some(meta) = scope.vars.get(&root) {
                if meta.bound_at_region_depth >= barrier_depth
                    && is_heap_typed(&meta.ty)
                {
                    return Some((root, val.span));
                }
                return None;
            }
        }
        None
    }

    fn check_break_escape(
        &self,
        val: &ast::Spanned<ast::Expr>,
    ) -> Option<(String, ast::Span)> {
        let loop_depth = *self.loop_body_region_depth.last()?;
        self.check_escape_past(val, loop_depth)
    }

    // Return/throw unwind every enclosing loop; the barrier is the outermost
    // loop body region currently in scope.
    fn check_return_escape(
        &self,
        val: &ast::Spanned<ast::Expr>,
    ) -> Option<(String, ast::Span)> {
        let outer_loop_depth = *self.loop_body_region_depth.first()?;
        self.check_escape_past(val, outer_loop_depth)
    }

    fn root_ident(expr: &ast::Spanned<ast::Expr>) -> Option<String> {
        match &expr.node {
            ast::Expr::Identifier(n) => Some(n.clone()),
            ast::Expr::FieldAccess { base, .. } => Self::root_ident(base),
            ast::Expr::Index { base, .. } => Self::root_ident(base),
            _ => None,
        }
    }
}

// True when a value of `ty` carries a heap reference internally — i.e.,
// when a Ref-snapshot of it would contain a handle that goes stale after
// region_pop force-frees the underlying cell. Primitives (Int/Float/Bool/
// Char/Unit/Never) hold their bits inline, so &them is safe to escape.
fn is_heap_typed(ty: &Type) -> bool {
    match ty {
        Type::Int | Type::Float | Type::Bool | Type::Char
        | Type::Unit | Type::Never => false,
        // Reference itself is an 8-byte handle, but the cell it points to
        // may live in the same region as the borrow — conservatively heap.
        _ => true,
    }
}

impl Checker {
    // ↑ helper above; reopen impl for the divergence-analysis methods below.

    // Returns true when evaluating `expr` definitely encounters a `resume`,
    // `return`, or `throw` on every control-flow path. Operand-evaluating
    // forms (Binary, Unary, Call, …) propagate divergence from any operand:
    // once an operand resumes/returns, surrounding operators never run, so
    // patterns like `v + resume(())` qualify. Branching forms (If, Match)
    // need every arm to diverge.
    fn arm_resumes_or_diverges(expr: &ast::Spanned<ast::Expr>) -> bool {
        match &expr.node {
            ast::Expr::Resume(_)
            | ast::Expr::Return(_)
            | ast::Expr::Throw(_) => true,
            ast::Expr::Block(b) => {
                let stmt_diverges = b.stmts.iter().any(|s| match &s.node {
                    ast::Stmt::Expr(e) => Self::arm_resumes_or_diverges(e),
                    ast::Stmt::Let { value, .. } => Self::arm_resumes_or_diverges(value),
                    ast::Stmt::Empty => false,
                });
                if stmt_diverges { return true; }
                match &b.ret {
                    Some(tail) => Self::arm_resumes_or_diverges(tail),
                    None => false,
                }
            }
            ast::Expr::If { condition, consequence, alternative: Some(alt) } => {
                Self::arm_resumes_or_diverges(condition)
                    || (Self::arm_resumes_or_diverges(consequence)
                        && Self::arm_resumes_or_diverges(alt))
            }
            ast::Expr::If { condition, alternative: None, .. } => {
                Self::arm_resumes_or_diverges(condition)
            }
            ast::Expr::Match { scrutinee, arms } => {
                if Self::arm_resumes_or_diverges(scrutinee) { return true; }
                !arms.is_empty()
                    && arms.iter().all(|a| Self::arm_resumes_or_diverges(&a.body))
            }
            ast::Expr::Binary { left, right, .. } => {
                Self::arm_resumes_or_diverges(left)
                    || Self::arm_resumes_or_diverges(right)
            }
            ast::Expr::Unary { right, .. } => Self::arm_resumes_or_diverges(right),
            ast::Expr::Call { callee, args } => {
                Self::arm_resumes_or_diverges(callee)
                    || args.iter().any(|a| Self::arm_resumes_or_diverges(a))
            }
            ast::Expr::Index { base, index } => {
                Self::arm_resumes_or_diverges(base)
                    || Self::arm_resumes_or_diverges(index)
            }
            ast::Expr::FieldAccess { base, .. } => Self::arm_resumes_or_diverges(base),
            ast::Expr::Tuple(items) | ast::Expr::Array(items) => {
                items.iter().any(|e| Self::arm_resumes_or_diverges(e))
            }
            _ => false,
        }
    }

    fn find_nested_handle(expr: &ast::Spanned<ast::Expr>) -> Option<ast::Span> {
        match &expr.node {
            ast::Expr::Handle { .. } => Some(expr.span),
            ast::Expr::Block(b) => {
                for stmt in &b.stmts {
                    let inner = match &stmt.node {
                        ast::Stmt::Expr(e) => Self::find_nested_handle(e),
                        ast::Stmt::Let { value, .. } => Self::find_nested_handle(value),
                        ast::Stmt::Empty => None,
                    };
                    if inner.is_some() { return inner; }
                }
                b.ret.as_deref().and_then(Self::find_nested_handle)
            }
            ast::Expr::If { condition, consequence, alternative } => {
                Self::find_nested_handle(condition)
                    .or_else(|| Self::find_nested_handle(consequence))
                    .or_else(|| alternative.as_deref().and_then(Self::find_nested_handle))
            }
            ast::Expr::Match { scrutinee, arms } => {
                Self::find_nested_handle(scrutinee)
                    .or_else(|| arms.iter().find_map(|a| Self::find_nested_handle(&a.body)))
            }
            ast::Expr::Binary { left, right, .. } => {
                Self::find_nested_handle(left).or_else(|| Self::find_nested_handle(right))
            }
            ast::Expr::Unary { right, .. } => Self::find_nested_handle(right),
            ast::Expr::Call { callee, args } => {
                Self::find_nested_handle(callee)
                    .or_else(|| args.iter().find_map(Self::find_nested_handle))
            }
            ast::Expr::Index { base, index } => {
                Self::find_nested_handle(base).or_else(|| Self::find_nested_handle(index))
            }
            ast::Expr::FieldAccess { base, .. } => Self::find_nested_handle(base),
            ast::Expr::Tuple(items) | ast::Expr::Array(items) => {
                items.iter().find_map(Self::find_nested_handle)
            }
            ast::Expr::ArrayRepeat { elem, count } => {
                Self::find_nested_handle(elem).or_else(|| Self::find_nested_handle(count))
            }
            ast::Expr::Variant { args, .. } => args.iter().find_map(Self::find_nested_handle),
            ast::Expr::Resume(Some(e)) => Self::find_nested_handle(e),
            ast::Expr::Return(Some(e)) | ast::Expr::Throw(e) | ast::Expr::Question(e) => {
                Self::find_nested_handle(e)
            }
            ast::Expr::Break(Some(e)) => Self::find_nested_handle(e),
            ast::Expr::While { condition, body } => {
                Self::find_nested_handle(condition).or_else(|| Self::find_nested_handle_block(body))
            }
            ast::Expr::For { iter, body, .. } => {
                Self::find_nested_handle(iter).or_else(|| Self::find_nested_handle_block(body))
            }
            ast::Expr::Loop { body } => Self::find_nested_handle_block(body),
            ast::Expr::Region { body, .. } => Self::find_nested_handle_block(body),
            ast::Expr::Closure { body, .. } => Self::find_nested_handle(body),
            ast::Expr::Range { start, end, .. } => {
                start.as_deref().and_then(Self::find_nested_handle)
                    .or_else(|| end.as_deref().and_then(Self::find_nested_handle))
            }
            _ => None,
        }
    }

    fn find_nested_handle_block(block: &ast::Block) -> Option<ast::Span> {
        for stmt in &block.stmts {
            let inner = match &stmt.node {
                ast::Stmt::Expr(e) => Self::find_nested_handle(e),
                ast::Stmt::Let { value, .. } => Self::find_nested_handle(value),
                ast::Stmt::Empty => None,
            };
            if inner.is_some() { return inner; }
        }
        block.ret.as_deref().and_then(Self::find_nested_handle)
    }
}
