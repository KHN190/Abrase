use abrase::ast;
use abrase::ty::Type;
use abrase::typeck::Checker;

fn mk() -> Checker { Checker::new() }
fn span() -> ast::Span { ast::Span { line: 1, col: 1 } }
fn gp(name: &str) -> ast::GenericParam { ast::GenericParam { name: name.into() } }
fn wb(ty: &str, traits: &[&str]) -> ast::WhereBound {
    ast::WhereBound {
        ty: ast::Type::Named(ty.into()),
        bounds: traits.iter().map(|t| vec![t.to_string()]).collect(),
    }
}
fn fn_decl(name: &str, generics: Vec<ast::GenericParam>, where_clause: Vec<ast::WhereBound>) -> ast::FnDecl {
    ast::FnDecl {
        name: name.into(),
        is_pub: false,
        attrs: vec![],
        params: vec![],
        return_type: None,
        effects: vec![],
        where_clause,
        generics,
        body: ast::Block { stmts: vec![], ret: None },
    }
}

// Generics & Trait Constraints Tests

#[test]
fn verify_register_trait() {
    let mut checker = Checker::new();

    let methods = vec!["display".into(), "debug".into()];
    checker.register_trait("Show".into(), methods.clone());

    let trait_methods = checker.get_trait("Show");
    assert!(trait_methods.is_some());
    assert_eq!(trait_methods.unwrap(), methods);
}

#[test]
fn verify_register_impl() {
    let mut checker = Checker::new();

    checker.register_impl("Int".into(), "Show".into());
    assert!(checker.has_impl("Int", "Show"));
}

#[test]
fn verify_impl_not_registered() {
    let checker = Checker::new();

    assert!(!checker.has_impl("String", "Iterator"));
}

#[test]
fn verify_register_generic_params() {
    let mut checker = Checker::new();

    let params = vec!["T".into(), "U".into()];
    checker.register_generic_params("map".into(), params.clone());

    let registered = checker.get_generic_params("map");
    assert!(registered.is_some());
    assert_eq!(registered.unwrap(), params);
}

#[test]
fn verify_register_trait_bound() {
    let mut checker = Checker::new();

    checker.register_trait_bound("T".into(), "Show".into());
    checker.register_trait_bound("T".into(), "Debug".into());

    let bounds = checker.get_trait_bounds("T");
    assert!(bounds.is_some());
    assert_eq!(bounds.unwrap().len(), 2);
}

#[test]
fn verify_validate_where_clause_satisfied() {
    let mut checker = Checker::new();

    checker.register_trait_bound("T".into(), "Show".into());
    checker.register_impl("Int".into(), "Show".into());

    assert!(checker.validate_where_clause("T", "Int"));
}

#[test]
fn verify_validate_where_clause_unsatisfied() {
    let mut checker = Checker::new();

    checker.register_trait_bound("T".into(), "Show".into());

    assert!(!checker.validate_where_clause("T", "String"));
}

#[test]
fn verify_validate_where_clause_no_bounds() {
    let checker = Checker::new();

    assert!(checker.validate_where_clause("T", "Int"));
}

#[test]
fn verify_validate_generic_instance_correct_params() {
    let mut checker = Checker::new();

    checker.register_generic_params("Vec".into(), vec!["T".into()]);
    assert!(checker.validate_generic_instance("Vec", &[Type::Int]));
}

#[test]
fn verify_validate_generic_instance_wrong_param_count() {
    let mut checker = Checker::new();

    checker.register_generic_params("Vec".into(), vec!["T".into()]);
    assert!(!checker.validate_generic_instance("Vec", &[Type::Int, Type::Bool]));
}

#[test]
fn verify_validate_generic_instance_no_params() {
    let checker = Checker::new();

    assert!(checker.validate_generic_instance("Int", &[]));
}

