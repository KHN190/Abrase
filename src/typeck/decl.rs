use std::collections::HashMap;
use crate::ast;
use crate::ast::{Span, Spanned};
use crate::ty::{Ownership, Type};
use super::*;

impl Checker {

    pub fn check_program(&mut self, decls: &[ast::Decl]) {
        // Pass 1: Collect signatures 
        //  register all types, functions, effects, traits, imports
        for decl in decls {
            self.check_decl_signature(decl);
        }

        // Pass 2: Check bodies 
        //  type-check function bodies, impl methods, const expressions
        for decl in decls {
            self.check_decl_body(decl);
        }
    }

    fn check_decl_signature(&mut self, decl: &ast::Decl) {
        match decl {
            ast::Decl::Fn(fn_decl) => {
                let params: Vec<Type> = fn_decl.params.iter()
                    .filter_map(|p| match p {
                        ast::Param::Named { ty, .. } => Some(self.convert_type(ty)),
                        _ => None,
                    })
                    .collect();
                let effects: Vec<crate::ty::Effect> = fn_decl.effects.iter()
                    .filter_map(|eff| self.convert_effect(eff))
                    .collect();
                let ret = fn_decl.return_type.as_ref()
                    .map(|t| Box::new(self.convert_type(t)))
                    .unwrap_or_else(|| Box::new(Type::Unit));

                let fn_type = Type::Function { params, effects, ret };
                let module_path = self.current_module.clone();
                self.register_module_item(&module_path, fn_decl.name.clone(), fn_type.clone());
                self.insert_var(fn_decl.name.clone(), fn_type, false, ast::Span { line: 0, col: 0 });

                if fn_decl.is_pub {
                    self.mark_public(fn_decl.name.clone());
                }
            },

            ast::Decl::Type { name, body, is_pub, ownership, .. } => {
                let module_path = self.current_module.clone();
                self.register_module_item(&module_path, name.clone(), Type::Named(name.clone()));
                self.register_type(name.clone(), body.clone());

                if *is_pub {
                    self.mark_public(name.clone());
                }
                if let Some(own_attr) = ownership {
                    let ownership = match own_attr {
                        ast::OwnershipAttr::Copy => Ownership::Copy,
                        ast::OwnershipAttr::Move => Ownership::Move,
                        ast::OwnershipAttr::Share => Ownership::Share,
                    };
                    self.register_ownership(name.clone(), ownership);
                }
                let mut visited = std::collections::HashSet::new();
                self.check_type_recursion(name, body, &mut visited, ast::Span { line: 0, col: 0 });
            },

            ast::Decl::TypeAlias { name, ty, is_pub, .. } => {
                let converted = self.convert_type(ty);
                let module_path = self.current_module.clone();
                self.register_module_item(&module_path, name.clone(), converted.clone());
                self.type_alias_registry.insert(name.clone(), converted);

                if *is_pub {
                    self.mark_public(name.clone());
                }
            },

            ast::Decl::Trait { name, is_pub, items, .. } => {
                let method_names: Vec<String> = items.iter().filter_map(|i| match i {
                    ast::TraitItem::Required(sig) => Some(sig.name.clone()),
                    ast::TraitItem::Default(decl) => Some(decl.name.clone()),
                }).collect();
                let module_path = self.current_module.clone();
                self.register_module_item(&module_path, name.clone(), Type::Named(name.clone()));
                self.register_trait(name.clone(), method_names);

                if *is_pub {
                    self.mark_public(name.clone());
                }
            },

            ast::Decl::Const { name, ty, is_pub, .. } => {
                let const_type = self.convert_type(ty);
                let module_path = self.current_module.clone();
                self.register_module_item(&module_path, name.clone(), const_type.clone());
                self.insert_const_var(name.clone(), const_type);

                if *is_pub {
                    self.mark_public(name.clone());
                }
            },

            ast::Decl::Effect { name, is_pub, ops } => {
                let op_names: Vec<String> = ops.iter().map(|o| o.name.clone()).collect();
                let module_path = self.current_module.clone();
                self.register_module_item(&module_path, name.clone(), Type::Named(name.clone()));
                self.register_effect(name.clone(), op_names);

                if *is_pub {
                    self.mark_public(name.clone());
                }
            },

            ast::Decl::EffectAlias { name, is_pub, effects } => {
                let module_path = self.current_module.clone();
                self.register_module_item(&module_path, name.clone(), Type::Named(name.clone()));

                // Convert and register the effect alias
                let converted_effects = self.convert_effect_items(effects);
                self.register_effect_alias(name.clone(), converted_effects);

                if *is_pub {
                    self.mark_public(name.clone());
                }
            },

            ast::Decl::Import { path, items } => {
                self.register_import_items(path.clone(), items.clone());

                for item in items {
                    let import_name = item.alias.as_ref().unwrap_or(&item.name).clone();
                    self.check_import_collision(&import_name, path.clone());
                }
            },

            ast::Decl::Mod(name) => {
                // Register sub-module as an item in the parent, then enter it
                let parent_module = self.current_module.clone();
                self.register_module_item(&parent_module, name.clone(), Type::Named(format!("module::{}", name)));
                self.mark_public(name.clone()); // sub-modules are public so they can be traversed
                self.push_module(name.clone());
            },

            ast::Decl::Impl { .. } => {
                // Impl blocks are checked in pass 2
            },
        }
    }

