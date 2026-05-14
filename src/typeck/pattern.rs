use std::collections::HashMap;
use crate::ast;
use crate::ast::{Span, Spanned};
use crate::ty::{Ownership, Type};
use super::*;

impl Checker {

    // Pattern Matching (Exhaustiveness & Unreachability)

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
        if existing_patterns.contains(&"_") {
            return true;
        }
        if existing_patterns.iter().any(|p| *p == new_pattern) {
            return true;
        }
        false
    }

    pub fn validate_match_exhaustiveness(&mut self, scrutinee_type: &Type, patterns: &[String], span: Span) -> bool {
        match scrutinee_type {
            Type::Named(name) if name.contains("Variant") || name.contains("Enum") => {
                if patterns.contains(&"_".to_string()) {
                    return true;
                }
                self.report_error("Non-exhaustive patterns in match".into(), span);
                false
            }
            _ => {
                patterns.contains(&"_".to_string())
            }
        }
    }

    pub fn detect_unreachable_patterns(&mut self, patterns: &[String]) -> Vec<usize> {
        let mut unreachable = Vec::new();
        let mut covered = Vec::new();

        for (i, pattern) in patterns.iter().enumerate() {
            if pattern == "_" {
                for j in (i + 1)..patterns.len() {
                    unreachable.push(j);
                }
                break;
            }
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
        patterns.contains(&"_".to_string())
    }

    // Collect variant case names from match arms

    pub fn collect_arm_patterns(arms: &[ast::MatchArm]) -> (Vec<String>, bool) {
        let mut covered = Vec::new();
        let mut has_wildcard = false;

        for arm in arms {
            match &arm.pattern.node {
                ast::Pattern::Wildcard => {
                    has_wildcard = true;
                }
                ast::Pattern::Bind(_) => {
                    // A bare binding like `x => body` covers all cases
                    has_wildcard = true;
                }
                ast::Pattern::Variant { ty, .. } => {
                    // Extract the last component of the type path (the variant case name)
                    if let Some(case_name) = ty.last() {
                        covered.push(case_name.clone());
                    }
                }
                ast::Pattern::Or(pats) => {
                    for pat in pats {
                        if let ast::Pattern::Variant { ty, .. } = &pat.node {
                            if let Some(case_name) = ty.last() {
                                covered.push(case_name.clone());
                            }
                        }
                    }
                }
                _ => {
                    // Other patterns (Literal, Bind, etc.) don't count as variant coverage
                }
            }
        }
        (covered, has_wildcard)
    }

    pub fn check_variant_exhaustiveness(
        &mut self,
        type_name: &str,
        covered: &[String],
        has_wildcard: bool,
        span: Span,
    ) -> bool {
        if has_wildcard {
            return true;
        }
        if let Some(required_cases) = self.variant_registry.get(type_name).cloned() {
            let mut all_covered = true;
            for case in required_cases {
                if !covered.contains(&case) {
                    self.report_error(
                        format!("Non-exhaustive pattern: variant case '{}' not covered in match on '{}'", case, type_name),
                        span
                    );
                    all_covered = false;
                }
            }
            all_covered
        } else {
            // Unknown type
            true
        }
    }

    pub fn type_implements_show(&self, ty: &Type) -> bool {
        // Handle reference types with auto-deref
        let check_type = match ty {
            Type::Reference { inner, .. } => inner.as_ref(),
            t => t,
        };

        let type_name = match check_type {
            Type::Named(n) => n.clone(),
            Type::Int => "Int".into(),
            Type::Float => "Float".into(),
            Type::Bool => "Bool".into(),
            Type::Char => "Char".into(),
            Type::String => "String".into(),
            Type::Unit => "Unit".into(),
            _ => return false,
        };

        // Check direct implementation
        if self.impl_registry.get(&(type_name.clone(), "Show".into())).copied().unwrap_or(false) {
            return true;
        }

        // Check trait bounds for generic types
        if self.trait_bounds.get(&type_name).map(|bounds| {
            bounds.iter().any(|b| b == "Show")
        }).unwrap_or(false) {
            return true;
        }

        false
    }

    pub fn check_string_interpolation(&mut self, parts: &[ast::StringPart], span: Span) -> bool {
        let mut all_valid = true;

        for part in parts {
            if let ast::StringPart::Interp(path) = part {
                if path.is_empty() {
                    continue;
                }

                // Resolve the base identifier (search all scopes)
                let base_name = &path[0];
                let base_var = self.resolve_var_in_scopes(base_name);

                if base_var.is_none() {
                    self.report_error(
                        format!("Undefined variable '{}' in string interpolation", base_name),
                        span
                    );
                    all_valid = false;
                    continue;
                }

                let mut current_type = base_var.unwrap();

                // Handle auto-deref for references
                if let Type::Reference { inner, .. } = &current_type {
                    current_type = inner.as_ref().clone();
                }

                // Resolve field accesses in the path
                for field_name in &path[1..] {
                    match &current_type {
                        Type::Named(type_name) => {

                            if let Some(field_type) = self.get_field_type(type_name, field_name) {
                                current_type = field_type;
                                if let Type::Reference { inner, .. } = &current_type {
                                    current_type = inner.as_ref().clone();
                                }
                            } else {
                                self.report_error(
                                    format!("Field '{}' not found in type '{}'", field_name, type_name),
                                    span
                                );
                                all_valid = false;
                                break;
                            }
                        }
                        _ => {
                            self.report_error(
                                format!("Cannot access field '{}' on type {:?}", field_name, current_type),
                                span
                            );
                            all_valid = false;
                            break;
                        }
                    }
                }

                // Check if the final type implements Show
                if all_valid && !self.type_implements_show(&current_type) {
                    let type_name = match &current_type {
                        Type::Named(n) => n.clone(),
                        Type::Int => "Int".into(),
                        Type::Float => "Float".into(),
                        Type::Bool => "Bool".into(),
                        Type::Char => "Char".into(),
                        Type::String => "String".into(),
                        Type::Unit => "Unit".into(),
                        _ => "Unknown".into(),
                    };

                    self.report_error(
                        format!("Type '{}' does not implement Show trait required for string interpolation", type_name),
                        span
                    );
                    all_valid = false;
                }
            }
        }
        all_valid
    }

    pub fn infer_literal(&mut self, lit: &ast::Literal, span: Span) -> Type {
        match lit {
            ast::Literal::Int(_) => Type::Int,
            ast::Literal::Float(_) => Type::Float,
            ast::Literal::Bool(_) => Type::Bool,
            ast::Literal::Char(_) => Type::Char,
            ast::Literal::String(_) => Type::String,
            ast::Literal::StringInterp(parts) => {
                self.check_string_interpolation(parts, span);
                Type::String
            }
            ast::Literal::Unit => Type::Unit,
        }
    }

    // Pattern Matching

    pub fn check_pattern(&mut self, pattern: &Spanned<ast::Pattern>, value_ty: &Type, _pattern_span: Span) {
        match &pattern.node {
            ast::Pattern::Bind(name) => {
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
                Type::Generic {
                    name: name.clone(),
                    args: args.iter().map(|arg| self.convert_type(arg)).collect(),
                }
            },
            ast::Type::Array { elem, size } => {
                // Track array sizes
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
}
