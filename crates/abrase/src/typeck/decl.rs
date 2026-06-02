use std::collections::HashMap;
use crate::ast;
use crate::ast::{Span, Spanned};
use crate::ty::{Ownership, Type};
use super::*;

impl Checker {

    pub fn check_program(&mut self, decls: &[ast::Decl]) {
        for decl in decls {
            self.check_decl_signature(decl);
        }
        for decl in decls {
            self.check_decl_body(decl);
        }
        let all_idents = all_program_idents(decls);
        lint_unused_imports(decls, &all_idents, self);
        lint_dead_code(decls, self);
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

                // main() must be pure (no effects)
                if fn_decl.name == "main" && !effects.is_empty() {
                    self.report_error(
                        format!("`main` function must be pure (no effects); found: {}",
                            fn_decl.effects.iter().map(|e| e.name.join(".")).collect::<Vec<_>>().join(", ")),
                        ast::Span { line: 0, col: 0 }
                    );
                }

                let fn_type = Type::Function { params, effects, ret };
                let module_path = self.current_module.clone();
                self.register_module_item(&module_path, fn_decl.name.clone(), fn_type.clone());
                self.insert_var(fn_decl.name.clone(), fn_type, false, ast::Span { line: 0, col: 0 });

                if !fn_decl.generics.is_empty() {
                    let names: Vec<String> = fn_decl.generics.iter().map(|g| g.name.clone()).collect();
                    self.register_generic_params(fn_decl.name.clone(), names);
                }

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
                        ast::OwnershipAttr::Share => {
                            self.report_error(
                                format!(
                                    "type '{}' cannot be declared `@share`; \
                                     wrap it in `Shared<T>` instead", name
                                ),
                                ast::Span { line: 0, col: 0 },
                            );
                            Ownership::Move
                        }
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
                if is_reserved_trait_name(name) {
                    self.report_error(
                        format!("Cannot redefine built-in trait '{}'; \
                                 it is reserved by `@derive` and cannot be shadowed",
                                name),
                        ast::Span { line: 0, col: 0 },
                    );
                    return;
                }
                let method_names: Vec<String> = items.iter().filter_map(|i| match i {
                    ast::TraitItem::Required(sig) => Some(sig.name.clone()),
                    ast::TraitItem::Default(decl) => Some(decl.name.clone()),
                }).collect();
                let module_path = self.current_module.clone();
                self.register_module_item(&module_path, name.clone(), Type::Named(name.clone()));
                self.register_trait(name.clone(), method_names);

                // Record each method's signature
                for item in items {
                    let (mname, params_ast, ret_ast) = match item {
                        ast::TraitItem::Required(sig) => (
                            sig.name.clone(),
                            sig.params.clone(),
                            sig.return_type.clone(),
                        ),
                        ast::TraitItem::Default(decl) => (
                            decl.name.clone(),
                            decl.params.clone(),
                            decl.return_type.clone(),
                        ),
                    };
                    let params: Vec<Type> = params_ast.iter().map(|p| match p {
                        ast::Param::Named { ty, .. } => self.convert_type(ty),
                        ast::Param::SelfVal => Type::Named("Self".into()),
                        ast::Param::SelfRef { is_mut } => Type::Reference {
                            is_mut: *is_mut,
                            inner: Box::new(Type::Named("Self".into())),
                        },
                    }).collect();
                    let ret = ret_ast
                        .map(|t| self.convert_type(&t))
                        .unwrap_or(Type::Unit);
                    self.register_trait_method_sig(name, &mname, params, ret);
                }

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

            ast::Decl::Static { name, ty, is_pub, is_mut, .. } => {
                let static_type = self.convert_type(ty);
                if matches!(static_type, Type::Shared { .. }) {
                    self.report_error(
                        format!("`static {}` cannot hold `Shared<T>`; Shared must be constructed inside a region", name),
                        ast::Span { line: 0, col: 0 },
                    );
                }
                if matches!(static_type, Type::Reference { .. }) {
                    self.report_error(
                        format!("`static {}` cannot hold `&T`; references are region-scoped", name),
                        ast::Span { line: 0, col: 0 },
                    );
                }
                let module_path = self.current_module.clone();
                if self.lookup_module_item(&module_path, name).is_some() {
                    self.report_error(
                        format!("`{}` is already declared in this module", name),
                        ast::Span { line: 0, col: 0 },
                    );
                }
                self.register_module_item(&module_path, name.clone(), static_type.clone());
                self.insert_static_var(name.clone(), static_type, *is_mut);
                if *is_pub {
                    self.mark_public(name.clone());
                }
            },

            ast::Decl::Effect { name, is_pub, ops } => {
                // Register effect name and per-op Function types for call-site resolution.
                let module_path = self.current_module.clone();
                self.register_module_item(&module_path, name.clone(), Type::Named(name.clone()));
                self.check_effect_decl(name, ops, *is_pub);
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

            ast::Decl::Use { path, items } => {
                self.register_import_items(path.clone(), items.clone());

                for item in items {
                    let import_name = item.alias.as_ref().unwrap_or(&item.name).clone();
                    self.check_import_collision(&import_name, path.clone());

                    if self.lookup_module_item(path, &item.name).is_some()
                        && !self.is_accessible(&item.name, path)
                    {
                        self.report_error(
                            format!("Cannot import '{}' from {}: item is private",
                                item.name, path.join(".")),
                            ast::Span { line: 0, col: 0 },
                        );
                    }
                }
            },

            ast::Decl::ModEnter(path) => {
                self.enter_imported_module(path.clone());
            },

            ast::Decl::ModExit => {
                self.exit_imported_module();
            },

            ast::Decl::Impl { methods, for_type, trait_name, .. } => {
                // Register impl methods early so call sites can resolve them
                // regardless of declaration order. Conformance checks run in pass 2.
                let type_name = match for_type {
                    ast::Type::Named(n) => n.clone(),
                    ast::Type::Qualified(parts) => parts.join("::"),
                    _ => "UnknownType".into(),
                };
                if let Some(trait_path) = trait_name {
                    let trait_str = trait_path.join("::");
                    for method in methods {
                        let mangled = Self::mangle_impl_method(&trait_str, &type_name, &method.name);
                        self.register_impl_method(&trait_str, &type_name, &method.name, mangled);
                    }
                }
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

            ast::Decl::Static { name, value, ty, .. } => {
                let static_type = self.convert_type(ty);
                let inferred = self.infer_expr(value);
                if !self.types_compatible(&static_type, &inferred) {
                    self.report_error(
                        format!("`static {}` expression type mismatch: expected {:?}, got {:?}",
                            name, static_type, inferred),
                        value.span,
                    );
                }
            },

            ast::Decl::Impl { methods, for_type, trait_name, generics, where_clause, .. } => {
                self.check_impl_decl(for_type, trait_name, generics, where_clause, methods);
            },

            ast::Decl::ModEnter(path) => {
                self.enter_imported_module(path.clone());
            },

            ast::Decl::ModExit => {
                self.exit_imported_module();
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
                    match &pattern.node {
                        ast::Pattern::Bind(name) => {
                            self.insert_var(name.clone(), param_type, false, ast::Span { line: 0, col: 0 });
                        }
                        _ => {
                            self.check_pattern(pattern, &param_type, pattern.span);
                        }
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

        self.exit_scope();
        self.fn_declared_effects = saved_declared;
        self.fn_required_effects = saved_required;
        self.handled_effects = saved_handled;

        self.lint_unused_variables(fn_decl);
    }

    fn lint_unused_variables(&mut self, fn_decl: &ast::FnDecl) {
        let uses = crate::compiler::liveness::count_uses(&fn_decl.body);

        // Parameters
        for param in &fn_decl.params {
            if let ast::Param::Named { pattern, .. } = param {
                if let ast::Pattern::Bind(name) = &pattern.node {
                    if !name.starts_with('_') && uses.get(name.as_str()).copied().unwrap_or(0) == 0 {
                        self.report_warning(
                            "unused_variable",
                            format!("unused parameter `{}`", name),
                            pattern.span,
                        );
                    }
                }
            }
        }

        // Let bindings in function body
        lint_unused_in_block(&fn_decl.body, &uses, self);
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
                ast::OwnershipAttr::Share => {
                    self.report_error(
                        format!(
                            "type '{}' cannot be declared `@share`; \
                             wrap it in `Shared<T>` instead", name
                        ),
                        ast::Span { line: 0, col: 0 },
                    );
                    Ownership::Move
                }
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

        // Translate Self-style params to a regular `self: ReceiverType` binding so
        // the existing fn checker can type-check the body without special cases.
        for method in methods {
            let translated_params: Vec<ast::Param> = method.params.iter().map(|p| match p {
                ast::Param::SelfVal => ast::Param::Named {
                    pattern: ast::Spanned {
                        node: ast::Pattern::Bind("self".into()),
                        span: ast::Span { line: 0, col: 0 },
                    },
                    ty: ast::Type::Named(type_name.clone()),
                },
                ast::Param::SelfRef { is_mut } => ast::Param::Named {
                    pattern: ast::Spanned {
                        node: ast::Pattern::Bind("self".into()),
                        span: ast::Span { line: 0, col: 0 },
                    },
                    ty: ast::Type::Reference {
                        is_mut: *is_mut,
                        inner: Box::new(ast::Type::Named(type_name.clone())),
                        region: None,
                    },
                },
                ast::Param::Named { pattern, ty } => ast::Param::Named {
                    pattern: pattern.clone(),
                    ty: subst_self_in_ast_type(ty, &type_name),
                },
            }).collect();
            let translated_ret = method.return_type.as_ref()
                .map(|t| subst_self_in_ast_type(t, &type_name));
            let mut translated = method.clone();
            translated.params = translated_params;
            translated.return_type = translated_ret;
            self.check_fn_decl(&translated);
        }

        if let Some(trait_path) = trait_name {
            let trait_str = trait_path.join("::");
            self.register_impl(&type_name, &trait_str);

            // Per-method registration + signature conformance check.
            let trait_method_names: Vec<String> = self.get_trait(&trait_str).unwrap_or_default();
            let provided: std::collections::HashSet<String> = methods.iter()
                .map(|m| m.name.clone()).collect();
            for required in &trait_method_names {
                if !provided.contains(required) {
                    self.report_error(
                        format!("impl of trait '{}' for type '{}' is missing method '{}'",
                            trait_str, type_name, required),
                        ast::Span { line: 0, col: 0 },
                    );
                }
            }
            for method in methods {
                let mangled = Self::mangle_impl_method(&trait_str, &type_name, &method.name);
                self.register_impl_method(&trait_str, &type_name, &method.name, mangled);

                if let Some((expected_params, expected_ret)) =
                    self.get_trait_method_sig(&trait_str, &method.name)
                {
                    let impl_params: Vec<Type> = method.params.iter().map(|p| match p {
                        ast::Param::Named { ty, .. } => self.convert_type(ty),
                        ast::Param::SelfVal => Type::Named(type_name.clone()),
                        ast::Param::SelfRef { is_mut } => Type::Reference {
                            is_mut: *is_mut,
                            inner: Box::new(Type::Named(type_name.clone())),
                        },
                    }).collect();
                    let impl_ret = method.return_type.as_ref()
                        .map(|t| self.convert_type(t)).unwrap_or(Type::Unit);

                    let sub_self = |t: &Type| -> Type {
                        match t {
                            Type::Named(n) if n == "Self" => Type::Named(type_name.clone()),
                            Type::Reference { is_mut, inner } => {
                                let inner = if let Type::Named(n) = inner.as_ref() {
                                    if n == "Self" { Type::Named(type_name.clone()) }
                                    else { (**inner).clone() }
                                } else { (**inner).clone() };
                                Type::Reference { is_mut: *is_mut, inner: Box::new(inner) }
                            }
                            other => other.clone(),
                        }
                    };
                    let expected_params: Vec<Type> = expected_params.iter().map(&sub_self).collect();
                    let expected_ret = sub_self(&expected_ret);

                    let sig_ok = impl_params.len() == expected_params.len()
                        && impl_params.iter().zip(expected_params.iter()).all(|(a, e)| a == e)
                        && impl_ret == expected_ret;
                    if !sig_ok {
                        self.report_error(
                            format!("impl method '{}' for '{}' does not match trait '{}' signature: \
                                     expected ({:?}) -> {:?}, got ({:?}) -> {:?}",
                                method.name, type_name, trait_str,
                                expected_params, expected_ret, impl_params, impl_ret),
                            ast::Span { line: 0, col: 0 },
                        );
                    }
                }
            }
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
            Type::Reference { inner, .. } => self.resolve_field_access(inner, field_name, span),
            Type::Tuple(elems) => {
                if let Ok(idx) = field_name.parse::<usize>() {
                    if let Some(t) = elems.get(idx) { return t.clone(); }
                }
                self.report_error(
                    format!("Tuple has no element '{}'", field_name), span);
                Type::Unknown
            }
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

    pub fn extract_iterable_element_type(&mut self, iter_ty: &Type, span: Span) -> Type {
        match iter_ty {
            Type::Unknown => Type::Unknown,
            Type::Generic { name, args } => {
                match name.as_str() {
                    "List" | "Vec" | "Array" | "Option" | "Result" => {
                        if args.is_empty() {
                            self.report_error(
                                format!("Generic type '{}' iterated without a type argument", name),
                                span,
                            );
                            Type::Unknown
                        } else {
                            args[0].clone()
                        }
                    }
                    _ => {
                        self.report_error(
                            format!("Type '{}<...>' is not iterable", name),
                            span,
                        );
                        Type::Unknown
                    }
                }
            }
            Type::String => Type::Char,
            Type::Named(n) if n == "Range<Int>" || n.starts_with("Range") => Type::Int,
            other => {
                self.report_error(
                    format!("Type {:?} is not iterable", other),
                    span,
                );
                Type::Unknown
            }
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

/// Built-in trait names reserved by `@derive`
pub(crate) const RESERVED_TRAIT_NAMES: &[&str] = &["Show", "Eq", "Ord", "Clone"];

pub(crate) fn is_reserved_trait_name(name: &str) -> bool {
    RESERVED_TRAIT_NAMES.iter().any(|n| *n == name)
}

fn subst_self_in_ast_type(ty: &ast::Type, receiver: &str) -> ast::Type {
    use ast::Type as T;
    match ty {
        T::Named(n) if n == "Self" => T::Named(receiver.into()),
        T::Named(_) | T::Qualified(_) => ty.clone(),
        T::Generic { name, args } => T::Generic {
            name: name.clone(),
            args: args.iter().map(|a| subst_self_in_ast_type(a, receiver)).collect(),
        },
        T::Array { elem, size } => T::Array {
            elem: Box::new(subst_self_in_ast_type(elem, receiver)),
            size: *size,
        },
        T::Tuple(ts) => T::Tuple(ts.iter().map(|t| subst_self_in_ast_type(t, receiver)).collect()),
        T::Reference { is_mut, inner, region } => T::Reference {
            is_mut: *is_mut,
            inner: Box::new(subst_self_in_ast_type(inner, receiver)),
            region: region.clone(),
        },
        T::Function { params, effects, ret } => T::Function {
            params: params.iter().map(|p| subst_self_in_ast_type(p, receiver)).collect(),
            effects: effects.clone(),
            ret: Box::new(subst_self_in_ast_type(ret, receiver)),
        },
    }
}

fn lint_unused_in_block(block: &ast::Block, uses: &std::collections::HashMap<String, usize>, checker: &mut super::Checker) {
    for stmt in &block.stmts {
        lint_unused_in_stmt(stmt, uses, checker);
    }
    if let Some(ret) = &block.ret {
        lint_unused_in_expr(ret, uses, checker);
    }
}

fn lint_unused_in_stmt(stmt: &ast::Spanned<ast::Stmt>, uses: &std::collections::HashMap<String, usize>, checker: &mut super::Checker) {
    match &stmt.node {
        ast::Stmt::Let { pattern, value, .. } => {
            if let ast::Pattern::Bind(name) = &pattern.node {
                if !name.starts_with('_') && uses.get(name.as_str()).copied().unwrap_or(0) == 0 {
                    checker.report_warning(
                        "unused_variable",
                        format!("unused variable `{}`", name),
                        pattern.span,
                    );
                }
            }
            lint_unused_in_expr(value, uses, checker);
        }
        ast::Stmt::Expr(e) => lint_unused_in_expr(e, uses, checker),
        ast::Stmt::Empty => {}
    }
}

fn lint_unused_in_expr(expr: &ast::Spanned<ast::Expr>, uses: &std::collections::HashMap<String, usize>, checker: &mut super::Checker) {
    match &expr.node {
        ast::Expr::Block(b) => lint_unused_in_block(b, uses, checker),
        ast::Expr::If { consequence, alternative, .. } => {
            lint_unused_in_expr(consequence, uses, checker);
            if let Some(alt) = alternative { lint_unused_in_expr(alt, uses, checker); }
        }
        ast::Expr::While { body, .. } | ast::Expr::Loop { body } => {
            lint_unused_in_block(body, uses, checker);
        }
        ast::Expr::For { body, .. } => lint_unused_in_block(body, uses, checker),
        ast::Expr::Region { body, .. } => lint_unused_in_block(body, uses, checker),
        _ => {}
    }
}

// ── Global ident collection (for unused_import / dead_code) ──────────────────

fn collect_idents_block(block: &ast::Block, out: &mut std::collections::HashSet<String>) {
    for stmt in &block.stmts { collect_idents_stmt(stmt, out); }
    if let Some(r) = &block.ret { collect_idents_expr(r, out); }
}

fn collect_idents_stmt(stmt: &ast::Spanned<ast::Stmt>, out: &mut std::collections::HashSet<String>) {
    match &stmt.node {
        ast::Stmt::Let { value, .. } => collect_idents_expr(value, out),
        ast::Stmt::Expr(e) => collect_idents_expr(e, out),
        ast::Stmt::Empty => {}
    }
}

fn collect_idents_expr(expr: &ast::Spanned<ast::Expr>, out: &mut std::collections::HashSet<String>) {
    use ast::Expr::*;
    match &expr.node {
        Identifier(n) => { out.insert(n.clone()); }
        Call { callee, args } => {
            collect_idents_expr(callee, out);
            for a in args { collect_idents_expr(a, out); }
        }
        Binary { left, right, .. } => { collect_idents_expr(left, out); collect_idents_expr(right, out); }
        Unary { right, .. } => collect_idents_expr(right, out),
        If { condition, consequence, alternative } => {
            collect_idents_expr(condition, out);
            collect_idents_expr(consequence, out);
            if let Some(alt) = alternative { collect_idents_expr(alt, out); }
        }
        Block(b) => collect_idents_block(b, out),
        While { condition, body } => { collect_idents_expr(condition, out); collect_idents_block(body, out); }
        For { iter, body, .. } => { collect_idents_expr(iter, out); collect_idents_block(body, out); }
        Loop { body } => collect_idents_block(body, out),
        Region { body, .. } => collect_idents_block(body, out),
        Match { scrutinee, arms } => {
            collect_idents_expr(scrutinee, out);
            for arm in arms {
                collect_idents_expr(&arm.body, out);
                if let Some(g) = &arm.guard { collect_idents_expr(g, out); }
            }
        }
        Handle { expr, arms } => {
            collect_idents_expr(expr, out);
            for arm in arms { collect_idents_expr(&arm.body, out); }
        }
        FieldAccess { base, .. } => collect_idents_expr(base, out),
        Index { base, index } => { collect_idents_expr(base, out); collect_idents_expr(index, out); }
        Record { ty, fields } => {
            if let Some(n) = ty.last() { out.insert(n.clone()); }
            for f in fields { if let Some(v) = &f.value { collect_idents_expr(v, out); } }
        }
        Variant { ty, args } => {
            if let Some(n) = ty.last() { out.insert(n.clone()); }
            for a in args { collect_idents_expr(a, out); }
        }
        Tuple(items) | Array(items) => { for i in items { collect_idents_expr(i, out); } }
        Closure { body, .. } => collect_idents_expr(body, out),
        Return(Some(e)) | Throw(e) | Paren(e) | Question(e) | Resume(Some(e)) => collect_idents_expr(e, out),
        Break(Some(e)) => collect_idents_expr(e, out),
        Range { start, end, .. } => {
            if let Some(s) = start { collect_idents_expr(s, out); }
            if let Some(e) = end { collect_idents_expr(e, out); }
        }
        ArrayRepeat { elem, count } => { collect_idents_expr(elem, out); collect_idents_expr(count, out); }
        _ => {}
    }
}

fn collect_type_names(ty: &ast::Type, out: &mut std::collections::HashSet<String>) {
    match ty {
        ast::Type::Named(n) => { out.insert(n.clone()); }
        ast::Type::Generic { name, args } => {
            out.insert(name.clone());
            for a in args { collect_type_names(a, out); }
        }
        ast::Type::Tuple(ts) => { for t in ts { collect_type_names(t, out); } }
        ast::Type::Array { elem, .. } => collect_type_names(elem, out),
        ast::Type::Function { params, ret, .. } => {
            for p in params { collect_type_names(p, out); }
            collect_type_names(ret, out);
        }
        ast::Type::Reference { inner, .. } => collect_type_names(inner, out),
        _ => {}
    }
}

// Collect all idents referenced across every function/static body in the program.
fn all_program_idents(decls: &[ast::Decl]) -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::new();
    for decl in decls {
        match decl {
            ast::Decl::Fn(f) => {
                collect_idents_block(&f.body, &mut out);
                for param in &f.params {
                    if let ast::Param::Named { ty, .. } = param {
                        collect_type_names(ty, &mut out);
                    }
                }
                if let Some(ret) = &f.return_type { collect_type_names(ret, &mut out); }
            }
            ast::Decl::Static { value, ty, .. } | ast::Decl::Const { value, ty, .. } => {
                collect_idents_expr(value, &mut out);
                collect_type_names(ty, &mut out);
            }
            ast::Decl::Type { body, .. } => {
                collect_type_names_in_body(body, &mut out);
            }
            _ => {}
        }
    }
    out
}

fn collect_type_names_in_body(body: &ast::TypeBody, out: &mut std::collections::HashSet<String>) {
    match body {
        ast::TypeBody::Record(fields) => {
            for f in fields { collect_type_names(&f.ty, out); }
        }
        ast::TypeBody::Variant(variants) => {
            for v in variants {
                match v {
                    ast::VariantCase::Tuple(_, tys) => { for t in tys { collect_type_names(t, out); } }
                    ast::VariantCase::Record(_, fields) => { for f in fields { collect_type_names(&f.ty, out); } }
                    ast::VariantCase::Unit(_) => {}
                }
            }
        }
    }
}

// ── Lint: unused_import ───────────────────────────────────────────────────────

pub(super) fn lint_unused_imports(decls: &[ast::Decl], all_idents: &std::collections::HashSet<String>, checker: &mut super::Checker) {
    for decl in decls {
        if let ast::Decl::Use { items, .. } = decl {
            for item in items {
                let local_name = item.alias.as_ref().unwrap_or(&item.name);
                if !all_idents.contains(local_name.as_str()) {
                    checker.report_warning(
                        "unused_import",
                        format!("unused import `{}`", local_name),
                        ast::Span { line: 0, col: 0 },
                    );
                }
            }
        }
    }
}

// ── Lint: dead_code (functions + types) ──────────────────────────────────────

pub(super) fn lint_dead_code(decls: &[ast::Decl], checker: &mut super::Checker) {
    use std::collections::{HashMap, HashSet, VecDeque};

    // Collect fn metadata: name → (is_pub, span, is_synthetic)
    let mut fn_info: HashMap<String, (bool, ast::Span)> = HashMap::new();
    // Per-function referenced idents
    let mut fn_refs: HashMap<String, HashSet<String>> = HashMap::new();
    // Type metadata
    let mut type_info: HashMap<String, (bool, ast::Span)> = HashMap::new();

    for decl in decls {
        match decl {
            ast::Decl::Fn(f) => {
                let synthetic = f.name.starts_with("__");
                if !synthetic {
                    fn_info.insert(f.name.clone(), (f.is_pub, ast::Span { line: 0, col: 0 }));
                }
                let mut refs = HashSet::new();
                collect_idents_block(&f.body, &mut refs);
                fn_refs.insert(f.name.clone(), refs);
            }
            ast::Decl::Type { name, is_pub, .. } => {
                type_info.insert(name.clone(), (*is_pub, ast::Span { line: 0, col: 0 }));
            }
            _ => {}
        }
    }

    // BFS from live roots: main + pub fns + synthetic fns
    let mut live: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<String> = VecDeque::new();

    for name in fn_info.keys() {
        let (is_pub, _) = fn_info[name];
        if is_pub || name == "main" {
            live.insert(name.clone());
            queue.push_back(name.clone());
        }
    }
    // Also seed synthetic fns as live
    for name in fn_refs.keys() {
        if name.starts_with("__") {
            live.insert(name.clone());
            queue.push_back(name.clone());
        }
    }

    while let Some(fn_name) = queue.pop_front() {
        if let Some(refs) = fn_refs.get(&fn_name) {
            for r in refs {
                if fn_refs.contains_key(r) && !live.contains(r) {
                    live.insert(r.clone());
                    queue.push_back(r.clone());
                }
            }
        }
    }

    // Collect all type references across all fn bodies
    let all_idents = all_program_idents(decls);

    // Report dead functions
    for (name, (is_pub, span)) in &fn_info {
        if !is_pub && !live.contains(name) {
            checker.report_warning(
                "dead_code",
                format!("function `{}` is never used", name),
                *span,
            );
        }
    }

    // Report dead types (non-pub, never referenced as ident or in type positions)
    for (name, (is_pub, span)) in &type_info {
        if !is_pub && !all_idents.contains(name.as_str()) {
            checker.report_warning(
                "dead_code",
                format!("type `{}` is never used", name),
                *span,
            );
        }
    }
}
