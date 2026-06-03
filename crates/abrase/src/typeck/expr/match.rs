use crate::ast;
use crate::ast::Spanned;
use crate::ty::Type;
use super::super::*;

impl Checker {
    pub(super) fn infer_match(
        &mut self,
        scrutinee: &Spanned<ast::Expr>,
        arms: &[ast::MatchArm],
        span: ast::Span,
    ) -> Type {
                let prop = self.exn_prop;
                self.exn_prop = false;
                self.context_stack.push("In match expression".into());
                let required_before = self.fn_required_effects.clone();
                let scrutinee_ty = if let ast::Expr::Identifier(name) = &scrutinee.node {
                    self.get_var(name, true, scrutinee.span)
                } else {
                    self.exn_prop = true;
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
                    self.check_variant_exhaustiveness(&type_name, &covered, has_wildcard, span);
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
                    self.exn_prop = prop;
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

                if prop {
                    let mut yields = arms.iter()
                        .filter(|a| !matches!(a.body.node, ast::Expr::Throw(_) | ast::Expr::Return(_)))
                        .map(|a| self.tail_yields_result(&a.body));
                    if let Some(first) = yields.next() {
                        if yields.any(|y| y != first) {
                            self.report_error(
                                "in a fallible tail, all `match` arms must be uniformly fallible or \
                                 uniformly plain values; add `?` to the fallible arms".into(),
                                span,
                            );
                        }
                    }
                }

                // All arms must have same type
                if !arm_types.is_empty() {
                    let first = arm_types[0].clone();
                    for ty in arm_types.iter().skip(1) {
                        if *ty != first && first != Type::Unknown && *ty != Type::Unknown {
                            self.report_error("Match arm types do not match".into(), span);
                        }
                    }
                }

                self.context_stack.pop();
                if arm_types.is_empty() { Type::Unknown } else { arm_types[0].clone() }
    }
}
