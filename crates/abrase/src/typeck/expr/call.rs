use crate::ast;
use crate::ast::Spanned;
use crate::ty::Type;
use super::super::*;

impl Checker {
    pub(super) fn infer_call(
        &mut self,
        callee: &Spanned<ast::Expr>,
        args: &[Spanned<ast::Expr>],
        span: ast::Span,
    ) -> Type {
                self.context_stack.push(format!("In function call"));

                // `Shared(x)` must be inside a region
                if let ast::Expr::Identifier(name) = &callee.node {
                    if name == "Shared" && args.len() == 1 {
                        let inner_ty = self.infer_expr(&args[0]);
                        let region = self.region_stack.last().cloned();
                        if region.is_none() {
                            self.report_error(
                                "`Shared(...)` must be constructed inside a region expression".into(),
                                span,
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
                                    span,
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
                            self.check_borrow_barrier(&op_key, span);
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
                    if field == "clone" && args.is_empty() {
                        return base_ty;
                    }
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
                                    span,
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
                                        span,
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
                                let field_ty = match self.type_registry.get(&rname) {
                                    Some(ast::TypeBody::Record(fs)) =>
                                        fs.iter().find(|f| &f.name == field).map(|f| f.ty.clone()),
                                    _ => None,
                                };
                                match field_ty {
                                    None => {
                                        self.report_error(
                                            format!("No method '{}' for type '{}'", field, rname),
                                            span,
                                        );
                                        self.context_stack.pop();
                                        return Type::Unknown;
                                    }
                                    Some(ft) => {
                                        // Record field holding a function value: `b.f(args)`
                                        // calls the field. Check args against its fn type.
                                        if let ast::Type::Function { params, ret, .. } = ft {
                                            let ps: Vec<Type> = params.iter().map(|p| self.convert_type(p)).collect();
                                            if args.len() != ps.len() {
                                                self.report_error(
                                                    format!("Field '{}.{}' expects {} argument(s), got {}",
                                                        rname, field, ps.len(), args.len()),
                                                    span);
                                            }
                                            for (i, arg) in args.iter().enumerate() {
                                                let at = self.infer_expr(arg);
                                                if i < ps.len() && at != ps[i]
                                                    && at != Type::Unknown && ps[i] != Type::Unknown {
                                                    self.report_error(
                                                        format!("Argument {} type mismatch: expected {:?}, got {:?}",
                                                            i, ps[i], at), arg.span);
                                                }
                                            }
                                            self.context_stack.pop();
                                            return self.convert_type(&ret);
                                        }
                                    }
                                }
                            }
                            Err(traits) => {
                                self.report_error(
                                    format!("Ambiguous method call '{}.{}': implemented by traits {:?}",
                                        rname, field, traits),
                                    span,
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
                let callee_ty = self.infer_expr(callee);
                let result = if let Type::Function { params, effects, ret } = callee_ty {
                    if args.len() != params.len() {
                        self.report_error(
                            format!("Expected {} arguments, got {}", params.len(), args.len()),
                            span
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
                        if fn_type_has_unknown(arg_ty) && fn_type_is_concrete(param_ty) {
                            self.report_error(
                                format!("Argument {}: cannot infer closure type from context; \
                                         annotate the closure, e.g. `|x: Int| -> Int ...`", i),
                                args[i].span,
                            );
                        } else if !is_param_generic && arg_ty != param_ty && *arg_ty != Type::Unknown && *param_ty != Type::Unknown {
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
                        self.check_borrow_barrier(&op_name, span);
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
                                    span,
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
                                                span,
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
}

fn fn_type_has_unknown(ty: &Type) -> bool {
    matches!(ty, Type::Function { params, ret, .. }
        if params.iter().any(|p| *p == Type::Unknown) || **ret == Type::Unknown)
}

fn fn_type_is_concrete(ty: &Type) -> bool {
    matches!(ty, Type::Function { params, ret, .. }
        if params.iter().all(|p| *p != Type::Unknown) && **ret != Type::Unknown)
}
