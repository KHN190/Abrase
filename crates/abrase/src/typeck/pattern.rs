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

    // Whether the match arms cover both Ok and Err variants (treat as exn-handler).
    pub fn arms_cover_ok_err(arms: &[ast::MatchArm]) -> bool {
        let mut has_ok = false;
        let mut has_err = false;
        for arm in arms {
            if let ast::Pattern::Variant { ty, .. } = &arm.pattern.node {
                match ty.last().map(String::as_str) {
                    Some("Ok") => has_ok = true,
                    Some("Err") => has_err = true,
                    _ => {}
                }
            }
        }
        has_ok && has_err
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

        if matches!(
            check_type,
            Type::Int | Type::Float | Type::Bool | Type::Char | Type::String | Type::Unit
        ) {
            return true;
        }

        let type_name = match check_type {
            Type::Named(n) => n.clone(),
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
                // Ident in pattern position: if it matches a variant case, treat as nullary variant, not binding.
                if self.lookup_variant_constructor(name).is_some() {
                    // Nullary variant: no binding, type compatibility checked in exhaustiveness pass.
                    return;
                }
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
                if let Type::Tuple(elem_types) = value_ty {
                    let rest_at = pats.iter().position(|p| matches!(p.node, ast::Pattern::Rest));
                    match rest_at {
                        None => {
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
                        }
                        Some(i) => {
                            let head = &pats[..i];
                            let tail = &pats[i+1..];
                            if head.len() + tail.len() > elem_types.len() {
                                self.report_error(
                                    format!("Tuple pattern too long: {} fixed elements + .. cannot fit in {}-tuple",
                                        head.len() + tail.len(), elem_types.len()),
                                    pattern.span,
                                );
                            }
                            for (pat, ty) in head.iter().zip(elem_types.iter()) {
                                self.check_pattern(pat, ty, pat.span);
                            }
                            let tail_start = elem_types.len().saturating_sub(tail.len());
                            for (pat, ty) in tail.iter().zip(elem_types[tail_start..].iter()) {
                                self.check_pattern(pat, ty, pat.span);
                            }
                        }
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
                    Type::Generic { name, args } if name == "Array" => {
                        let elem_ty = args.get(0).cloned().unwrap_or(Type::Unknown);
                        for pat in pats {
                            self.check_pattern(pat, &elem_ty, pat.span);
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
            ast::Pattern::Record { ty, fields, rest } => {
                let type_name = match (ty.last(), value_ty) {
                    (Some(n), _) => Some(n.clone()),
                    (None, Type::Named(n)) => Some(n.clone()),
                    _ => None,
                };
                let expected_fields: Vec<(String, Type)> = type_name.as_ref()
                    .and_then(|n| self.type_registry.get(n))
                    .and_then(|body| match body {
                        ast::TypeBody::Record(fs) => Some(
                            fs.iter().map(|f| (f.name.clone(), self.convert_type(&f.ty))).collect()
                        ),
                        _ => None,
                    })
                    .unwrap_or_default();
                if !expected_fields.is_empty() && !*rest {
                    let listed: std::collections::HashSet<&str> =
                        fields.iter().map(|f| f.name.as_str()).collect();
                    let missing: Vec<&str> = expected_fields.iter()
                        .filter(|(n, _)| !listed.contains(n.as_str()))
                        .map(|(n, _)| n.as_str())
                        .collect();
                    if !missing.is_empty() {
                        self.report_error(
                            format!("Record pattern missing fields: {} (use `..` to ignore)", missing.join(", ")),
                            pattern.span,
                        );
                    }
                }
                for field in fields {
                    let field_ty = expected_fields.iter()
                        .find(|(n, _)| n == &field.name)
                        .map(|(_, t)| t.clone())
                        .unwrap_or(Type::Unknown);
                    if let Some(pat) = &field.pattern {
                        self.check_pattern(pat, &field_ty, pat.span);
                    } else {
                        self.insert_var(field.name.clone(), field_ty, false, pattern.span);
                    }
                }
            }
            ast::Pattern::Variant { ty, args } => {
                // Look up payload types and use as arg pattern types to avoid Unknown-as-Move.
                debug_assert!(!ty.is_empty(), "variant pattern with empty type path");
                let case_name = match ty.last().cloned() {
                    Some(n) => n,
                    None => {
                        self.report_error("variant pattern has empty type path".into(), pattern.span);
                        return;
                    }
                };
                let lookup = self.lookup_variant_constructor(&case_name);
                let was_resolved = lookup.is_some();
                let payload_tys: Vec<Type> = match lookup {
                    Some(Type::Function { params, .. }) => params,
                    _ => Vec::new(),
                };
                if was_resolved && args.len() != payload_tys.len() {
                    self.report_error(
                        format!(
                            "variant pattern '{}' expects {} arg(s), got {}",
                            case_name, payload_tys.len(), args.len()
                        ),
                        pattern.span,
                    );
                }
                if was_resolved && case_name != "Shared" {
                    let scrutinee_type_name = match value_ty {
                        Type::Named(n) => Some(n.clone()),
                        Type::Generic { name, .. } => Some(name.clone()),
                        _ => None,
                    };
                    if let Some(stn) = scrutinee_type_name {
                        let case_ok = self.variant_registry.get(&stn)
                            .map(|cases| cases.iter().any(|c| c == &case_name))
                            .unwrap_or(true); // unknown type — let other checks surface it
                        if !case_ok {
                            self.report_error(
                                format!(
                                    "variant pattern '{}' does not belong to type '{}'",
                                    case_name, stn
                                ),
                                pattern.span,
                            );
                        }
                    }
                }
                for (i, pat) in args.iter().enumerate() {
                    let arg_ty = payload_tys.get(i).cloned().unwrap_or(Type::Unknown);
                    self.check_pattern(pat, &arg_ty, pat.span);
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
            ast::Pattern::Rest => {}
        }
    }

    pub fn peek_var(&self, name: &str) -> Option<Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(meta) = scope.vars.get(name) {
                return Some(meta.ty.clone());
            }
        }
        None
    }

    pub fn peek_var_is_mut(&self, name: &str) -> Option<bool> {
        for scope in self.scopes.iter().rev() {
            if let Some(meta) = scope.vars.get(name) {
                return Some(meta.is_mut);
            }
        }
        None
    }

    pub fn get_var(&mut self, name: &str, is_ref: bool, usage_span: Span) -> Type {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(meta) = scope.vars.get_mut(name) {
                if meta.is_moved {
                    let move_line = meta.moved_at.map_or(0, |s| s.line);
                    let msg = format!("Use of moved value '{}'. It was moved at line {}.", name, move_line);
                    return self.report_error(msg, usage_span);
                }
                if !matches!(&meta.ty, Type::Function { .. }) &&
                   meta.ty.ownership() == Ownership::Move && !is_ref {
                    meta.is_moved = true;
                    meta.moved_at = Some(usage_span);
                }
                return meta.ty.clone();
            }
        }
        if let Some(ty) = self.lookup_variant_constructor(name) {
            return ty;
        }
        if let Some(ty) = self.get_const(name) {
            return ty;
        }
        if let Some((module_path, original)) = self.get_imported_name(name) {
            if let Some(ty) = self.lookup_module_item(&module_path, &original) {
                if self.is_accessible(&original, &module_path) {
                    return ty;
                }
                return self.report_error(
                    format!("'{}' is private in module {}; cannot import",
                        original, module_path.join(".")),
                    usage_span,
                );
            }
        }
        self.report_error(format!("Undefined variable: {}", name), usage_span)
    }

    pub(super) fn lookup_variant_constructor(&self, case_name: &str) -> Option<Type> {
        // Host builtin `Shared(v)`: codegen recognises call, typeck needs entry for identifier resolution.
        if case_name == "Shared" {
            return Some(Type::Function {
                params: vec![Type::Unknown],
                effects: vec![],
                ret: Box::new(Type::Shared {
                    inner: Box::new(Type::Unknown),
                    region: None,
                }),
            });
        }
        // Reverse-search variant_registry for the type that owns this case.
        for (type_name, cases) in self.variant_registry.iter() {
            if !cases.iter().any(|c| c == case_name) { continue; }
            let body = self.type_registry.get(type_name)?;
            let ast::TypeBody::Variant(variant_cases) = body else { continue; };
            let case = variant_cases.iter().find(|c| match c {
                ast::VariantCase::Unit(n)      => n == case_name,
                ast::VariantCase::Tuple(n, _)  => n == case_name,
                ast::VariantCase::Record(n, _) => n == case_name,
            })?;
            let owning_ty = Type::Named(type_name.clone());
            return Some(match case {
                ast::VariantCase::Unit(_) => owning_ty,
                ast::VariantCase::Tuple(_, payload_tys) => Type::Function {
                    params: payload_tys.iter().map(|t| self.convert_type(t)).collect(),
                    effects: vec![],
                    ret: Box::new(owning_ty),
                },
                ast::VariantCase::Record(_, fields) => Type::Function {
                    params: fields.iter().map(|f| self.convert_type(&f.ty)).collect(),
                    effects: vec![],
                    ret: Box::new(owning_ty),
                },
            });
        }
        None
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
                _ => {
                    // Check if it's a registered type alias
                    if let Some(resolved) = self.type_alias_registry.get(name) {
                        resolved.clone()
                    } else {
                        Type::Named(name.clone())
                    }
                },
            },
            ast::Type::Qualified(parts) => Type::Named(parts.join(".")),
            ast::Type::Generic { name, args } => {
                if name == "Shared" && args.len() == 1 {
                    return Type::Shared {
                        inner: Box::new(self.convert_type(&args[0])),
                        region: None,
                    };
                }
                Type::Generic {
                    name: name.clone(),
                    args: args.iter().map(|arg| self.convert_type(arg)).collect(),
                }
            },
            ast::Type::Array { elem, .. } => {
                Type::Generic {
                    name: "Array".into(),
                    args: vec![self.convert_type(elem)],
                }
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
        }
    }
}