    fn check_decl_body(&mut self, decl: &ast::Decl) {
        match decl {
            ast::Decl::Fn(fn_decl) => {
                self.check_fn_decl(fn_decl);
            },

            ast::Decl::Const { value, ty, .. } => {
                let const_type = self.convert_type(ty);
                let inferred = self.infer_expr(value);

                if !self.types_compatible(&const_type, &inferred) {
                    self.report_error(
                        format!("Const expression type mismatch: expected {}, got {}",
                            format!("{:?}", const_type), format!("{:?}", inferred)),
                        value.span,
                    );
                }
            },

            ast::Decl::Impl { methods, for_type, trait_name, generics, where_clause, .. } => {
                self.check_impl_decl(for_type, trait_name, generics, where_clause, methods);
            },

            _ => {},
        }
    }

    pub fn check_fn_decl(&mut self, fn_decl: &ast::FnDecl) {
        // Register generics and enforce where clause bounds.
        // type_args is empty at definition time; abstract generic vars are skipped.
        self.enforce_where_clause(
            &fn_decl.name,
            &fn_decl.generics,
            &fn_decl.where_clause,
            &[],
            ast::Span { line: 0, col: 0 },
        );

        let saved_declared = std::mem::take(&mut self.fn_declared_effects);
        let saved_required = std::mem::take(&mut self.fn_required_effects);
        let saved_handled = std::mem::take(&mut self.handled_effects);
        let converted = self.convert_effect_items(&fn_decl.effects);
        self.fn_declared_effects.extend(converted);

        self.scopes.push(Scope { vars: HashMap::new() });

        for param in &fn_decl.params {
            match param {
                ast::Param::Named { pattern, ty } => {
                    let param_type = self.convert_type(ty);
                    if let ast::Pattern::Bind(name) = &pattern.node {
                        self.insert_var(name.clone(), param_type, false, ast::Span { line: 0, col: 0 });
                    }
                },
                ast::Param::SelfVal | ast::Param::SelfRef { .. } => {
                    // Handle self parameter if needed
                },
            }
        }
        let body_type = self.infer_block(&fn_decl.body);

        if let Some(return_ty) = &fn_decl.return_type {
            let expected_return = self.convert_type(return_ty);
            if !self.types_compatible(&expected_return, &body_type) {
                let span = fn_decl.body.ret.as_ref().map(|r| r.span)
                    .or_else(|| fn_decl.body.stmts.last().map(|s| s.span))
                    .unwrap_or(ast::Span { line: 1, col: 1 });
                self.report_error(
                    format!("Return type mismatch in '{}': expected {}, got {}",
                        fn_decl.name, format!("{:?}", expected_return), format!("{:?}", body_type)),
                    span,
                );
            }
        }

        // Effect declaration check: every required effect produced by the body
        // must be either declared in the signature or handled inside the body.
        let required = self.fn_required_effects.clone();
        self.compute_unhandled_effects(&required);
        let leaked: Vec<_> = self.unhandled_effects.clone();
        for effect in &leaked {
            let declared = self.fn_declared_effects.iter().any(|d| self.effects_equal(d, effect));
            if !declared {
                let span = fn_decl.body.ret.as_ref().map(|r| r.span)
                    .or_else(|| fn_decl.body.stmts.first().map(|s| s.span))
                    .unwrap_or(ast::Span { line: 1, col: 1 });
                self.report_error(
                    format!("Function '{}' uses effect {:?} but does not declare it in its signature",
                        fn_decl.name, effect),
                    span,
                );
            }
        }

        self.scopes.pop();
        self.fn_declared_effects = saved_declared;
        self.fn_required_effects = saved_required;
        self.handled_effects = saved_handled;
    }

