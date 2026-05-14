use crate::ty::Type;
use super::*;

impl Checker {

    // Generics & Trait Constraints

    pub fn register_trait(&mut self, trait_name: String, methods: Vec<String>) {
        self.trait_registry.insert(trait_name, methods);
    }

    pub fn get_trait(&self, trait_name: &str) -> Option<Vec<String>> {
        self.trait_registry.get(trait_name).cloned()
    }

    pub fn register_impl(&mut self, type_name: &str, trait_name: &str) {
        self.impl_registry.insert((type_name.to_string(), trait_name.to_string()), true);
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
        if let Some(expected_params) = self.get_generic_params(type_name) {
            expected_params.len() == type_args.len()
        } else {
            true
        }
    }

    pub fn check_all_trait_bounds(&self, fn_name: &str, type_args: &[(String, Type)]) -> bool {
        if let Some(params) = self.get_generic_params(fn_name) {
            for (param_name, arg_type) in type_args {
                // skip still-abstract generic type variables
                if let Type::Named(n) = arg_type {
                    if params.contains(n) { continue; }
                }
                if let Some(bounds) = self.get_trait_bounds(param_name) {
                    let type_str = Self::type_name_for_bound(arg_type);
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

    fn type_name_for_bound(ty: &Type) -> String {
        match ty {
            Type::Int    => "Int".into(),
            Type::Float  => "Float".into(),
            Type::Bool   => "Bool".into(),
            Type::Char   => "Char".into(),
            Type::String => "String".into(),
            Type::Unit   => "Unit".into(),
            Type::Named(n) => n.clone(),
            _ => format!("{:?}", ty),
        }
    }

    pub fn enforce_where_clause(
        &mut self,
        fn_name: &str,
        generics: &[crate::ast::GenericParam],
        where_clause: &[crate::ast::WhereBound],
        type_args: &[(String, Type)],
        span: crate::ast::Span,
    ) {
        let param_names: Vec<String> = generics.iter().map(|g| g.name.clone()).collect();
        if !param_names.is_empty() {
            self.register_generic_params(fn_name.into(), param_names.clone());
        }

        for bound in where_clause {
            if let crate::ast::Type::Named(var) = &bound.ty {
                for trait_path in &bound.bounds {
                    let trait_name = trait_path.last().cloned().unwrap_or_default();
                    if !trait_name.is_empty() {
                        self.register_trait_bound(var.clone(), trait_name);
                    }
                }
            }
        }

        for (param_name, actual_type) in type_args {
            if let Type::Named(n) = actual_type {
                if param_names.contains(n) { continue; } // still abstract
            }
            if let Some(bounds) = self.get_trait_bounds(param_name) {
                let type_str = Self::type_name_for_bound(actual_type);
                for trait_name in &bounds {
                    if !self.has_impl(&type_str, trait_name) {
                        self.report_error(
                            format!("Type '{}' does not satisfy bound '{}' required for type parameter '{}'",
                                type_str, trait_name, param_name),
                            span,
                        );
                    }
                }
            }
        }
    }
}