#[test]
fn verify_check_all_trait_bounds_satisfied() {
    let mut checker = Checker::new();

    checker.register_generic_params("apply".into(), vec!["T".into()]);
    checker.register_trait_bound("T".into(), "Show".into());
    checker.register_impl("Int".into(), "Show".into());

    let type_args = vec![("T".into(), Type::Int)];
    assert!(checker.check_all_trait_bounds("apply", &type_args));
}

#[test]
fn verify_check_all_trait_bounds_unsatisfied() {
    let mut checker = Checker::new();

    checker.register_generic_params("apply".into(), vec!["T".into()]);
    checker.register_trait_bound("T".into(), "Show".into());

    let type_args = vec![("T".into(), Type::String)];
    assert!(!checker.check_all_trait_bounds("apply", &type_args));
}

#[test]
fn verify_check_all_trait_bounds_multiple_constraints() {
    let mut checker = Checker::new();

    checker.register_generic_params("apply".into(), vec!["T".into()]);
    checker.register_trait_bound("T".into(), "Show".into());
    checker.register_trait_bound("T".into(), "Debug".into());
    checker.register_impl("Int".into(), "Show".into());
    checker.register_impl("Int".into(), "Debug".into());

    let type_args = vec![("T".into(), Type::Int)];
    assert!(checker.check_all_trait_bounds("apply", &type_args));
}

#[test]
fn verify_multiple_generic_params() {
    let mut checker = Checker::new();

    checker.register_generic_params("map".into(), vec!["T".into(), "U".into()]);
    checker.register_trait_bound("T".into(), "Show".into());
    checker.register_trait_bound("U".into(), "Debug".into());
    checker.register_impl("Int".into(), "Show".into());
    checker.register_impl("Bool".into(), "Debug".into());

    let type_args = vec![
        ("T".into(), Type::Int),
        ("U".into(), Type::Bool),
    ];
    assert!(checker.check_all_trait_bounds("map", &type_args));
}

#[test]
fn verify_trait_not_registered() {
    let checker = Checker::new();

    assert!(checker.get_trait("UnknownTrait").is_none());
}

#[test]
fn verify_generic_params_not_registered() {
    let checker = Checker::new();

    assert!(checker.get_generic_params("unknown_fn").is_none());
}

#[test]
fn verify_trait_bounds_multiple_traits() {
    let mut checker = Checker::new();

    checker.register_trait_bound("T".into(), "Show".into());
    checker.register_trait_bound("T".into(), "Eq".into());
    checker.register_trait_bound("T".into(), "Ord".into());

    let bounds = checker.get_trait_bounds("T");
    assert_eq!(bounds.unwrap().len(), 3);
}

#[test]
fn verify_impl_registry_multiple_impls() {
    let mut checker = Checker::new();

    checker.register_impl("Int".into(), "Show".into());
    checker.register_impl("Int".into(), "Debug".into());
    checker.register_impl("String".into(), "Show".into());

    assert!(checker.has_impl("Int", "Show"));
    assert!(checker.has_impl("Int", "Debug"));
    assert!(checker.has_impl("String", "Show"));
    assert!(!checker.has_impl("String", "Debug"));
}

#[test]
fn verify_validate_where_clause_multiple_bounds_all_satisfied() {
    let mut checker = Checker::new();

    checker.register_trait_bound("T".into(), "Show".into());
    checker.register_trait_bound("T".into(), "Debug".into());
    checker.register_impl("Bool".into(), "Show".into());
    checker.register_impl("Bool".into(), "Debug".into());

    assert!(checker.validate_where_clause("T", "Bool"));
}

#[test]
fn verify_validate_where_clause_multiple_bounds_partially_satisfied() {
    let mut checker = Checker::new();

    checker.register_trait_bound("T".into(), "Show".into());
    checker.register_trait_bound("T".into(), "Debug".into());
    checker.register_impl("Bool".into(), "Show".into());

    assert!(!checker.validate_where_clause("T", "Bool"));
}


#[test]
fn verify_enforce_registers_generic_params() {
    let mut c = mk();
    c.enforce_where_clause("foo", &[gp("T")], &[], &[], span());
    assert!(c.get_generic_params("foo").is_some());
}