    // Per-declaration check
    pub fn check_type_decl(&mut self, name: &str, body: &ast::TypeBody, is_pub: bool, ownership: &Option<ast::OwnershipAttr>) {
        self.register_type(name.into(), body.clone());

        if is_pub {
            self.mark_public(name.into());
        }

        if let Some(own_attr) = ownership {
            let own = match own_attr {
                ast::OwnershipAttr::Copy => Ownership::Copy,
                ast::OwnershipAttr::Move => Ownership::Move,
                ast::OwnershipAttr::Share => Ownership::Share,
            };
            self.register_ownership(name.into(), own);
        }

        if let ast::TypeBody::Variant(cases) = body {
            let case_names: Vec<String> = cases.iter().map(|c| match c {
                ast::VariantCase::Unit(n) => n.clone(),
                ast::VariantCase::Tuple(n, _) => n.clone(),
                ast::VariantCase::Record(n, _) => n.clone(),
            }).collect();
            self.register_variant_cases(name.into(), case_names);
        }

        let mut visited = std::collections::HashSet::new();
        self.check_type_recursion(name, body, &mut visited, ast::Span { line: 0, col: 0 });
    }

    pub fn check_impl_decl(
        &mut self,
        for_type: &ast::Type,
        trait_name: &Option<Vec<String>>,
        generics: &[ast::GenericParam],
        where_clause: &[ast::WhereBound],
        methods: &[ast::FnDecl],
    ) {
        let type_name = match for_type {
            ast::Type::Named(n) => n.clone(),
            ast::Type::Qualified(parts) => parts.join("::"),
            _ => "UnknownType".into(),
        };

        // Build concrete type_args from for_type's generic arguments.
        // e.g. impl<T: Show> Wrapper<Int> -> T = Int.
        let converted = self.convert_type(for_type);
        let type_args: Vec<(String, Type)> = match &converted {
            Type::Generic { args, .. } => generics.iter().zip(args.iter())
                .map(|(g, arg)| (g.name.clone(), arg.clone()))
                .collect(),
            _ => vec![],
        };

        self.enforce_where_clause(
            &type_name,
            generics,
            where_clause,
            &type_args,
            ast::Span { line: 0, col: 0 },
        );

        for method in methods {
            self.check_fn_decl(method);
        }

        if let Some(trait_path) = trait_name {
            let trait_str = trait_path.join("::");
            self.register_impl(&type_name, &trait_str);
        }
    }

    pub fn check_const_decl(&mut self, name: &str, ty: &ast::Type, value: &Spanned<ast::Expr>, is_pub: bool) {
        let const_type = self.convert_type(ty);

        self.insert_const_var(name.into(), const_type.clone());
        if is_pub {
            self.mark_public(name.into());
        }
        let inferred = self.infer_expr(value);

        if !self.types_compatible(&const_type, &inferred) {
            self.report_error(
                format!("Const type mismatch: expected {:?}, found {:?}",
                    const_type, inferred),
                value.span,
            );
        }

        if !self.check_const_expr(&value.node, value.span) {
            self.report_error(
                "Const expression must be pure (no side effects)".into(),
                value.span,
            );
        }
    }

