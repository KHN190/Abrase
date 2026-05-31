use crate::ast;
use crate::ast::Spanned;
use crate::ty::Type;
use super::super::*;

impl Checker {
    pub(super) fn infer_closure(
        &mut self,
        is_move: bool,
        params: &[ast::ClosureParam],
        effects: &[ast::EffectItem],
        return_type: &Option<ast::Type>,
        body: &Spanned<ast::Expr>,
    ) -> Type {
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
                            if !is_move {
                                if let Some(true) = self.peek_var_is_mut(name) {
                                    self.report_error(
                                        format!(
                                            "mutable binding '{}' cannot be captured by a non-move closure; \
                                             use `move |...|` to transfer ownership, or model mutation \
                                             through the `state` effect",
                                            name
                                        ),
                                        body.span,
                                    );
                                }
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
}