#[test]
fn verify_enforce_registers_bounds_from_where_clause() {
    let mut c = mk();
    c.enforce_where_clause("foo", &[gp("T")], &[wb("T", &["Show"])], &[], span());
    let bounds = c.get_trait_bounds("T").unwrap();
    assert!(bounds.contains(&"Show".to_string()));
}

#[test]
fn verify_enforce_multiple_bounds_registered() {
    let mut c = mk();
    c.enforce_where_clause("foo", &[gp("T")], &[wb("T", &["Show", "Clone"])], &[], span());
    let bounds = c.get_trait_bounds("T").unwrap();
    assert!(bounds.contains(&"Show".to_string()));
    assert!(bounds.contains(&"Clone".to_string()));
}

#[test]
fn verify_enforce_concrete_type_satisfies_bound_no_error() {
    let mut c = mk();
    c.register_impl("Int", "Show");
    c.enforce_where_clause(
        "foo", &[gp("T")], &[wb("T", &["Show"])],
        &[("T".into(), Type::Int)], span()
    );
    assert!(c.errors.is_empty());
}

#[test]
fn verify_enforce_concrete_type_missing_bound_reports_error() {
    let mut c = mk();
    // Int does NOT implement Show (not registered)
    c.enforce_where_clause(
        "foo", &[gp("T")], &[wb("T", &["Show"])],
        &[("T".into(), Type::Int)], span()
    );
    assert!(!c.errors.is_empty());
    assert!(c.errors[0].message.contains("Int"));
    assert!(c.errors[0].message.contains("Show"));
}

#[test]
fn verify_enforce_abstract_generic_is_skipped() {
    let mut c = mk();
    // T is still abstract (Named("T") and "T" is in generics) — no error
    c.enforce_where_clause(
        "foo", &[gp("T")], &[wb("T", &["Show"])],
        &[("T".into(), Type::Named("T".into()))], span()
    );
    assert!(c.errors.is_empty(), "abstract generic should not be checked against impl_registry");
}

#[test]
fn verify_enforce_multiple_params_all_satisfied() {
    let mut c = mk();
    c.register_impl("Int", "Show");
    c.register_impl("Bool", "Clone");
    c.enforce_where_clause(
        "foo",
        &[gp("T"), gp("U")],
        &[wb("T", &["Show"]), wb("U", &["Clone"])],
        &[("T".into(), Type::Int), ("U".into(), Type::Bool)],
        span()
    );
    assert!(c.errors.is_empty());
}

#[test]
fn verify_enforce_second_param_unsatisfied_reports_error() {
    let mut c = mk();
    c.register_impl("Int", "Show");
    // Bool does NOT implement Clone
    c.enforce_where_clause(
        "foo",
        &[gp("T"), gp("U")],
        &[wb("T", &["Show"]), wb("U", &["Clone"])],
        &[("T".into(), Type::Int), ("U".into(), Type::Bool)],
        span()
    );
    assert_eq!(c.errors.len(), 1);
    assert!(c.errors[0].message.contains("Bool"));
}

#[test]
fn verify_check_fn_decl_registers_generic_params() {
    let mut c = mk();
    let f = fn_decl("apply", vec![gp("T")], vec![wb("T", &["Show"])]);
    c.check_fn_decl(&f);
    assert!(c.get_generic_params("apply").is_some());
}

#[test]
fn verify_check_fn_decl_registers_where_bounds() {
    let mut c = mk();
    let f = fn_decl("apply", vec![gp("T")], vec![wb("T", &["Show"])]);
    c.check_fn_decl(&f);
    let bounds = c.get_trait_bounds("T").unwrap_or_default();
    assert!(bounds.contains(&"Show".to_string()));
}

#[test]
fn verify_check_fn_decl_no_where_clause_no_error() {
    let mut c = mk();
    let f = fn_decl("plain", vec![], vec![]);
    c.check_fn_decl(&f);
    assert!(c.errors.is_empty());
}

