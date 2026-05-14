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
                    if cons_ty != alt_ty && cons_ty != Type::Unknown && alt_ty != Type::Unknown {
                        self.report_error("If branch types do not match".into(), alt.span);
                    }
                }
                cons_ty
            }
            ast::Expr::Match { scrutinee, arms } => {
                self.context_stack.push("In match expression".into());
                let scrutinee_ty = self.infer_expr(scrutinee);

                // Pattern type checking and exhaustiveness analysis
                for arm in arms {
                    self.check_pattern(&arm.pattern, &scrutinee_ty, arm.pattern.span);
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
                        let is_param_generic = matches!(param_ty, Type::Generic { .. });
                        if !is_param_generic && arg_ty != param_ty && *arg_ty != Type::Unknown && *param_ty != Type::Unknown {
                            self.report_error(
                                format!("Argument {} type mismatch: expected {:?}, got {:?}", i, param_ty, arg_ty),
                                args[i].span
                            );
                        }
                    }

                    for effect in &effects {
                        self.add_required_effect(effect.clone());
                    }
                    self.apply_substitution(&ret, &subst)

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
                match &inner_ty {
                    Type::Generic { name, args } if name == "Result" => {
                        // Result<T, E>? propagates exn<E> and unwraps T
                        let ok_ty = args.first().cloned().unwrap_or(Type::Unknown);
                        let err_ty = args.get(1).cloned().unwrap_or(Type::Unknown);
                        self.add_required_effect(crate::ty::Effect::Exn(Box::new(err_ty)));
                        ok_ty
                    }
                    Type::Generic { name, args } if name == "Option" => {
                        // Option<T>? propagates exn and unwraps T
                        let inner_t = args.first().cloned().unwrap_or(Type::Unknown);
                        self.add_required_effect(crate::ty::Effect::Exn(
                            Box::new(Type::Named("NoneError".into()))
                        ));
                        inner_t
                    }
                    Type::Unknown => Type::Unknown,
                    _ => {
                        self.report_error(
                            format!("'?' operator requires Result<T,E> or Option<T>, got {:?}", inner_ty),
                            inner.span,
                        );
                        Type::Unknown
                    }
                }
            }
            ast::Expr::Await(inner) => {
                let inner_ty = self.infer_expr(inner);
                match &inner_ty {
                    Type::Generic { name, args } if name == "Future" => {
                        let output_ty = args.first().cloned().unwrap_or(Type::Unknown);
                        self.add_required_effect(crate::ty::Effect::Async);
                        output_ty
                    }
                    Type::Unknown => Type::Unknown,
                    _ => {
                        self.report_error(
                            format!("'.await' requires Future<T>, got {:?}", inner_ty),
                            inner.span,
                        );
                        Type::Unknown
                    }
                }
            }
            ast::Expr::Scope { label, options, body } => {
                self.context_stack.push(format!("In scope{}",
                    label.as_ref().map(|l| format!(" '{}'", l)).unwrap_or_default()));

                // Validate scope with context expression
                if let Some(opts) = options {
                    let opts_ty = self.infer_expr(opts);
                    if !self.validate_scope_with_context(&opts_ty) {
                        self.report_error("Scope 'with' expression must provide valid context".into(), opts.span);
                    }
                }

                // Push new scope effect context
                self.effect_stack.push(self.active_effects.clone());
                let body_ty = self.infer_block(body);
                self.effect_stack.pop();

                self.context_stack.pop();
                body_ty
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
                self.handled_effects.clear();

                let _expr_ty = self.infer_expr(handler_expr);

                let mut arm_types = Vec::new();
                for arm in arms {
                    // Type-check the arm body
                    let arm_ty = self.infer_expr(&arm.body);
                    arm_types.push(arm_ty);

                    // Validate arm pattern if present
                    if let Some(pat) = &arm.pattern {
                        match &pat.node {
                            ast::Pattern::Bind(name) => {
                                // For parameterized exceptions, bind with correct type
                                let var_ty = match &arm.kind {
                                    ast::HandleArmKind::Exn => Type::Unknown,
                                    _ => Type::Unknown,
                                };
                                self.insert_var(name.clone(), var_ty, false, pat.span);
                            }
                            _ => {}
                        }
                    }

                    // Register handled effect based on kind
                    match &arm.kind {
                        ast::HandleArmKind::Return => {
                            // Return handler doesn't remove an effect
                        }
                        ast::HandleArmKind::Exn => {
                            // Exception handler - mark exn as handled
                            if !self.active_effects.contains(&"exn".to_string()) {
                                self.report_error(
                                    "Handling exn but no exn effect is active".into(),
                                    expr.span
                                );
                            }
                            self.mark_effect_handled("exn".into());
                        }
                        ast::HandleArmKind::Effect(effect_path) => {
                            let effect_name = effect_path.join(".");
                            if !self.active_effects.contains(&effect_name) {
                                self.report_error(
                                    format!("Handling effect {} but it is not active", effect_name),
                                    expr.span
                                );
                            }
                            self.mark_effect_handled(effect_name);
                        }
                    }
                }

                // Compute unhandled effects and propagate them
                let required_effects = self.fn_required_effects.clone();
                self.compute_unhandled_effects(&required_effects);
                self.propagate_effects_to_parent();

                // All arms must have same type
                if !arm_types.is_empty() {
                    let first = arm_types[0].clone();
                    for ty in arm_types.iter().skip(1) {
                        if *ty != first && first != Type::Unknown && *ty != Type::Unknown {
                            self.report_error("Handle arm types do not match".into(), expr.span);
                        }
                    }
                }

                self.context_stack.pop();
                self.clear_handle_context();
                if arm_types.is_empty() { Type::Unknown } else { arm_types[0].clone() }
            }
            _ => self.report_error("Expression not supported yet".into(), expr.span),
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
