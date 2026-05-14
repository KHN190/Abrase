// src/typeck.rs

use std::collections::HashMap;
use crate::ast;
use crate::ast::{Span, Spanned};
use crate::ty::{Ownership, Type};

fn elem_name(ty: &ast::Type) -> String {
    match ty {
        ast::Type::Named(n) => n.clone(),
        _ => "?".into(),
    }
}

#[derive(Debug, Clone)]
pub struct TypeError {
    pub message: String,
    pub span: Span,
    pub context: Vec<String>,
}

impl TypeError {
    pub fn display(&self) -> String {
        let mut output = format!("TypeError at line {}, col {}: {}", self.span.line, self.span.col, self.message);
        if !self.context.is_empty() {
            output.push_str("\n  Context stack:");
            for (i, ctx) in self.context.iter().enumerate() {
                output.push_str(&format!("\n    {}: {}", i + 1, ctx));
            }
        }
        output
    }
}

#[derive(Clone)]
struct VarMeta {
    ty: Type,
    is_mut: bool,
    is_moved: bool,
    defined_at: Span,
    moved_at: Option<Span>,
    // Phase 7: Ownership & Borrowing
    immut_borrow_count: usize, // count of active immutable borrows
    mut_borrow_active: bool,   // whether a mutable borrow is active
}

#[derive(Clone)]
pub struct Scope {
    vars: HashMap<String, VarMeta>,
}

pub struct Checker {
    scopes: Vec<Scope>,
    pub errors: Vec<TypeError>,
    context_stack: Vec<String>,
    loop_depth: usize,
    in_function: bool,
    fn_return_type: Option<crate::ty::Type>,
    active_effects: Vec<String>, // track active effect handlers
    effect_stack: Vec<Vec<String>>, // stack of effect scopes for region/scope

    // Type Environment (Phase 5)
    fn_registry: HashMap<String, (Vec<Type>, Type)>, // name -> (params, return_type)
    type_registry: HashMap<String, ast::TypeBody>, // name -> type definition
    const_registry: HashMap<String, Type>, // name -> const type

    // Phase 7: Ownership & Borrowing
    borrow_stack: Vec<(String, bool)>, // stack of (var_name, is_mutable)

    // Phase 8: Effects System
    effect_registry: HashMap<String, Vec<String>>, // effect_name -> list of operations
    effect_alias_registry: HashMap<String, Vec<crate::ty::Effect>>, // alias_name -> effects
    current_effects: Vec<crate::ty::Effect>, // effects in current function context

    // Phase 9: Type Ownership Attributes
    ownership_registry: HashMap<String, Ownership>, // type_name -> ownership

    // Phase 10: Effect Unification & Inference
    fn_declared_effects: Vec<crate::ty::Effect>, // declared effects for current function
    fn_required_effects: Vec<crate::ty::Effect>, // required effects from function body

    // Phase 11: Effect Shadowing, Propagation & Scope Semantics
    handled_effects: Vec<String>, // effects handled in current handle block
    unhandled_effects: Vec<crate::ty::Effect>, // effects not handled, propagated up

    // Phase 12: Generics & Trait Constraints
    trait_registry: HashMap<String, Vec<String>>, // trait_name -> list of methods/requirements
    impl_registry: HashMap<(String, String), bool>, // (type_name, trait_name) -> implemented
    generic_params: HashMap<String, Vec<String>>, // fn_name -> list of generic type params
    trait_bounds: HashMap<String, Vec<String>>, // generic_param -> list of trait bounds

    // Phase 13: Region Escape Analysis & Advanced Borrow Checking
    region_stack: Vec<String>, // stack of active region names
    reference_lifetimes: HashMap<String, String>, // ref_name -> region_name (lifetime)
    pattern_borrows: HashMap<String, Vec<String>>, // pattern_var -> list of borrow constraints

    // Phase 14: Pattern Matching Analysis (Exhaustiveness & Unreachability)
    covered_patterns: Vec<String>, // patterns covered so far in match
    unreachable_patterns: Vec<usize>, // indices of unreachable patterns

    // Phase 15: Visibility & Module Scoping
    current_module: Vec<String>, // current module path, e.g. ["io", "file"]
    public_items: std::collections::HashSet<String>, // items marked as pub (fully qualified names)
    private_items: std::collections::HashSet<String>, // items marked as private

    // Phase 16: Qualified Name Resolution
    qualified_names: HashMap<String, Vec<Vec<String>>>, // simple_name -> list of possible qualified paths
}

impl Checker {
    pub fn new() -> Self {
        Self {
            scopes: vec![Scope { vars: HashMap::new() }],
            errors: Vec::new(),
            context_stack: Vec::new(),
            loop_depth: 0,
            in_function: false,
            fn_return_type: None,
            active_effects: Vec::new(),
            effect_stack: vec![Vec::new()],
            fn_registry: HashMap::new(),
            type_registry: HashMap::new(),
            const_registry: HashMap::new(),
            borrow_stack: Vec::new(),
            effect_registry: HashMap::new(),
            effect_alias_registry: HashMap::new(),
            current_effects: Vec::new(),
            ownership_registry: HashMap::new(),
            fn_declared_effects: Vec::new(),
            fn_required_effects: Vec::new(),
            handled_effects: Vec::new(),
            unhandled_effects: Vec::new(),
            trait_registry: HashMap::new(),
            impl_registry: HashMap::new(),
            generic_params: HashMap::new(),
            trait_bounds: HashMap::new(),
            region_stack: Vec::new(),
            reference_lifetimes: HashMap::new(),
            pattern_borrows: HashMap::new(),
            covered_patterns: Vec::new(),
            unreachable_patterns: Vec::new(),
            current_module: vec!["root".into()],
            public_items: std::collections::HashSet::new(),
            private_items: std::collections::HashSet::new(),
            qualified_names: HashMap::new(),
        }
    }