fn make_impl(for_ty: ast::Type, trait_name: Option<Vec<String>>, generics: Vec<ast::GenericParam>, where_clause: Vec<ast::WhereBound>) -> ast::Decl {
    ast::Decl::Impl { for_type: for_ty, trait_name, generics, where_clause, methods: vec![] }
}

#[test]
fn verify_impl_concrete_type_arg_satisfies_bound() {
    let mut c = mk();
    c.register_impl("Int", "Show");
    // impl<T: Show> Wrapper<Int>  →  T = Int, Int: Show ✓
    c.check_impl_decl(
        &ast::Type::Generic { name: "Wrapper".into(), args: vec![ast::Type::Named("Int".into())] },
        &None,
        &[gp("T")],
        &[wb("T", &["Show"])],
        &[],
    );
    assert!(c.errors.is_empty());
}

#[test]
fn verify_impl_concrete_type_arg_missing_bound_errors() {
    let mut c = mk();
    // Int does NOT implement Show
    c.check_impl_decl(
        &ast::Type::Generic { name: "Wrapper".into(), args: vec![ast::Type::Named("Int".into())] },
        &None,
        &[gp("T")],
        &[wb("T", &["Show"])],
        &[],
    );
    assert!(!c.errors.is_empty());
    assert!(c.errors[0].message.contains("Int"));
    assert!(c.errors[0].message.contains("Show"));
}

#[test]
fn verify_impl_abstract_generic_not_checked() {
    let mut c = mk();
    // impl<T: Show> Wrapper<T>  →  T is still abstract, no error
    c.check_impl_decl(
        &ast::Type::Generic { name: "Wrapper".into(), args: vec![ast::Type::Named("T".into())] },
        &None,
        &[gp("T")],
        &[wb("T", &["Show"])],
        &[],
    );
    assert!(c.errors.is_empty(), "abstract T should not be checked");
}

#[test]
fn verify_impl_registers_trait_association() {
    let mut c = mk();
    c.register_impl("Int", "Show");
    c.check_impl_decl(
        &ast::Type::Named("Int".into()),
        &Some(vec!["Display".into()]),
        &[],
        &[],
        &[],
    );
    assert!(c.has_impl("Int", "Display"));
}

#[test]
fn verify_impl_no_where_clause_no_error() {
    let mut c = mk();
    c.check_impl_decl(
        &ast::Type::Named("Int".into()),
        &None,
        &[],
        &[],
        &[],
    );
    assert!(c.errors.is_empty());
}

#[test]
fn verify_check_program_impl_where_satisfied_no_error() {
    let mut c = mk();
    c.register_impl("Int", "Show");
    let decls = vec![make_impl(
        ast::Type::Generic { name: "Box".into(), args: vec![ast::Type::Named("Int".into())] },
        None,
        vec![gp("T")],
        vec![wb("T", &["Show"])],
    )];
    c.check_program(&decls);
    assert!(c.errors.is_empty());
}

#[test]
fn verify_check_program_impl_where_unsatisfied_errors() {
    let mut c = mk();
    // Int does NOT implement Show — program should produce an error
    let decls = vec![make_impl(
        ast::Type::Generic { name: "Box".into(), args: vec![ast::Type::Named("Int".into())] },
        None,
        vec![gp("T")],
        vec![wb("T", &["Show"])],
    )];
    c.check_program(&decls);
    assert!(!c.errors.is_empty());
}

#[test]
fn verify_check_all_bounds_uses_correct_type_name() {
    let mut c = mk();
    c.register_generic_params("foo".into(), vec!["T".into()]);
    c.register_trait_bound("T".into(), "Show".into());
    c.register_impl("Int", "Show");
    // Type::Int should produce "Int", not "Int { .. }" or similar
    assert!(c.check_all_trait_bounds("foo", &[("T".into(), Type::Int)]));
}

