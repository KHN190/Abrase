use crate::ast;
use crate::ast::Spanned;
use crate::ty::Type;
use super::super::*;

impl Checker {
    pub(super) fn infer_handle(
        &mut self,
        handler_expr: &Spanned<ast::Expr>,
        arms: &[ast::HandleArm],
        span: ast::Span,
    ) -> Type {
                if matches!(handler_expr.node, ast::Expr::Handle { .. }) {
                    self.report_error(
                        "inline nested handle is not allowed; extract the inner handle into a function and call it".into(),
                        handler_expr.span,
                    );
                }
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
                                        span,
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
                                    span
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
                        self.report_error("Handle arm types do not match".into(), span);
                    }
                }

                self.context_stack.pop();
                self.handled_effects = saved_handled;
                self.unhandled_effects.clear();
                result_ty
    }
}