    pub fn check_effect_decl(&mut self, name: &str, ops: &[ast::FnSignature], is_pub: bool) {

        let op_names: Vec<String> = ops.iter().map(|op| op.name.clone()).collect();
        self.register_effect(name.into(), op_names);

        if is_pub {
            self.mark_public(name.into());
        }

        for op in ops {
            let params: Vec<Type> = op.params.iter()
                .filter_map(|p| match p {
                    ast::Param::Named { ty, .. } => Some(self.convert_type(ty)),
                    _ => None,
                })
                .collect();
            let effects: Vec<crate::ty::Effect> = op.effects.iter()
                .filter_map(|eff| self.convert_effect(eff))
                .collect();
            let ret = op.return_type.as_ref()
                .map(|t| Box::new(self.convert_type(t)))
                .unwrap_or_else(|| Box::new(Type::Unit));

            let op_type = Type::Function { params, effects, ret };

            let op_key = format!("{}::{}", name, op.name);
            self.effect_ops_registry.insert(op_key, op_type);
        }
    }

    pub fn check_import_decl(&mut self, path: &[String], items: &[ast::ImportItem]) {
        self.register_import_items(path.to_vec(), items.to_vec());
        for item in items {
            let import_name = item.alias.as_ref().unwrap_or(&item.name).clone();
            self.check_import_collision(&import_name, path.to_vec());
        }
    }

    pub fn resolve_field_access(&mut self, base_ty: &Type, field_name: &str, span: Span) -> Type {
        match base_ty {
            Type::Named(type_name) => {
                if let Some(type_body) = self.type_registry.get(type_name).cloned() {
                    match type_body {
                        ast::TypeBody::Record(fields) => {
                            for field in fields {
                                if field.name == field_name {
                                    return self.convert_type(&field.ty);
                                }
                            }
                            self.report_error(
                                format!("Field '{}' not found in record type '{}'", field_name, type_name),
                                span,
                            );
                            Type::Unknown
                        },
                        ast::TypeBody::Variant(_) => {
                            self.report_error(
                                format!("Cannot access field '{}' on variant type '{}'", field_name, type_name),
                                span,
                            );
                            Type::Unknown
                        }
                    }
                } else {
                    self.report_error(
                        format!("Type '{}' not found - cannot resolve field '{}'", type_name, field_name),
                        span,
                    );
                    Type::Unknown
                }
            },
            Type::Generic { name, args: _ } => {
                if let Some(type_body) = self.type_registry.get(name).cloned() {
                    match type_body {
                        ast::TypeBody::Record(fields) => {
                            for field in fields {
                                if field.name == field_name {
                                    let field_type = self.convert_type(&field.ty);
                                    return self.substitute_generic_field_type(base_ty, &field_type);
                                }
                            }
                            self.report_error(
                                format!("Field '{}' not found in record type '{}'", field_name, name),
                                span,
                            );
                            Type::Unknown
                        },
                        _ => {
                            self.report_error(
                                format!("Cannot access field '{}' on type '{}'", field_name, name),
                                span,
                            );
                            Type::Unknown
                        }
                    }
                } else {
                    self.report_error(
                        format!("Type '{}' not found - cannot resolve field '{}'", name, field_name),
                        span,
                    );
                    Type::Unknown
                }
            },
            Type::Unknown => Type::Unknown,
            _ => {
                self.report_error(
                    format!("Cannot access field '{}' on type {:?}", field_name, base_ty),
                    span,
                );
                Type::Unknown
            }
        }
    }

    pub fn extract_iterable_element_type(&self, iter_ty: &Type) -> Type {
        match iter_ty {
            Type::Generic { name, args } => {
                match name.as_str() {
                    "List" | "Vec" | "Array" => {
                        if !args.is_empty() {
                            args[0].clone()
                        } else {
                            Type::Unknown
                        }
                    },
                    "Option" => {
                        if !args.is_empty() {
                            args[0].clone()
                        } else {
                            Type::Unknown
                        }
                    },
                    "Result" => {
                        if !args.is_empty() {
                            args[0].clone()
                        } else {
                            Type::Unknown
                        }
                    },
                    _ => Type::Unknown,
                }
            },
            Type::String => Type::Char,
            Type::Named(_) => Type::Unknown,
            _ => Type::Unknown,
        }
    }

    pub fn substitute_generic_field_type(&self, base_ty: &Type, field_ty: &Type) -> Type {
        match (base_ty, field_ty) {
            (Type::Generic { args, .. }, Type::Named(type_var)) if type_var == "T" && !args.is_empty() => {
                args[0].clone()
            },
            _ => field_ty.clone(),
        }
    }
}