#[test]
fn verify_check_all_bounds_named_type_uses_name() {
    let mut c = mk();
    c.register_generic_params("foo".into(), vec!["T".into()]);
    c.register_trait_bound("T".into(), "Eq".into());
    c.register_impl("MyStruct", "Eq");
    assert!(c.check_all_trait_bounds("foo", &[("T".into(), Type::Named("MyStruct".into()))]));
}

// Feature 22: trait method call resolution (uses non-reserved `Doubler` trait).

fn s2() -> ast::Span { ast::Span { line: 1, col: 1 } }
fn sp_pat(p: ast::Pattern) -> ast::Spanned<ast::Pattern> { ast::Spanned { node: p, span: s2() } }
fn sp_expr(e: ast::Expr) -> ast::Spanned<ast::Expr> { ast::Spanned { node: e, span: s2() } }

fn doubler_trait_decl(return_ty: ast::Type) -> ast::Decl {
    let sig = ast::FnSignature {
        name: "double".into(),
        generics: vec![],
        params: vec![ast::Param::SelfVal],
        effects: vec![],
        return_type: Some(return_ty),
        where_clause: vec![],
    };
    ast::Decl::Trait {
        is_pub: false,
        name: "Doubler".into(),
        generics: vec![],
        where_clause: vec![],
        items: vec![ast::TraitItem::Required(sig)],
    }
}

fn impl_doubler_for_int(method_ret: ast::Type, body: ast::Block) -> ast::Decl {
    ast::Decl::Impl {
        generics: vec![],
        trait_name: Some(vec!["Doubler".into()]),
        for_type: ast::Type::Named("Int".into()),
        where_clause: vec![],
        methods: vec![ast::FnDecl {
            attrs: vec![],
            is_pub: false,
            name: "double".into(),
            generics: vec![],
            params: vec![ast::Param::SelfVal],
            effects: vec![],
            return_type: Some(method_ret),
            where_clause: vec![],
            body,
        }],
    }
}

