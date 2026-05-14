use std::collections::HashMap;
use crate::ast;
use crate::ast::{Span, Spanned};
use crate::ty::{Ownership, Type};
use super::*;

impl Checker {

    pub fn validate_string_interpolation(&mut self,
        identifiers: &[String],
        span: Span
    ) -> bool {
        let mut all_valid = true;
        for ident in identifiers {
            if self.get_var(ident, false, span) == Type::Unknown {
                self.report_error(
                    format!("String interpolation references undefined variable '{}'", ident),
                    span
                );
                all_valid = false;
            }
        }
        all_valid
    }

    pub fn extract_interpolation_identifiers(&self, parts: &[ast::StringPart]) -> Vec<String> {
        let mut identifiers = Vec::new();
        for part in parts {
            if let ast::StringPart::Interp(segments) = part {
                // Each segment is part of the path
                //  e.g., "user", "name" for {user.name}
                if !segments.is_empty() {
                    identifiers.push(segments[0].clone()); // Root identifier
                }
            }
        }
        identifiers
    }

    pub fn check_interpolation_paths(&mut self,
        parts: &[ast::StringPart],
        span: Span
    ) -> bool {
        // Validate full paths in interpolations 
        //  e.g., {user.name}
        let mut all_valid = true;
        for part in parts {
            if let ast::StringPart::Interp(segments) = part {
                if segments.is_empty() {
                    continue;
                }

                // Check root identifier — use is_ref=true to avoid marking as moved
                let root = &segments[0];
                let current_ty = self.get_var(root, true, span);

                if current_ty == Type::Unknown {
                    self.report_error(
                        format!("Interpolation references undefined identifier '{}'", root),
                        span
                    );
                    all_valid = false;
                    continue;
                }

                // Check field accesses (.field notation) using get_field_type
                // Only errors if a field is explicitly not found on a registered type
                let mut current_ty = current_ty;
                for field in &segments[1..] {
                    if let Type::Named(type_name) = &current_ty.clone() {
                        if let Some(field_ty) = self.get_field_type(type_name, field) {
                            current_ty = field_ty;
                        } else if self.type_registry.contains_key(type_name.as_str()) {
                            // Type is registered but field is missing — real error
                            self.report_error(
                                format!("Field '{}' not found in type '{}'", field, type_name),
                                span,
                            );
                            all_valid = false;
                            break;
                        } else {
                            // Type not registered — skip validation
                            break;
                        }
                    } else {
                        break;
                    }
                }
            }
        }
        all_valid
    }

    pub fn validate_interpolation_types(&mut self,
        parts: &[ast::StringPart],
        span: Span
    ) -> bool {
        let mut all_valid = true;
        for part in parts {
            if let ast::StringPart::Interp(segments) = part {
                if segments.is_empty() {
                    continue;
                }

                let root = &segments[0];
                // use is_ref=true — validation is read-only, should not trigger moves
                let mut ty = self.get_var(root, true, span);

                if ty == Type::Unknown {
                    all_valid = false;
                    continue;
                }

                // Traverse field accesses
                for field in &segments[1..] {
                    ty = self.resolve_field_access(&ty, field, span);
                    if ty == Type::Unknown {
                        all_valid = false;
                        break;
                    }
                }

                // Primitives always implement Show; other Named types need explicit impl
                let is_showable = match &ty {
                    Type::Int | Type::Float | Type::Bool | Type::Char
                    | Type::String | Type::Unit => true,
                    _ => self.type_implements_show(&ty),
                };

                if all_valid && !is_showable {
                    let type_name = match &ty {
                        Type::Named(n) => n.clone(),
                        Type::Never => "Never".into(),
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

    pub fn validate_string_literal(&mut self,
        _value: &str,
        is_interpolated: bool,
        parts: Option<&[ast::StringPart]>,
        span: Span
    ) -> bool {
        if !is_interpolated {
            return true;
        }

        if let Some(interp_parts) = parts {
            let paths_valid = self.check_interpolation_paths(interp_parts, span);
            let types_valid = self.validate_interpolation_types(interp_parts, span);
            paths_valid && types_valid

        } else {
            true
        }
    }

    pub fn count_interpolations(&self, parts: &[ast::StringPart]) -> usize {
        parts.iter().filter(|p| matches!(p, ast::StringPart::Interp(_))).count()
    }

    pub fn has_interpolations(&self, parts: &[ast::StringPart]) -> bool {
        parts.iter().any(|p| matches!(p, ast::StringPart::Interp(_)))
    }
}