    pub fn enter_scope(&mut self) {
        self.scopes.push(Scope { vars: HashMap::new() });
    }

    pub fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn display_errors(&self) -> String {
        if self.errors.is_empty() {
            return "No type errors".to_string();
        }
        let mut output = format!("Found {} type error(s):\n", self.errors.len());
        for (i, error) in self.errors.iter().enumerate() {
            output.push_str(&format!("\n{}: {}", i + 1, error.display()));
        }
        output
    }

    pub fn insert_var(&mut self, name: String, ty: Type, is_mut: bool, defined_at: Span) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.vars.insert(name, VarMeta {
                ty,
                is_mut,
                is_moved: false,
                defined_at,
                moved_at: None,
                immut_borrow_count: 0,
                mut_borrow_active: false,
            });
        }
    }

    // Phase 5: Type Environment Management
    pub fn register_function(&mut self, name: String, params: Vec<Type>, ret: Type) {
        self.fn_registry.insert(name, (params, ret));
    }

    pub fn get_function(&self, name: &str) -> Option<(Vec<Type>, Type)> {
        self.fn_registry.get(name).cloned()
    }

    pub fn register_type(&mut self, name: String, body: ast::TypeBody) {
        self.type_registry.insert(name, body);
    }

    pub fn get_type(&self, name: &str) -> Option<ast::TypeBody> {
        self.type_registry.get(name).cloned()
    }

    pub fn register_const(&mut self, name: String, ty: Type) {
        self.const_registry.insert(name, ty);
    }

    pub fn get_const(&self, name: &str) -> Option<Type> {
        self.const_registry.get(name).cloned()
    }

    // Phase 7: Ownership & Borrowing
    pub fn try_immut_borrow(&mut self, var_name: &str, _borrow_span: Span) -> Result<(), String> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(meta) = scope.vars.get_mut(var_name) {
                if meta.mut_borrow_active {
                    return Err(format!("Cannot immutably borrow '{}': mutable borrow already active", var_name));
                }
                meta.immut_borrow_count += 1;
                self.borrow_stack.push((var_name.to_string(), false));
                return Ok(());
            }
        }
        Err(format!("Variable '{}' not found", var_name))
    }

    pub fn try_mut_borrow(&mut self, var_name: &str, _borrow_span: Span) -> Result<(), String> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(meta) = scope.vars.get_mut(var_name) {
                if meta.immut_borrow_count > 0 {
                    return Err(format!("Cannot mutably borrow '{}': immutable borrow already active", var_name));
                }
                if meta.mut_borrow_active {
                    return Err(format!("Cannot mutably borrow '{}': mutable borrow already active", var_name));
                }
                if !meta.is_mut {
                    return Err(format!("Cannot mutably borrow immutable variable '{}'", var_name));
                }
                meta.mut_borrow_active = true;
                self.borrow_stack.push((var_name.to_string(), true));
                return Ok(());
            }
        }
        Err(format!("Variable '{}' not found", var_name))
    }

    pub fn release_borrow(&mut self, var_name: &str) {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(meta) = scope.vars.get_mut(var_name) {
                if meta.immut_borrow_count > 0 {
                    meta.immut_borrow_count = meta.immut_borrow_count.saturating_sub(1);
                }
                if meta.mut_borrow_active && self.borrow_stack.last().map_or(false, |(name, _)| name == var_name) {
                    meta.mut_borrow_active = false;
                }
                return;
            }
        }
    }

    pub fn check_ownership(&self, ty: &Type) -> Ownership {
        ty.ownership()
    }

    // Phase 8: Effects System
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
        // All expected effects must be present in actual effects
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

    // Phase 9: Type Ownership Attributes
    pub fn register_ownership(&mut self, type_name: String, ownership: Ownership) {
        self.ownership_registry.insert(type_name, ownership);
    }

    pub fn get_type_ownership(&self, type_name: &str) -> Option<Ownership> {
        self.ownership_registry.get(type_name).cloned()
    }

    pub fn infer_type_ownership(&self, type_name: &str) -> Ownership {
        // Primitives are always Copy (cannot be overridden)
        match type_name {
            "Int" | "Float" | "Bool" | "Char" | "Unit" => return Ownership::Copy,
            _ => {}
        }

        // Check registry for explicit declarations
        if let Some(ownership) = self.get_type_ownership(type_name) {
            return ownership;
        }

        // String defaults to Share (can be shared across ownership boundaries)
        if type_name == "String" {
            return Ownership::Share;
        }

        // Unknown types default to Move
        Ownership::Move
    }

    pub fn register_type_with_ownership(&mut self, type_name: String, ownership: Ownership, body: ast::TypeBody) {
        self.register_ownership(type_name.clone(), ownership);
        self.register_type(type_name, body);
    }

    pub fn convert_ownership_attr(&self, attr: &Option<ast::OwnershipAttr>) -> Ownership {
        match attr {
            Some(ast::OwnershipAttr::Copy) => Ownership::Copy,
            Some(ast::OwnershipAttr::Move) => Ownership::Move,
            Some(ast::OwnershipAttr::Share) => Ownership::Share,
            None => Ownership::Move, // default
        }
    }

    // Phase 10: Effect Unification & Inference
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
        // Check if provided effects include all required effects
        required.iter().all(|req| {
            provided.iter().any(|prov| self.effects_equal(req, prov))
        })
    }

    pub fn infer_closure_effects(&self, body_effects: &[crate::ty::Effect]) -> Vec<crate::ty::Effect> {
        // If declared effects, use those; otherwise infer from body
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

    // Phase 11: Effect Shadowing, Propagation & Scope Semantics
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
        // Propagate unhandled effects up to parent context
        let unhandled = self.unhandled_effects.clone();
        for effect in unhandled {
            self.add_required_effect(effect);
        }
    }

    pub fn clear_handle_context(&mut self) {
        self.handled_effects.clear();
        self.unhandled_effects.clear();
    }

    // Phase 12: Generics & Trait Constraints
    pub fn register_trait(&mut self, trait_name: String, methods: Vec<String>) {
        self.trait_registry.insert(trait_name, methods);
    }

    pub fn get_trait(&self, trait_name: &str) -> Option<Vec<String>> {
        self.trait_registry.get(trait_name).cloned()
    }

    pub fn register_impl(&mut self, type_name: String, trait_name: String) {
        self.impl_registry.insert((type_name, trait_name), true);
    }

    pub fn has_impl(&self, type_name: &str, trait_name: &str) -> bool {
        self.impl_registry.get(&(type_name.to_string(), trait_name.to_string())).copied().unwrap_or(false)
    }

    pub fn register_generic_params(&mut self, fn_name: String, params: Vec<String>) {
        self.generic_params.insert(fn_name, params);
    }

    pub fn get_generic_params(&self, fn_name: &str) -> Option<Vec<String>> {
        self.generic_params.get(fn_name).cloned()
    }

    pub fn register_trait_bound(&mut self, param: String, trait_name: String) {
        self.trait_bounds.entry(param)
            .or_insert_with(Vec::new)
            .push(trait_name);
    }

    pub fn get_trait_bounds(&self, param: &str) -> Option<Vec<String>> {
        self.trait_bounds.get(param).cloned()
    }

    pub fn validate_where_clause(&self, param: &str, provided_type: &str) -> bool {
        if let Some(bounds) = self.get_trait_bounds(param) {
            bounds.iter().all(|trait_name| self.has_impl(provided_type, trait_name))
        } else {
            true
        }
    }

    pub fn validate_generic_instance(&self, type_name: &str, type_args: &[Type]) -> bool {
        // Check if type exists and has correct number of generic parameters
        if let Some(expected_params) = self.get_generic_params(type_name) {
            expected_params.len() == type_args.len()
        } else {
            true
        }
    }

    pub fn check_all_trait_bounds(&self, fn_name: &str, type_args: &[(String, Type)]) -> bool {
        // Get generic parameters for function
        if let Some(_params) = self.get_generic_params(fn_name) {
            // For each type argument, validate its trait bounds
            for (param_name, arg_type) in type_args {
                if let Some(bounds) = self.get_trait_bounds(param_name) {
                    let type_str = format!("{:?}", arg_type);
                    for trait_name in bounds {
                        if !self.has_impl(&type_str, &trait_name) {
                            return false;
                        }
                    }
                }
            }
            true
        } else {
            true
        }
    }

    // Phase 13: Region Escape Analysis & Advanced Borrow Checking
    pub fn push_region(&mut self, region_name: String) {
        self.region_stack.push(region_name);
    }

    pub fn pop_region(&mut self) -> Option<String> {
        self.region_stack.pop()
    }

    pub fn get_current_region(&self) -> Option<&str> {
        self.region_stack.last().map(|s| s.as_str())
    }

    pub fn bind_reference_lifetime(&mut self, ref_name: String, region: String) {
        self.reference_lifetimes.insert(ref_name, region);
    }

    pub fn get_reference_lifetime(&self, ref_name: &str) -> Option<String> {
        self.reference_lifetimes.get(ref_name).cloned()
    }

    pub fn check_escape_analysis(&mut self, expr_region: Option<&str>, ref_region: Option<&str>, escape_span: Span) -> bool {
        // A reference in an inner region should not escape to outer region
        match (expr_region, ref_region) {
            (Some(outer), Some(inner)) if outer != inner => {
                self.report_error(
                    format!("Reference from region '{}' escapes to region '{}'", inner, outer),
                    escape_span
                );
                false
            }
            _ => true,
        }
    }

    pub fn register_pattern_borrow(&mut self, pattern_var: String, borrow_constraint: String) {
        self.pattern_borrows.entry(pattern_var)
            .or_insert_with(Vec::new)
            .push(borrow_constraint);
    }

    pub fn get_pattern_borrows(&self, pattern_var: &str) -> Option<Vec<String>> {
        self.pattern_borrows.get(pattern_var).cloned()
    }

    pub fn check_pattern_borrow_exclusivity(&self, patterns: &[&str]) -> bool {
        // Check that borrow constraints from pattern matching don't conflict
        let mut has_mut_borrow = false;

        for pattern in patterns {
            if let Some(borrows) = self.get_pattern_borrows(pattern) {
                for borrow in borrows {
                    // Check if it's exactly "mut" or starts with "mut_" (not "immut")
                    if borrow == "mut" || (borrow.starts_with("mut") && !borrow.starts_with("immut")) {
                        // Can't have multiple mutable borrows
                        if has_mut_borrow {
                            return false;
                        }
                        has_mut_borrow = true;
                    }
                }
            }
        }
        true
    }

    pub fn validate_reference_escape(&self, ref_var: &str, current_scope_region: Option<&str>) -> bool {
        if let Some(ref_region) = self.get_reference_lifetime(ref_var) {
            // Reference can only escape to parent scope, not to different region
            match current_scope_region {
                Some(current) if current != ref_region => false,
                _ => true,
            }
        } else {
            true
        }
    }

    pub fn clear_region_context(&mut self) {
        self.region_stack.clear();
        self.reference_lifetimes.clear();
        self.pattern_borrows.clear();
    }

    // Phase 14: Pattern Matching Analysis (Exhaustiveness & Unreachability)
    pub fn add_covered_pattern(&mut self, pattern: String) {
        self.covered_patterns.push(pattern);
    }

    pub fn get_covered_patterns(&self) -> &[String] {
        &self.covered_patterns
    }

    pub fn mark_unreachable_pattern(&mut self, pattern_index: usize) {
        self.unreachable_patterns.push(pattern_index);
    }

    pub fn get_unreachable_patterns(&self) -> &[usize] {
        &self.unreachable_patterns
    }

    pub fn check_pattern_subsumption(&self, new_pattern: &str, existing_patterns: &[&str]) -> bool {
        // Check if new_pattern is subsumed by (covered by) existing patterns
        // Wildcard pattern subsumes everything
        if existing_patterns.contains(&"_") {
            return true;
        }

        // Check if exact same pattern exists
        if existing_patterns.iter().any(|p| *p == new_pattern) {
            return true;
        }

        false
    }

    pub fn validate_match_exhaustiveness(&mut self, scrutinee_type: &Type, patterns: &[String], span: Span) -> bool {
        // Check if all variants are covered for typed expressions
        match scrutinee_type {
            Type::Named(name) if name.contains("Variant") || name.contains("Enum") => {
                // For variant types, check if wildcard exists or specific coverage
                if patterns.contains(&"_".to_string()) {
                    return true;
                }

                // If no wildcard, it's non-exhaustive (in real scenario would need variant info)
                self.report_error("Non-exhaustive patterns in match".into(), span);
                false
            }
            _ => {
                // For other types, wildcard pattern is sufficient
                patterns.contains(&"_".to_string())
            }
        }
    }

    pub fn detect_unreachable_patterns(&mut self, patterns: &[String]) -> Vec<usize> {
        let mut unreachable = Vec::new();
        let mut covered = Vec::new();

        for (i, pattern) in patterns.iter().enumerate() {
            // Wildcard pattern makes all subsequent patterns unreachable
            if pattern == "_" {
                for j in (i + 1)..patterns.len() {
                    unreachable.push(j);
                }
                break;
            }

            // Check if this pattern is subsumed by previous patterns
            let covered_strs: Vec<&str> = covered.iter().map(|s: &String| s.as_str()).collect();
            if self.check_pattern_subsumption(pattern, &covered_strs) {
                unreachable.push(i);
            } else {
                covered.push(pattern.clone());
            }
        }

        unreachable
    }

    pub fn is_pattern_exhaustive(&self, patterns: &[String]) -> bool {
        // Pattern set is exhaustive if it contains a wildcard
        patterns.contains(&"_".to_string())
    }

    pub fn clear_pattern_analysis(&mut self) {
        self.covered_patterns.clear();
        self.unreachable_patterns.clear();
    }

    // Phase 15: Visibility & Module Scoping
    pub fn push_module(&mut self, module_name: String) {
        self.current_module.push(module_name);
    }

    pub fn pop_module(&mut self) {
        if self.current_module.len() > 1 {
            self.current_module.pop();
        }
    }

    pub fn get_current_module(&self) -> Vec<String> {
        self.current_module.clone()
    }

    pub fn set_current_module(&mut self, module_path: Vec<String>) {
        if !module_path.is_empty() {
            self.current_module = module_path;
        }
    }

    pub fn mark_public(&mut self, item_name: String) {
        let qualified_name = format!("{}::{}", self.current_module.join("::"), item_name);
        self.public_items.insert(qualified_name.clone());
        self.private_items.remove(&qualified_name);
    }

    pub fn mark_private(&mut self, item_name: String) {
        let qualified_name = format!("{}::{}", self.current_module.join("::"), item_name);
        self.private_items.insert(qualified_name.clone());
        self.public_items.remove(&qualified_name);
    }

    pub fn is_public(&self, item_name: &str) -> bool {
        // Check if item is accessible from current module
        // An item is accessible if:
        // 1. It's marked as public
        // 2. It's in the same module as the current context
        // 3. It's a built-in item

        // Check for public registration
        for public_item in &self.public_items {
            if public_item.ends_with(&format!("::{}", item_name)) {
                return true;
            }
        }

        // Check if it's in the same module
        let qualified_name = format!("{}::{}", self.current_module.join("::"), item_name);
        !self.private_items.contains(&qualified_name)
    }

    pub fn is_accessible(&self, item_name: &str, item_module: &[String]) -> bool {
        // Check if item from item_module is accessible from current_module
        // Items from the same module are always accessible
        if item_module == self.current_module {
            return true;
        }

        // Items marked as public are accessible from anywhere
        let qualified_name = format!("{}::{}", item_module.join("::"), item_name);
        self.public_items.contains(&qualified_name)
    }

    pub fn validate_visibility(&mut self, item_name: &str, item_module: &[String], access_span: Span) -> bool {
        // Validate that item_name from item_module is accessible
        if self.is_accessible(item_name, item_module) {
            return true;
        }

        // Report visibility error
        let qualified = format!("{}::{}", item_module.join("::"), item_name);
        self.report_error(
            format!("Cannot access private item '{}'", qualified),
            access_span
        );
        false
    }

    pub fn get_public_items(&self) -> Vec<String> {
        self.public_items.iter().cloned().collect()
    }

    pub fn get_private_items(&self) -> Vec<String> {
        self.private_items.iter().cloned().collect()
    }

    pub fn clear_visibility_context(&mut self) {
        self.current_module = vec!["root".into()];
        self.public_items.clear();
        self.private_items.clear();
    }

    // Phase 16: Qualified Name Resolution
    pub fn register_qualified_name(&mut self, simple_name: String, qualified_path: Vec<String>) {
        self.qualified_names
            .entry(simple_name)
            .or_insert_with(Vec::new)
            .push(qualified_path);
    }

    pub fn resolve_qualified_name(&self, name_parts: &[String]) -> Option<Vec<String>> {
        // Try to resolve a potentially qualified name through module hierarchy
        if name_parts.is_empty() {
            return None;
        }

        // Try exact match first (fully qualified from root)
        if name_parts[0] == "root" {
            // This is a fully qualified name starting with root
            if let Some(paths_list) = self.qualified_names.get(&name_parts[name_parts.len() - 1]) {
                for path in paths_list {
                    if path == name_parts {
                        return Some(path.clone());
                    }
                }
            }
        }

        // Try relative to current module
        let mut candidate = self.current_module.clone();
        for part in name_parts {
            candidate.push(part.clone());
        }

        // Look for this full path in qualified_names
        if let Some(paths_list) = self.qualified_names.get(&name_parts[name_parts.len() - 1]) {
            for path in paths_list {
                if path == &candidate {
                    return Some(path.clone());
                }
            }
        }

        // Try as-is if it's in the registry
        if let Some(paths_list) = self.qualified_names.get(&name_parts[name_parts.len() - 1]) {
            for path in paths_list {
                if path == name_parts {
                    return Some(path.clone());
                }
            }
        }

        None
    }

    pub fn resolve_name(&self, name: &str) -> Option<Vec<String>> {
        // Simple name resolution - get first possible path
        if let Some(paths) = self.qualified_names.get(name) {
            if !paths.is_empty() {
                return Some(paths[0].clone());
            }
        }

        None
    }

    pub fn is_name_resolvable(&self, name_parts: &[String]) -> bool {
        self.resolve_qualified_name(name_parts).is_some()
    }

    pub fn get_all_resolutions(&self, name: &str) -> Vec<Vec<String>> {
        self.qualified_names
            .get(name)
            .cloned()
            .unwrap_or_default()
    }

    pub fn clear_name_resolution(&mut self) {
        self.qualified_names.clear();
    }

    // Phase 5: Pattern Matching Support
    pub fn check_pattern(&mut self, pattern: &Spanned<ast::Pattern>, value_ty: &Type, _pattern_span: Span) {
        match &pattern.node {
            ast::Pattern::Bind(name) => {
                // Bind pattern: extract variable
                self.insert_var(name.clone(), value_ty.clone(), false, pattern.span);
            }
            ast::Pattern::Wildcard => {
                // Wildcard: accept any type
            }
            ast::Pattern::Literal(lit) => {
                // Literal pattern: verify value matches literal type
                let lit_ty = match lit {
                    ast::Literal::Int(_) => Type::Int,
                    ast::Literal::Float(_) => Type::Float,
                    ast::Literal::Bool(_) => Type::Bool,
                    ast::Literal::Char(_) => Type::Char,
                    ast::Literal::String(_) | ast::Literal::StringInterp(_) => Type::String,
                    ast::Literal::Unit => Type::Unit,
                };
                if *value_ty != lit_ty && *value_ty != Type::Unknown {
                    self.report_error(
                        format!("Pattern type mismatch: expected {:?}, found {:?}", lit_ty, value_ty),
                        pattern.span
                    );
                }
            }
            ast::Pattern::Tuple(pats) => {
                // Tuple pattern: recursively check nested patterns
                if let Type::Tuple(elem_types) = value_ty {
                    if pats.len() != elem_types.len() {
                        self.report_error(
                            format!("Tuple pattern length mismatch: expected {}, got {}",
                                elem_types.len(), pats.len()),
                            pattern.span
                        );
                    }
                    for (pat, elem_ty) in pats.iter().zip(elem_types.iter()) {
                        self.check_pattern(pat, elem_ty, pat.span);
                    }
                } else {
                    self.report_error(
                        format!("Expected tuple pattern, got {:?}", value_ty),
                        pattern.span
                    );
                }
            }
            ast::Pattern::Or(pats) => {
                // Or pattern: all branches must be compatible
                for pat in pats {
                    self.check_pattern(pat, value_ty, pat.span);
                }
            }
            ast::Pattern::Range { start: _, end: _, inclusive: _ } => {
                // Range pattern: start and end must be comparable
                if *value_ty != Type::Int && *value_ty != Type::Unknown {
                    self.report_error(
                        format!("Range pattern requires Int, got {:?}", value_ty),
                        pattern.span
                    );
                }
            }
            ast::Pattern::Array(pats) => {
                // Array pattern: all elements must match element type
                match value_ty {
                    Type::Named(name) if name.starts_with("Array") => {
                        for pat in pats {
                            self.check_pattern(pat, &Type::Unknown, pat.span);
                        }
                    }
                    _ => {
                        self.report_error(
                            format!("Expected array pattern, got {:?}", value_ty),
                            pattern.span
                        );
                    }
                }
            }
            ast::Pattern::Record { ty: _, fields, rest: _ } => {
                // Record pattern: verify fields exist (without type registry, skip validation)
                for field in fields {
                    if let Some(pat) = &field.pattern {
                        self.check_pattern(pat, &Type::Unknown, pat.span);
                    }
                }
            }
            ast::Pattern::Variant { ty: _, args } => {
                // Variant pattern: verify variant args (without ADT registry, skip validation)
                for pat in args {
                    self.check_pattern(pat, &Type::Unknown, pat.span);
                }
            }
            ast::Pattern::Ref(pat) => {
                // Reference pattern: unwrap reference type
                if let Type::Reference { inner, .. } = value_ty {
                    self.check_pattern(pat, inner, pattern.span);
                } else {
                    self.report_error(
                        format!("Expected reference pattern, got {:?}", value_ty),
                        pattern.span
                    );
                }
            }
        }
    }

    fn report_error(&mut self, message: String, span: Span) -> Type {
        self.errors.push(TypeError {
            message,
            span,
            context: self.context_stack.clone(),
        });
        Type::Unknown
    }

    pub fn get_var(&mut self, name: &str, is_ref: bool, usage_span: Span) -> Type {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(meta) = scope.vars.get_mut(name) {
                if meta.is_moved {
                    let move_line = meta.moved_at.map_or(0, |s| s.line);
                    let msg = format!("Use of moved value '{}'. It was moved at line {}.", name, move_line);
                    return self.report_error(msg, usage_span);
                }
                if meta.ty.ownership() == Ownership::Move && !is_ref {
                    meta.is_moved = true;
                    meta.moved_at = Some(usage_span);
                }
                return meta.ty.clone();
            }
        }
        self.report_error(format!("Undefined variable: {}", name), usage_span)
    }

    pub fn convert_type(&self, ast_ty: &ast::Type) -> Type {
        match ast_ty {
            ast::Type::Named(name) => match name.as_str() {
                "Int" => Type::Int,
                "Float" => Type::Float,
                "Bool" => Type::Bool,
                "Char" => Type::Char,
                "String" => Type::String,
                "Unit" => Type::Unit,
                _ => Type::Named(name.clone()),
            },
            ast::Type::Qualified(parts) => Type::Named(parts.join(".")),
            ast::Type::Generic { name, args } => {
                // Phase 6: Better generic handling - preserve type arguments
                let arg_strs: Vec<String> = args.iter()
                    .map(|arg| format!("{:?}", self.convert_type(arg)))
                    .collect();
                Type::Named(format!("{}<{}>", name, arg_strs.join(", ")))
            },
            ast::Type::Array { elem, size } => {
                // Phase 6: Track array sizes
                Type::Named(format!("[{}; {}]", elem_name(elem), size))
            },
            ast::Type::Tuple(tys) => {
                Type::Tuple(tys.iter().map(|t| self.convert_type(t)).collect())
            }
            ast::Type::Reference { is_mut, inner, region: _ } => Type::Reference {
                is_mut: *is_mut,
                inner: Box::new(self.convert_type(inner)),
            },
            ast::Type::Function { params, effects, ret } => {
                let converted_effects = effects.iter()
                    .filter_map(|eff| self.convert_effect(eff))
                    .collect();
                Type::Function {
                    params: params.iter().map(|t| self.convert_type(t)).collect(),
                    effects: converted_effects,
                    ret: Box::new(self.convert_type(ret)),
                }
            },
            ast::Type::DynTrait(name) => Type::Named(format!("dyn {}", name)),
        }
    }

    // Phase 6: Type Compatibility & Unification
    pub fn types_compatible(&self, expected: &Type, actual: &Type) -> bool {
        match (expected, actual) {
            // Same types are compatible
            (a, b) if a == b => true,
            // Unknown types are compatible with anything
            (Type::Unknown, _) | (_, Type::Unknown) => true,
            // Named types might be compatible via type definitions
            (Type::Named(exp_name), Type::Named(act_name)) => {
                exp_name == act_name || self.are_types_equivalent(exp_name, act_name)
            },
            // Tuples must have same length and compatible elements
            (Type::Tuple(exp_elems), Type::Tuple(act_elems)) => {
                exp_elems.len() == act_elems.len() &&
                exp_elems.iter().zip(act_elems.iter())
                    .all(|(e, a)| self.types_compatible(e, a))
            },
            // References with compatible inner types
            (Type::Reference { is_mut: e_mut, inner: e_inner },
             Type::Reference { is_mut: a_mut, inner: a_inner }) => {
                // Mutability must match exactly
                e_mut == a_mut && self.types_compatible(e_inner, a_inner)
            },
            // Function types with compatible signatures
            (Type::Function { params: e_params, ret: e_ret, .. },
             Type::Function { params: a_params, ret: a_ret, .. }) => {
                e_params.len() == a_params.len() &&
                e_params.iter().zip(a_params.iter())
                    .all(|(e, a)| self.types_compatible(e, a)) &&
                self.types_compatible(e_ret, a_ret)
            },
            // Different kinds are not compatible
            _ => false,
        }
    }

    // Check if two named types are equivalent (through type definitions)
    fn are_types_equivalent(&self, ty1: &str, ty2: &str) -> bool {
        // TODO: Phase 5 - Use type registry to check equivalence
        // For now, only exact name matches are equivalent
        ty1 == ty2
    }

    // Try to unify two types (for generics)
    pub fn unify_types(&self, ty1: &Type, ty2: &Type) -> Option<Type> {
        match (ty1, ty2) {
            // Same types unify to themselves
            (a, b) if a == b => Some(a.clone()),
            // Unknown unifies with anything
            (Type::Unknown, b) => Some(b.clone()),
            (a, Type::Unknown) => Some(a.clone()),
            // Tuples unify element-wise
            (Type::Tuple(elems1), Type::Tuple(elems2)) if elems1.len() == elems2.len() => {
                let unified: Option<Vec<_>> = elems1.iter().zip(elems2.iter())
                    .map(|(e1, e2)| self.unify_types(e1, e2))
                    .collect();
                unified.map(Type::Tuple)
            },
            // References unify if inner types unify and mutability matches
            (Type::Reference { is_mut: m1, inner: i1 },
             Type::Reference { is_mut: m2, inner: i2 }) if m1 == m2 => {
                self.unify_types(i1, i2).map(|inner| Type::Reference {
                    is_mut: *m1,
                    inner: Box::new(inner),
                })
            },
            // Function types unify if signatures match
            (Type::Function { params: p1, ret: r1, .. },
             Type::Function { params: p2, ret: r2, .. }) if p1.len() == p2.len() => {
                let unified_params: Option<Vec<_>> = p1.iter().zip(p2.iter())
                    .map(|(pp1, pp2)| self.unify_types(pp1, pp2))
                    .collect();
                let unified_ret = self.unify_types(r1, r2);
                match (unified_params, unified_ret) {
                    (Some(params), Some(ret)) => Some(Type::Function {
                        params,
                        effects: vec![],
                        ret: Box::new(ret),
                    }),
                    _ => None,
                }
            },
            // No unification possible
            _ => None,
        }
    }

    // Check if type can be assigned to expected type (subtyping)
    pub fn is_assignable(&self, expected: &Type, actual: &Type) -> bool {
        self.types_compatible(expected, actual)
    }

    pub fn check_stmt(&mut self, stmt: &Spanned<ast::Stmt>) {
        match &stmt.node {
            ast::Stmt::Let { pattern, is_mut: _, ty, value } => {
                let name = match &pattern.node {
                    ast::Pattern::Bind(n) => n.clone(),
                    _ => "<pattern>".into(),
                };
                self.context_stack.push(format!("In let binding for '{}'", name));
                let mut val_ty = self.infer_expr(value);

                if let Some(expected_ast_ty) = ty {
                    let expected_ty = self.convert_type(expected_ast_ty);
                    if expected_ty != val_ty && val_ty != Type::Unknown {
                        self.report_error(
                            format!("Type mismatch: expected {:?}, found {:?}", expected_ty, val_ty),
                            value.span
                        );
                    }
                    val_ty = expected_ty;
                }

                // Use check_pattern for comprehensive pattern matching
                self.check_pattern(pattern, &val_ty, pattern.span);
                self.context_stack.pop();
            }
            ast::Stmt::Expr(expr) => { self.infer_expr(expr); }
            ast::Stmt::Empty => {}
        }
    }

    pub fn infer_expr(&mut self, expr: &Spanned<ast::Expr>) -> Type {
        match &expr.node {
            ast::Expr::Error => Type::Unknown, // Silently propagate unknown to prevent cascades
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
                match op {
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
                }
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
                let _scrutinee_ty = self.infer_expr(scrutinee);

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
                let _iter_ty = self.infer_expr(iter);

                self.enter_scope();
                self.loop_depth += 1;

                // Bind pattern variable
                if let ast::Pattern::Bind(name) = &pattern.node {
                    self.insert_var(name.clone(), Type::Unknown, false, pattern.span);
                }

                let body_ty = self.infer_block(body);

                self.loop_depth -= 1;
                self.exit_scope();
                self.context_stack.pop();
                body_ty
            }
            ast::Expr::While { condition, body } => {
                self.context_stack.push("In while loop".into());
                let cond_ty = self.infer_expr(condition);

                if cond_ty != Type::Bool && cond_ty != Type::Unknown {
                    self.report_error("While condition must be Bool".into(), condition.span);
                }

                self.loop_depth += 1;
                let body_ty = self.infer_block(body);
                self.loop_depth -= 1;

                self.context_stack.pop();
                body_ty
            }
            ast::Expr::Loop { body } => {
                self.context_stack.push("In loop".into());
                self.loop_depth += 1;
                let body_ty = self.infer_block(body);
                self.loop_depth -= 1;
                self.context_stack.pop();
                body_ty
            }
            ast::Expr::Break(_break_val) => {
                if self.loop_depth == 0 {
                    self.report_error("Break outside of loop".into(), expr.span);
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
                let _ex_ty = self.infer_expr(expr_val);
                Type::Never
            }
            // Phase 2: Complex Expressions
            ast::Expr::Call { callee, args } => {
                let callee_ty = self.infer_expr(callee);
                if let Type::Function { params, effects, ret } = callee_ty {
                    if args.len() != params.len() {
                        self.report_error(
                            format!("Expected {} arguments, got {}", params.len(), args.len()),
                            expr.span
                        );
                    }
                    for (i, (arg, param_ty)) in args.iter().zip(params.iter()).enumerate() {
                        let arg_ty = self.infer_expr(arg);
                        if arg_ty != *param_ty && arg_ty != Type::Unknown && *param_ty != Type::Unknown {
                            self.report_error(
                                format!("Argument {} type mismatch: expected {:?}, got {:?}", i, param_ty, arg_ty),
                                arg.span
                            );
                        }
                    }

                    // Phase 10: Check effect compatibility
                    for effect in &effects {
                        self.add_required_effect(effect.clone());
                    }

                    *ret
                } else {
                    self.report_error("Callee must be function type".into(), callee.span)
                }
            }
            ast::Expr::Tuple(elems) => {
                let elem_types: Vec<_> = elems.iter().map(|e| self.infer_expr(e)).collect();
                Type::Tuple(elem_types)
            }
            ast::Expr::Array(elems) => {
                if elems.is_empty() {
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
                }
            }
            ast::Expr::ArrayRepeat { elem, count } => {
                let _elem_ty = self.infer_expr(elem);
                let count_ty = self.infer_expr(count);
                if count_ty != Type::Int && count_ty != Type::Unknown {
                    self.report_error("Array repeat count must be Int".into(), count.span);
                }
                Type::Named("Array<Unknown>".into())
            }
            ast::Expr::Index { base, index } => {
                let base_ty = self.infer_expr(base);
                let index_ty = self.infer_expr(index);

                if index_ty != Type::Int && index_ty != Type::Unknown {
                    self.report_error("Index must be Int".into(), index.span);
                }

                match base_ty {
                    Type::Named(ref name) if name.starts_with("Array") => Type::Unknown,
                    Type::Tuple(ref elems) => {
                        if elems.is_empty() { Type::Unknown }
                        else { elems[0].clone() }
                    }
                    _ => self.report_error("Can only index arrays or tuples".into(), base.span),
                }
            }
            ast::Expr::FieldAccess { base, field: _ } => {
                let _base_ty = self.infer_expr(base);
                Type::Unknown // Would need record type registry
            }
            // Phase 3: Advanced Expressions (updated Phase 10: Effect Inference)
            ast::Expr::Closure { is_move: _, params, effects, ret_ty, body } => {
                self.enter_scope();

                // Phase 10: Set declared effects for the closure
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
                if let Some(expected_ret) = ret_ty {
                    let expected = self.convert_type(expected_ret);
                    if expected != body_ty && expected != Type::Unknown && body_ty != Type::Unknown {
                        self.report_error("Closure body type mismatch".into(), body.span);
                    }
                }

                // Phase 10: Infer closure effects and clear context
                let inferred_effects = self.infer_closure_effects(&self.fn_required_effects);
                self.fn_declared_effects.clear();
                self.fn_required_effects = saved_required;

                self.exit_scope();

                // Phase 10: Return function type with inferred effects
                Type::Function {
                    params: vec![],
                    effects: inferred_effects,
                    ret: Box::new(body_ty),
                }
            }
            ast::Expr::Record { ty, fields } => {
                for field in fields {
                    if let Some(value) = &field.value {
                        let _field_ty = self.infer_expr(value);
                    }
                }
                Type::Named(ty.join("."))
            }
            ast::Expr::Variant { ty, args } => {
                for arg in args {
                    let _arg_ty = self.infer_expr(arg);
                }
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
            ast::Expr::Question(_inner) => {
                Type::Unknown // Error propagation, would need context
            }
            ast::Expr::Await(_inner) => {
                Type::Unknown // Async, would need effect tracking
            }
            ast::Expr::Scope { label, options, body } => {
                self.context_stack.push(format!("In scope{}",
                    label.as_ref().map(|l| format!(" '{}'", l)).unwrap_or_default()));

                // Phase 11: Validate scope with context expression
                if let Some(opts) = options {
                    let opts_ty = self.infer_expr(opts);
                    if !self.validate_scope_with_context(&opts_ty) {
                        self.report_error("Scope 'with' expression must provide valid context".into(), opts.span);
                    }
                }

                // Push new scope effect context
                self.effect_stack.push(self.active_effects.clone());
                let body_ty = self.infer_block(body);
                // Pop scope effect context
                self.effect_stack.pop();

                self.context_stack.pop();
                body_ty
            }
            ast::Expr::Region { label, body } => {
                self.context_stack.push(format!("In region{}",
                    label.as_ref().map(|l| format!(" '{}'", l)).unwrap_or_default()));

                // Phase 13: Push region for escape analysis
                let region_name = label.as_ref()
                    .map(|l| l.clone())
                    .unwrap_or_else(|| format!("region_{}", self.region_stack.len()));
                self.push_region(region_name.clone());

                // Push new region effect context
                self.effect_stack.push(self.active_effects.clone());
                let body_ty = self.infer_block(body);
                // Pop region effect context
                self.effect_stack.pop();

                // Phase 13: Pop region and validate no escapes
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
                                // Phase 11: For parameterized exceptions, bind with correct type
                                let var_ty = match &arm.kind {
                                    ast::HandleArmKind::Exn => Type::Unknown, // Would need exn parameter type
                                    _ => Type::Unknown,
                                };
                                self.insert_var(name.clone(), var_ty, false, pat.span);
                            }
                            _ => {}
                        }
                    }

                    // Phase 11: Register handled effect based on kind
                    match &arm.kind {
                        ast::HandleArmKind::Return => {
                            // Return handler doesn't remove an effect
                        }
                        ast::HandleArmKind::Exn => {
                            // Exception handler - Phase 11: mark exn as handled
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

                // Phase 11: Compute unhandled effects and propagate them
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

    fn infer_block(&mut self, block: &ast::Block) -> Type {
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