fn main_fn(ret_ty: ast::Type, ret: ast::Expr) -> ast::Decl {
    ast::Decl::Fn(ast::FnDecl {
        attrs: vec![],
        is_pub: false,
        name: "main".into(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(ret_ty),
        where_clause: vec![],
        body: ast::Block { stmts: vec![], ret: Some(Box::new(sp_expr(ret))) },
    })
}

#[test]
fn verify_method_call_resolves_via_impl() {
    let mut c = mk();
    let trait_decl = doubler_trait_decl(ast::Type::Named("Int".into()));
    let body = ast::Block {
        stmts: vec![],
        ret: Some(Box::new(sp_expr(ast::Expr::Binary {
            op: ast::BinaryOp::Mul,
            left: Box::new(sp_expr(ast::Expr::Identifier("self".into()))),
            right: Box::new(sp_expr(ast::Expr::Literal(ast::Literal::Int(2)))),
        }))),
    };
    let impl_decl = impl_doubler_for_int(ast::Type::Named("Int".into()), body);
    let main_call = ast::Expr::Call {
        callee: Box::new(sp_expr(ast::Expr::FieldAccess {
            base: Box::new(sp_expr(ast::Expr::Literal(ast::Literal::Int(5)))),
            field: "double".into(),
        })),
        args: vec![],
    };
    let decls = vec![trait_decl, impl_decl, main_fn(ast::Type::Named("Int".into()), main_call)];
    c.check_program(&decls);
    assert!(c.errors.is_empty(),
        "expected no errors, got: {:?}",
        c.errors.iter().map(|e| &e.message).collect::<Vec<_>>());
}

#[test]
fn verify_method_call_without_impl_errors() {
    let mut c = mk();
    // Trait declared, but no impl provided.
    let trait_decl = doubler_trait_decl(ast::Type::Named("Int".into()));
    let main_call = ast::Expr::Call {
        callee: Box::new(sp_expr(ast::Expr::FieldAccess {
            base: Box::new(sp_expr(ast::Expr::Literal(ast::Literal::Int(5)))),
            field: "double".into(),
        })),
        args: vec![],
    };
    let decls = vec![trait_decl, main_fn(ast::Type::Named("Int".into()), main_call)];
    c.check_program(&decls);
    assert!(c.errors.iter().any(|e| e.message.contains("No method 'double'")),
        "expected 'No method' error, got: {:?}",
        c.errors.iter().map(|e| &e.message).collect::<Vec<_>>());
}

#[test]
fn verify_bounded_generic_calls_trait_method() {
    let mut c = mk();
    let trait_decl = doubler_trait_decl(ast::Type::Named("Int".into()));
    let impl_body = ast::Block {
        stmts: vec![],
        ret: Some(Box::new(sp_expr(ast::Expr::Identifier("self".into())))),
    };
    let impl_decl = impl_doubler_for_int(ast::Type::Named("Int".into()), impl_body);

    // fn p<T>(x: T) -> Int where T: Doubler { x.double() }
    let p = ast::Decl::Fn(ast::FnDecl {
        attrs: vec![],
        is_pub: false,
        name: "p".into(),
        generics: vec![gp("T")],
        params: vec![ast::Param::Named {
            pattern: sp_pat(ast::Pattern::Bind("x".into())),
            ty: ast::Type::Named("T".into()),
        }],
        effects: vec![],
        return_type: Some(ast::Type::Named("Int".into())),
        where_clause: vec![wb("T", &["Doubler"])],
        body: ast::Block {
            stmts: vec![],
            ret: Some(Box::new(sp_expr(ast::Expr::Call {
                callee: Box::new(sp_expr(ast::Expr::FieldAccess {
                    base: Box::new(sp_expr(ast::Expr::Identifier("x".into()))),
                    field: "double".into(),
                })),
                args: vec![],
            }))),
        },
    });

    // fn main() -> Int { p(5) }
    let main_call = ast::Expr::Call {
        callee: Box::new(sp_expr(ast::Expr::Identifier("p".into()))),
        args: vec![sp_expr(ast::Expr::Literal(ast::Literal::Int(5)))],
    };

    let decls = vec![trait_decl, impl_decl, p, main_fn(ast::Type::Named("Int".into()), main_call)];
    c.check_program(&decls);
    assert!(c.errors.is_empty(),
        "expected no errors, got: {:?}",
        c.errors.iter().map(|e| &e.message).collect::<Vec<_>>());
}

#[test]
fn verify_reserved_trait_name_rejected() {
    // The four `@derive` traits (wiki §11) are reserved — declaring a user
    // `trait` with any of these names must produce a typeck error.
    for reserved in ["Show", "Eq", "Ord", "Clone"] {
        let mut c = mk();
        let decl = ast::Decl::Trait {
            is_pub: false,
            name: reserved.into(),
            generics: vec![],
            where_clause: vec![],
            items: vec![],
        };
        c.check_program(&[decl]);
        assert!(
            c.errors.iter().any(|e|
                e.message.contains("Cannot redefine built-in trait")
                && e.message.contains(reserved)),
            "expected reserved-name error for trait '{}', got: {:?}",
            reserved,
            c.errors.iter().map(|e| &e.message).collect::<Vec<_>>(),
        );
    }
}

#[test]
fn verify_impl_method_sig_mismatch_errors() {
    let mut c = mk();
    // Trait expects -> Int; impl provides -> Bool.
    let trait_decl = doubler_trait_decl(ast::Type::Named("Int".into()));
    let body = ast::Block {
        stmts: vec![],
        ret: Some(Box::new(sp_expr(ast::Expr::Literal(ast::Literal::Bool(true))))),
    };
    let impl_decl = impl_doubler_for_int(ast::Type::Named("Bool".into()), body);
    let decls = vec![trait_decl, impl_decl];
    c.check_program(&decls);
    assert!(c.errors.iter().any(|e| e.message.contains("does not match trait")),
        "expected signature mismatch error, got: {:?}",
        c.errors.iter().map(|e| &e.message).collect::<Vec<_>>());
}
