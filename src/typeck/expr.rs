use crate::ast;
use crate::ast::Spanned;
use crate::ty::Type;
use super::*;

impl Checker {
    
    pub fn infer_expr(&mut self, expr: &Spanned<ast::Expr>) -> Type {
        match &expr.node {
            ast::Expr::Error => Type::Unknown,
            ast::Expr::Literal(lit) => match lit {
                ast::Literal::Int(_) => Type::Int,
                ast::Literal::Float(_) => Type::Float,
                ast::Literal::Bool(_) => Type::Bool,
                ast::Literal::Char(_) => Type::Char,
                ast::Literal::String(_) | ast::Literal::StringInterp(_) => Type::String,
                ast::Literal::Unit => Type::Unit,
            },
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
                        if let Type::Reference { inner, .. } = r_ty { *inner } else { self.report_error("Expected reference".into(), right.span) }
                    }
                };

                self.context_stack.pop();
                result
            }
            ast::Expr::Binary { op, left, right } => {
                self.context_stack.push("In binary expression".into());
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
                        | ast::BinaryOp::ModAssign => {
                            // Reject assignment to a non-`mut` binding.
                            // Only Identifier LHS is checked here; field/index
                            // assignment goes through a different lvalue path.
                            if let ast::Expr::Identifier(name) = &left.node {
                                let is_mut = self.scopes.iter().rev()
                                    .find_map(|s| s.vars.get(name).map(|m| m.is_mut));
                                if let Some(false) = is_mut {
                                    self.report_error(
                                        format!(
                                            "Cannot assign to immutable binding '{}'; \
                                             use `let mut {}` to allow mutation",
                                            name, name
                                        ),
                                        left.span,
                                    );
                                }
                            }
                            Type::Unit
                        }
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
                
                let cons_ty = self.infer_expr(consequence);
                if let Some(alt) = alternative {
                    let alt_ty = self.infer_expr(alt);
                    let compatible = cons_ty == alt_ty
                        || cons_ty == Type::Unknown || alt_ty == Type::Unknown
                        || cons_ty == Type::Never || alt_ty == Type::Never;
                    if !compatible {
                        self.report_error("If branch types do not match".into(), alt.span);
                    }
                    if cons_ty == Type::Never { return alt_ty; }
                }
                cons_ty
            }
            ast::Expr::Match { scrutinee, arms } => {
                self.context_stack.push("In match expression".into());
                let required_before = self.fn_required_effects.clone();
                let scrutinee_ty = self.infer_expr(scrutinee);
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
                for arm in arms {
                    if let Some(guard) = &arm.guard {
                        let guard_ty = self.infer_expr(guard);
                        if guard_ty != Type::Bool && guard_ty != Type::Unknown {
                            self.report_error("Guard must be Bool".into(), guard.span);
                        }
                    }
                    let body_ty = self.infer_expr(&arm.body);
                    arm_types.push(body_ty);
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

                let element_ty = self.extract_iterable_element_type(&iter_ty);
                if let ast::Pattern::Bind(name) = &pattern.node {
                    self.insert_var(name.clone(), element_ty, false, pattern.span);
                }

                let _body_ty = self.infer_block(body);

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
                let _body_ty = self.infer_block(body);
                self.loop_depth -= 1;
                let break_ty = self.loop_break_types.pop().flatten();

                self.context_stack.pop();
                break_ty.unwrap_or(Type::Unit)
            }
            ast::Expr::Loop { body } => {
                self.context_stack.push("In loop".into());
                self.loop_depth += 1;
                self.loop_break_types.push(None);
                let _body_ty = self.infer_block(body);
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
                    let _val_ty = self.infer_expr(val);
                }
                Type::Never
            }
            ast::Expr::Throw(expr_val) => {
                let ex_ty = self.infer_expr(expr_val);
                self.add_required_effect(crate::ty::Effect::Exn(Box::new(ex_ty)));
                Type::Never
            }
            ast::Expr::Call { callee, args } => {
                self.context_stack.push(format!("In function call"));

                if let ast::Expr::FieldAccess { base, field } = &callee.node {
                    if let ast::Expr::Identifier(eff_name) = &base.node {
                        if self.effect_registry.contains_key(eff_name) {
                            let op_key = format!("{}::{}", eff_name, field);
                            let op_ty = self.effect_ops_registry.get(&op_key).cloned();
                            let (params, ret): (Vec<Type>, Type) = match op_ty {
                                Some(Type::Function { params, ret, .. }) => (params, *ret),
                                _ => (vec![], Type::Unknown),
                            };
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
                            self.add_required_effect(crate::ty::Effect::Nondet);
                            self.context_stack.pop();
                            return ret;
                        }
                    }
                }

                let callee_generic_vars: Vec<String> = if let ast::Expr::Identifier(n) = &callee.node {
                    self.get_generic_params(n).unwrap_or_default()
                } else {
                    Vec::new()
                };
                let callee_ty = self.infer_expr(callee);
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

                    // Borrow barrier: if this call produces an effect, it is a
                    // potential suspension point. Reject if any borrow from an
                    // outer region is live in the calling frame.
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
                    fn subst_named(ty: &Type, subst: &std::collections::HashMap<String, Type>) -> Type {
                        match ty {
                            Type::Named(n) => subst.get(n).cloned().unwrap_or_else(|| ty.clone()),
                            Type::Generic { name, args } => Type::Generic {
                                name: name.clone(),
                                args: args.iter().map(|a| subst_named(a, subst)).collect(),
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
                    Type::Named("Array<Unknown>".into())
                } else {
                    let first_ty = self.infer_expr(&elems[0]);
                    for elem in &elems[1..] {
                        let elem_ty = self.infer_expr(elem);
                        if elem_ty != first_ty && elem_ty != Type::Unknown && first_ty != Type::Unknown {
                            self.report_error("Array elements must have same type".into(), elem.span);
                        }
                    }
                    Type::Named(format!("Array<{:?}>", first_ty))
                };
                self.context_stack.pop();
                result
            }
            ast::Expr::ArrayRepeat { elem, count } => {
                self.context_stack.push("In array repeat".into());
                let _elem_ty = self.infer_expr(elem);
                let count_ty = self.infer_expr(count);
                if count_ty != Type::Int && count_ty != Type::Unknown {
                    self.report_error("Array repeat count must be Int".into(), count.span);
                }
                self.context_stack.pop();
                Type::Named("Array<Unknown>".into())
            }
            ast::Expr::Index { base, index } => {
                self.context_stack.push("In array indexing".into());
                let base_ty = self.infer_expr(base);
                let index_ty = self.infer_expr(index);

                if index_ty != Type::Int && index_ty != Type::Unknown {
                    self.report_error("Index must be Int".into(), index.span);
                }

                let result = match base_ty {
                    Type::Named(ref name) if name.starts_with("Array") => Type::Unknown,
                    Type::Tuple(ref elems) => {
                        if elems.is_empty() { Type::Unknown }
                        else { elems[0].clone() }
                    }
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
                let base_ty = self.infer_expr(base);
                let field_type = self.resolve_field_access(&base_ty, field, base.span);

                self.context_stack.pop();
                field_type
            }
            // Advanced Expressions (updated Effect Inference)
            ast::Expr::Closure { is_move: _, params, effects, return_type, body } => {
                self.context_stack.push("In closure expression".into());
                self.enter_scope();

                // Set declared effects for the closure
                let declared_effects = self.convert_effect_items(effects);
                let saved_required = std::mem::take(&mut self.fn_required_effects);

                self.fn_declared_effects = declared_effects.clone();

                for param in params {
                    if let Some(param_ty) = &param.ty {
                        let converted_ty = self.convert_type(param_ty);
                        self.insert_var(
                            match &param.pattern.node {
                                ast::Pattern::Bind(n) => n.clone(),
                                _ => "<param>".into(),
                            },
                            converted_ty,
                            false,
                            param.pattern.span
                        );
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

                // Return function type with inferred effects
                let result = Type::Function {
                    params: vec![],
                    effects: inferred_effects,
                    ret: Box::new(body_ty),
                };

                self.context_stack.pop();
                result
            }
            ast::Expr::Record { ty, fields } => {
                self.context_stack.push(format!("In record construction of '{}'", ty.join(".")));
                for field in fields {
                    if let Some(value) = &field.value {
                        let _field_ty = self.infer_expr(value);
                    }
                }
                self.context_stack.pop();
                Type::Named(ty.join("."))
            }
            ast::Expr::Variant { ty, args } => {
                self.context_stack.push(format!("In variant construction of '{}'", ty.join(".")));
                for arg in args {
                    let _arg_ty = self.infer_expr(arg);
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

                // Push region for escape analysis
                let region_name = label.as_ref()
                    .map(|l| l.clone())
                    .unwrap_or_else(|| format!("region_{}", self.region_stack.len()));
                self.push_region(region_name.clone());

                // Push new region effect context
                self.effect_stack.push(self.active_effects.clone());
                let body_ty = self.infer_block(body);
                self.effect_stack.pop();

                // Pop region and validate no escapes
                self.pop_region();
                self.context_stack.pop();

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

                let mut arm_types = Vec::new();
                for (arm_idx, arm) in arms.iter().enumerate() {
                    // Validate arm pattern if present (introduces binder visible to body)
                    if let Some(pat) = &arm.pattern {
                        if let ast::Pattern::Bind(name) = &pat.node {
                            self.insert_var(name.clone(), Type::Unknown, false, pat.span);
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

                    let arm_ty = self.infer_expr(&arm.body);
                    arm_types.push(arm_ty);

                    if is_non_return {
                        self.pop_region();
                        self.in_handler_arm = saved_in_arm;
                    }

                    // Register handled effect based on kind
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
                            let effect_name = effect_path.join(".");
                            self.mark_effect_handled(effect_name);
                            if let Some(head) = effect_path.first() {
                                if self.effect_registry.contains_key(head) {
                                    self.mark_effect_handled("nondet".into());
                                }
                            }
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
}
