use crate::ast;
use crate::ast::Span;
use crate::ty::Type;
use super::*;

impl Checker {

    pub fn validate_record_exhaustiveness(&mut self,
        type_name: &str,
        provided_fields: &[String],
        required_fields: &[String],
        span: Span
    ) -> bool {
        let mut all_present = true;
        for required in required_fields {
            if !provided_fields.contains(required) {
                self.report_error(
                    format!("Record '{}' missing required field '{}'", type_name, required),
                    span
                );
                all_present = false;
            }
        }
        all_present
    }

    pub fn validate_record_fields(&mut self,
        _type_name: &str,
        field_types: &[(String, Type)],
        provided_values: &[(String, Type)],
        span: Span
    ) -> bool {
        let mut all_valid = true;
        for (field_name, provided_ty) in provided_values {
            if let Some((_, expected_ty)) = field_types.iter().find(|(n, _)| n == field_name) {
                if expected_ty != provided_ty && expected_ty != &Type::Unknown && provided_ty != &Type::Unknown {
                    self.report_error(
                        format!("Record field '{}' type mismatch: expected {:?}, got {:?}",
                                field_name, expected_ty, provided_ty),
                        span
                    );
                    all_valid = false;
                }
            }
        }
        all_valid
    }

    pub fn check_record_initialization(&mut self,
        type_name: &str,
        field_types: &[(String, Type)],
        provided_fields: &[String],
        provided_values: &[(String, Type)],
        span: Span
    ) -> bool {
        let required_field_names: Vec<String> = field_types.iter().map(|(n, _)| n.clone()).collect();
        let exhaustive = self.validate_record_exhaustiveness(type_name, provided_fields, &required_field_names, span);
        let types_valid = self.validate_record_fields(type_name, field_types, provided_values, span);

        exhaustive && types_valid
    }

    pub fn validate_variant_arguments(&mut self,
        variant_name: &str,
        expected_arg_count: usize,
        provided_arg_count: usize,
        span: Span
    ) -> bool {
        if expected_arg_count != provided_arg_count {
            self.report_error(
                format!("Variant '{}' expects {} arguments, got {}",
                        variant_name, expected_arg_count, provided_arg_count),
                span
            );
            return false;
        }
        true
    }

    pub fn validate_variant_argument_types(&mut self,
        variant_name: &str,
        expected_types: &[Type],
        provided_types: &[Type],
        span: Span
    ) -> bool {
        let mut all_valid = true;
        for (i, (expected, provided)) in expected_types.iter().zip(provided_types.iter()).enumerate() {
            if expected != provided && expected != &Type::Unknown && provided != &Type::Unknown {
                self.report_error(
                    format!("Variant '{}' argument {} type mismatch: expected {:?}, got {:?}",
                            variant_name, i, expected, provided),
                    span
                );
                all_valid = false;
            }
        }
        all_valid
    }

    pub fn check_variant_construction(&mut self,
        variant_name: &str,
        expected_arg_types: &[Type],
        provided_arg_types: &[Type],
        span: Span
    ) -> bool {
        let count_valid = self.validate_variant_arguments(
            variant_name,
            expected_arg_types.len(),
            provided_arg_types.len(),
            span
        );

        if !count_valid {
            return false;
        }

        self.validate_variant_argument_types(variant_name, expected_arg_types, provided_arg_types, span)
    }

    pub fn get_record_field_types(&self, type_name: &str) -> Option<Vec<(String, Type)>> {
        if let Some(body) = self.type_registry.get(type_name) {
            match body {
                ast::TypeBody::Record(fields) => {
                    let field_types = fields.iter()
                        .map(|f| (f.name.clone(), self.convert_type(&f.ty)))
                        .collect();
                    return Some(field_types);
                }
                _ => {}
            }
        }
        None
    }

    pub fn get_variant_arg_types(&self, type_name: &str, variant_name: &str) -> Option<Vec<Type>> {
        if let Some(body) = self.type_registry.get(type_name) {
            match body {
                ast::TypeBody::Variant(variants) => {
                    for variant in variants {
                        match variant {
                            ast::VariantCase::Unit(name) if name == variant_name => {
                                return Some(vec![]);
                            }
                            ast::VariantCase::Tuple(name, types) if name == variant_name => {
                                let arg_types = types.iter()
                                    .map(|t| self.convert_type(t))
                                    .collect();
                                return Some(arg_types);
                            }
                            ast::VariantCase::Record(name, fields) if name == variant_name => {
                                let arg_types = fields.iter()
                                    .map(|f| self.convert_type(&f.ty))
                                    .collect();
                                return Some(arg_types);
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        None
    }
}
