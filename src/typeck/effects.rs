use std::collections::HashMap;
use crate::ast;
use crate::ast::{Span, Spanned};
use crate::ty::{Ownership, Type};
use super::*;

impl Checker {

    pub fn register_effect(&mut self, name: String, operations: Vec<String>) {
        self.effect_registry.insert(name, operations);
    }

    pub fn get_effect(&self, name: &str) -> Option<Vec<String>> {
        self.effect_registry.get(name).cloned()
    }

    pub fn register_effect_alias(&mut self, alias_name: String, effects: Vec<crate::ty::Effect>) {
        self.effect_alias_registry.insert(alias_name, effects);
    }

    pub fn get_effect_alias(&self, alias_name: &str) -> Option<Vec<crate::ty::Effect>> {
        self.effect_alias_registry.get(alias_name).cloned()
    }

    pub fn push_effect(&mut self, effect: crate::ty::Effect) {
        self.current_effects.push(effect);
    }

    pub fn pop_effect(&mut self) {
        self.current_effects.pop();
    }

    pub fn effects_compatible(&self, expected: &[crate::ty::Effect], actual: &[crate::ty::Effect]) -> bool {
        expected.iter().all(|exp_effect| {
            actual.iter().any(|act_effect| self.effects_equal(exp_effect, act_effect))
        })
    }

    pub fn effects_equal(&self, e1: &crate::ty::Effect, e2: &crate::ty::Effect) -> bool {
        match (e1, e2) {
            (crate::ty::Effect::Total, crate::ty::Effect::Total) => true,
            (crate::ty::Effect::Async, crate::ty::Effect::Async) => true,
            (crate::ty::Effect::Alloc, crate::ty::Effect::Alloc) => true,
            (crate::ty::Effect::Nondet, crate::ty::Effect::Nondet) => true,
            (crate::ty::Effect::Exn(t1), crate::ty::Effect::Exn(t2)) => t1 == t2,
            _ => false,
        }
    }

    pub fn convert_effect(&self, eff: &ast::EffectItem) -> Option<crate::ty::Effect> {
        let name = eff.name.join(".").to_lowercase();
        match name.as_str() {
            "io" | "alloc" => Some(crate::ty::Effect::Alloc),
            "async" => Some(crate::ty::Effect::Async),
            "exn" => {
                if let Some(arg) = &eff.arg {
                    Some(crate::ty::Effect::Exn(Box::new(self.convert_type(arg))))
                } else {
                    Some(crate::ty::Effect::Exn(Box::new(Type::Named("Exception".into()))))
                }
            },
            "nondet" => Some(crate::ty::Effect::Nondet),
            _ => self.get_effect_alias(&name).and_then(|mut effs| effs.pop()),
        }
    }

    // Effect Unification & Inference

    pub fn set_fn_declared_effects(&mut self, effects: Vec<crate::ty::Effect>) {
        self.fn_declared_effects = effects;
    }

    pub fn get_fn_declared_effects(&self) -> &[crate::ty::Effect] {
        &self.fn_declared_effects
    }

    pub fn add_required_effect(&mut self, effect: crate::ty::Effect) {
        if !self.fn_required_effects.iter().any(|e| self.effects_equal(e, &effect)) {
            self.fn_required_effects.push(effect);
        }
    }

    pub fn get_fn_required_effects(&self) -> &[crate::ty::Effect] {
        &self.fn_required_effects
    }

    pub fn check_effect_compatibility(&mut self, fn_effects: &[crate::ty::Effect], call_span: Span) -> bool {
        for required_effect in self.fn_required_effects.iter() {
            let found = fn_effects.iter().any(|e| self.effects_equal(e, required_effect));
            if !found {
                self.report_error(
                    format!("Function call requires effect {:?} not in function signature", required_effect),
                    call_span
                );
                return false;
            }
        }
        true
    }

    pub fn unify_effects(&self, effects1: &[crate::ty::Effect], effects2: &[crate::ty::Effect]) -> Vec<crate::ty::Effect> {
        let mut unified = effects1.to_vec();
        for effect in effects2 {
            if !unified.iter().any(|e| self.effects_equal(e, effect)) {
                unified.push(effect.clone());
            }
        }
        unified
    }

    pub fn effects_subsume(&self, required: &[crate::ty::Effect], provided: &[crate::ty::Effect]) -> bool {
        required.iter().all(|req| {
            provided.iter().any(|prov| self.effects_equal(req, prov))
        })
    }

    pub fn infer_closure_effects(&self, body_effects: &[crate::ty::Effect]) -> Vec<crate::ty::Effect> {
        if !self.fn_declared_effects.is_empty() {
            self.fn_declared_effects.clone()
        } else {
            body_effects.to_vec()
        }
    }

    pub fn convert_effect_items(&self, items: &[ast::EffectItem]) -> Vec<crate::ty::Effect> {
        items.iter()
            .filter_map(|item| self.convert_effect(item))
            .collect()
    }

    // Effect Propagation & Scope Semantics

    pub fn mark_effect_handled(&mut self, effect_name: String) {
        if !self.handled_effects.contains(&effect_name) {
            self.handled_effects.push(effect_name);
        }
    }

    pub fn get_handled_effects(&self) -> &[String] {
        &self.handled_effects
    }

    pub fn compute_unhandled_effects(&mut self, all_effects: &[crate::ty::Effect]) {
        self.unhandled_effects.clear();
        for effect in all_effects {
            let handled = match effect {
                crate::ty::Effect::Total => self.handled_effects.contains(&"total".into()),
                crate::ty::Effect::Async => self.handled_effects.contains(&"async".into()),
                crate::ty::Effect::Alloc => self.handled_effects.contains(&"io".into()) || self.handled_effects.contains(&"alloc".into()),
                crate::ty::Effect::Nondet => self.handled_effects.contains(&"nondet".into()),
                crate::ty::Effect::Exn(_) => self.handled_effects.contains(&"exn".into()),
            };
            if !handled {
                self.unhandled_effects.push(effect.clone());
            }
        }
    }

    pub fn get_unhandled_effects(&self) -> &[crate::ty::Effect] {
        &self.unhandled_effects
    }

    pub fn validate_parameterized_exn_handler(&self, exn_type: &Type, pattern: &Option<Spanned<ast::Pattern>>) -> bool {
        if let Some(pat) = pattern {
            match &pat.node {
                ast::Pattern::Bind(_) => {
                    // For parameterized exceptions, the bound variable should have the exception type
                    *exn_type != Type::Unknown
                },
                _ => true,
            }
        } else {
            true
        }
    }

    pub fn validate_scope_with_context(&self, with_expr_type: &Type) -> bool {
        // Scope with <expr> should provide context, validate type is not Unknown
        *with_expr_type != Type::Unknown
    }

    pub fn propagate_effects_to_parent(&mut self) {
        let unhandled = self.unhandled_effects.clone();
        for effect in unhandled {
            self.add_required_effect(effect);
        }
    }

    pub fn clear_handle_context(&mut self) {
        self.handled_effects.clear();
        self.unhandled_effects.clear();
    }

    // Const Effect Checking

    pub fn register_function_effects(&mut self, fn_name: String, effects: Vec<ast::EffectItem>) {
        self.function_effects.insert(fn_name, effects);
    }

    pub fn register_effect_for_op(&mut self, op_name: &str, effects: Vec<ast::EffectItem>) {
        self.op_effects.insert(op_name.into(), effects);
    }

    pub fn insert_const_var(&mut self, name: String, ty: Type) {
        self.const_vars.insert(name.clone());
        self.const_registry.insert(name, ty);
    }
    fn has_pure_effects(effects: &[ast::EffectItem]) -> bool {
        effects.is_empty()
    }

    pub fn infer_expr_effects(&self, expr: &ast::Expr) -> Vec<ast::EffectItem> {
        match expr {
            ast::Expr::Literal(_) => vec![],
            ast::Expr::Identifier(name) => {
                // If referencing a const var, it's pure
                if self.const_vars.contains(name) {
                    vec![]
                } else if let Some(effects) = self.function_effects.get(name) {
                    effects.clone()
                } else {
                    vec![]
                }
            }
            ast::Expr::Call { callee, args: _ } => {
                if let ast::Expr::Identifier(fn_name) = &callee.node {
                    if let Some(effects) = self.function_effects.get(fn_name) {
                        effects.clone()
                    } else {
                        vec![]
                    }
                } else {
                    vec![]
                }
            }
            ast::Expr::If { condition: _, consequence, alternative } => {
                let mut effects = self.infer_expr_effects(&consequence.node);
                if let Some(alt) = alternative {
                    let alt_effects = self.infer_expr_effects(&alt.node);
                    effects.extend(alt_effects);
                }
                effects
            }
            ast::Expr::Binary { op, .. } => {
                match op {
                    ast::BinaryOp::Assign | ast::BinaryOp::AddAssign | ast::BinaryOp::SubAssign |
                    ast::BinaryOp::MulAssign | ast::BinaryOp::DivAssign | ast::BinaryOp::ModAssign => {
                        vec![ast::EffectItem {
                            name: vec!["mutation".into()],
                            arg: None,
                        }]
                    }
                    _ => vec![],
                }
            }
            _ => vec![],
        }
    }

    pub fn check_const_expr(&mut self, expr: &ast::Expr, span: Span) -> bool {
        match expr {
            // Literals are always pure
            ast::Expr::Literal(_) => true,

            // Identifiers: check if they refer to const vars
            ast::Expr::Identifier(name) => {
                if self.const_vars.contains(name) {
                    true
                } else if let Some(meta) = self.scopes.last().and_then(|s| s.vars.get(name)) {
                    if meta.is_mut {
                        self.report_error(
                            format!("Mutable variable '{}' cannot be used in const expression", name),
                            span
                        );
                        false
                    } else {
                        true
                    }
                } else {
                    true // Unknown identifier, let other checks handle it
                }
            }

            // Function calls: check if function has pure effects AND all arguments are pure
            ast::Expr::Call { callee, args } => {
                for arg in args {
                    if !self.check_const_expr(&arg.node, arg.span) {
                        return false;
                    }
                }

                if let ast::Expr::Identifier(fn_name) = &callee.node {
                    if let Some(effects) = self.function_effects.get(fn_name) {
                        if Self::has_pure_effects(effects) {
                            true
                        } else {
                            let effect_names: Vec<String> = effects.iter()
                                .map(|e| e.name.join("."))
                                .collect();
                            self.report_error(
                                format!(
                                    "Function call to '{}' with effects {:?} cannot be used in const expression. \
                                     Const expressions must be pure (no io, exn, etc.)",
                                    fn_name, effect_names
                                ),
                                span
                            );
                            false
                        }
                    } else {
                        true
                    }
                } else {
                    true
                }
            }

            // Binary operations: assignments are not allowed in const
            ast::Expr::Binary { op, left, right } => {
                match op {
                    ast::BinaryOp::Assign | ast::BinaryOp::AddAssign | ast::BinaryOp::SubAssign |
                    ast::BinaryOp::MulAssign | ast::BinaryOp::DivAssign | ast::BinaryOp::ModAssign => {
                        self.report_error(
                            "Assignment is not allowed in const expression".into(),
                            span
                        );
                        false
                    }
                    _ => {
                        // Check operands are also const-compatible
                        self.check_const_expr(&left.node, left.span) &&
                        self.check_const_expr(&right.node, right.span)
                    }
                }
            }

            // If-expressions: both branches must be pure
            ast::Expr::If { condition, consequence, alternative } => {
                let cond_ok = self.check_const_expr(&condition.node, condition.span);
                let cons_ok = self.check_const_expr(&consequence.node, consequence.span);
                let alt_ok = if let Some(alt) = alternative {
                    self.check_const_expr(&alt.node, alt.span)
                } else {
                    true
                };

                cond_ok && cons_ok && alt_ok
            }

            // Closures: check that the body doesn't use mutable state
            ast::Expr::Closure { body, .. } => {
                self.check_const_expr(&body.node, body.span)
            }

            // Default: allow if we can't prove it's impure
            _ => true,
        }
    }

    pub fn clear_pattern_analysis(&mut self) {
        self.covered_patterns.clear();
        self.unreachable_patterns.clear();
    }

    // Closure Effect Declaration Validation

    pub fn validate_closure_effects(&mut self,
        declared_effects: &[crate::ty::Effect],
        inferred_effects: &[crate::ty::Effect],
        span: Span
    ) -> bool {
        // If no effects are declared, inferred effects are always valid
        if declared_effects.is_empty() {
            return true;
        }
        // If effects are declared, inferred effects must be a subset
        // i.e., every inferred effect must exist in declared effects
        for inferred in inferred_effects {
            let is_covered = declared_effects.iter().any(|decl| {
                self.effects_equal(inferred, decl)
            });

            if !is_covered {
                self.report_error(
                    format!("Closure body produces effect not declared in closure type"),
                    span
                );
                return false;
            }
        }
        true
    }

    pub fn check_closure_effect_declaration(&mut self,
        declared_effects: &[crate::ty::Effect],
        body_span: Span
    ) -> bool {
        // Check if declared effects cover all required effects from body
        // fn_required_effects contains effects produced by closure body
        let inferred = &self.fn_required_effects.clone();
        self.validate_closure_effects(declared_effects, inferred, body_span)
    }

    pub fn inferred_effects_exceed_declared(&self,
        declared: &[crate::ty::Effect],
        inferred: &[crate::ty::Effect]
    ) -> Vec<crate::ty::Effect> {
        let mut exceeds = Vec::new();
        for inf in inferred {
            let found = declared.iter().any(|decl| {
                self.effects_equal(inf, decl)
            });
            if !found {
                exceeds.push(inf.clone());
            }
        }
        exceeds
    }

    pub fn all_effects_declared(&self,
        declared: &[crate::ty::Effect],
        inferred: &[crate::ty::Effect]
    ) -> bool {
        inferred.iter().all(|inf| {
            declared.iter().any(|decl| {
                self.effects_equal(inf, decl)
            })
        })
    }
}
