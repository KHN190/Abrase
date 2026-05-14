use ect::ast::{self, Pattern, Span, Spanned};
use ect::ty::Type;
use ect::typeck::Checker;

fn d_span() -> Span { Span::new(0, 0) }
fn sp<T>(node: T) -> Spanned<T> { Spanned { node, span: d_span() } }

// Special Scopes & Effects

#[test]
fn verify_scope_with_label() {
    let mut checker = Checker::new();
    let body = ast::Block {
        stmts: vec![],
        ret: Some(Box::new(sp(ast::Expr::Literal(ast::Literal::Int(42))))),
    };
    let expr = sp(ast::Expr::Scope {
        label: Some("outer".into()),
        options: None,
        body,
    });

    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Int);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_nested_scopes() {
    let mut checker = Checker::new();
    let inner_scope = sp(ast::Expr::Scope {
        label: Some("inner".into()),
        options: None,
        body: ast::Block {
            stmts: vec![],
            ret: Some(Box::new(sp(ast::Expr::Literal(ast::Literal::Bool(true))))),
        },
    });

    let outer_scope = sp(ast::Expr::Scope {
        label: Some("outer".into()),
        options: None,
        body: ast::Block {
            stmts: vec![sp(ast::Stmt::Expr(inner_scope))],
            ret: None,
        },
    });

    let ty = checker.infer_expr(&outer_scope);
    assert_eq!(ty, Type::Unit);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_region_with_label() {
    let mut checker = Checker::new();
    let body = ast::Block {
        stmts: vec![],
        ret: Some(Box::new(sp(ast::Expr::Literal(ast::Literal::Float(3.14))))),
    };
    let expr = sp(ast::Expr::Region {
        label: Some("heap".into()),
        body,
    });

    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Float);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_handle_return_arm() {
    let mut checker = Checker::new();
    let expr = sp(ast::Expr::Handle {
        expr: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(10)))),
        arms: vec![
            ast::HandleArm {
                kind: ast::HandleArmKind::Return,
                pattern: Some(sp(Pattern::Bind("x".into()))),
                body: sp(ast::Expr::Literal(ast::Literal::Int(42))),
            },
        ],
    });

    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Int);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_handle_exception_arm() {
    let mut checker = Checker::new();
    let expr = sp(ast::Expr::Handle {
        expr: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(1)))),
        arms: vec![
            ast::HandleArm {
                kind: ast::HandleArmKind::Exn,
                pattern: Some(sp(Pattern::Bind("e".into()))),
                body: sp(ast::Expr::Literal(ast::Literal::Int(0))),
            },
        ],
    });

    checker.infer_expr(&expr);
    assert_eq!(checker.errors.len(), 1);
    assert!(checker.errors[0].message.contains("no exn effect is active"));
}

#[test]
fn verify_handle_custom_effect() {
    let mut checker = Checker::new();
    let expr = sp(ast::Expr::Handle {
        expr: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(1)))),
        arms: vec![
            ast::HandleArm {
                kind: ast::HandleArmKind::Effect(vec!["logger".into(), "log".into()]),
                pattern: Some(sp(Pattern::Bind("msg".into()))),
                body: sp(ast::Expr::Literal(ast::Literal::Unit)),
            },
        ],
    });

    checker.infer_expr(&expr);
    assert_eq!(checker.errors.len(), 1);
    assert!(checker.errors[0].message.contains("not active"));
}

#[test]
fn verify_handle_multiple_arms_type_unification() {
    let mut checker = Checker::new();
    let expr = sp(ast::Expr::Handle {
        expr: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(1)))),
        arms: vec![
            ast::HandleArm {
                kind: ast::HandleArmKind::Return,
                pattern: Some(sp(Pattern::Bind("x".into()))),
                body: sp(ast::Expr::Literal(ast::Literal::Int(42))),
            },
            ast::HandleArm {
                kind: ast::HandleArmKind::Exn,
                pattern: Some(sp(Pattern::Bind("e".into()))),
                body: sp(ast::Expr::Literal(ast::Literal::Int(0))),
            },
        ],
    });

    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Int);
}

#[test]
fn verify_handle_arm_type_mismatch() {
    let mut checker = Checker::new();
    let expr = sp(ast::Expr::Handle {
        expr: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(1)))),
        arms: vec![
            ast::HandleArm {
                kind: ast::HandleArmKind::Return,
                pattern: Some(sp(Pattern::Bind("x".into()))),
                body: sp(ast::Expr::Literal(ast::Literal::Int(42))),
            },
            ast::HandleArm {
                kind: ast::HandleArmKind::Exn,
                pattern: Some(sp(Pattern::Bind("e".into()))),
                body: sp(ast::Expr::Literal(ast::Literal::String("error".into()))),
            },
        ],
    });

    checker.infer_expr(&expr);
    assert!(checker.errors.iter().any(|e| e.message.contains("Handle arm types do not match")));
}

#[test]
fn verify_scope_with_statements() {
    let mut checker = Checker::new();
    let body = ast::Block {
        stmts: vec![
            sp(ast::Stmt::Let {
                pattern: sp(Pattern::Bind("x".into())),
                is_mut: false,
                ty: None,
                value: sp(ast::Expr::Literal(ast::Literal::Int(5))),
            }),
            sp(ast::Stmt::Let {
                pattern: sp(Pattern::Bind("y".into())),
                is_mut: false,
                ty: None,
                value: sp(ast::Expr::Literal(ast::Literal::Int(10))),
            }),
        ],
        ret: Some(Box::new(sp(ast::Expr::Binary {
            op: ast::BinaryOp::Add,
            left: Box::new(sp(ast::Expr::Identifier("x".into()))),
            right: Box::new(sp(ast::Expr::Identifier("y".into()))),
        }))),
    };

    let expr = sp(ast::Expr::Scope {
        label: None,
        options: None,
        body,
    });

    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Int);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_region_with_statements() {
    let mut checker = Checker::new();
    let body = ast::Block {
        stmts: vec![
            sp(ast::Stmt::Let {
                pattern: sp(Pattern::Bind("ptr".into())),
                is_mut: false,
                ty: None,
                value: sp(ast::Expr::Literal(ast::Literal::Int(0))),
            }),
        ],
        ret: Some(Box::new(sp(ast::Expr::Identifier("ptr".into())))),
    };

    let expr = sp(ast::Expr::Region {
        label: Some("r".into()),
        body,
    });

    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Int);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_handle_without_arms() {
    let mut checker = Checker::new();
    let expr = sp(ast::Expr::Handle {
        expr: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(1)))),
        arms: vec![],
    });

    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Unknown);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_scope_effect_isolation() {
    let mut checker = Checker::new();
    let scope_expr = sp(ast::Expr::Scope {
        label: Some("s".into()),
        options: None,
        body: ast::Block {
            stmts: vec![],
            ret: Some(Box::new(sp(ast::Expr::Literal(ast::Literal::Bool(true))))),
        },
    });

    let ty = checker.infer_expr(&scope_expr);
    assert_eq!(ty, Type::Bool);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_region_effect_isolation() {
    let mut checker = Checker::new();
    let region_expr = sp(ast::Expr::Region {
        label: Some("r".into()),
        body: ast::Block {
            stmts: vec![],
            ret: Some(Box::new(sp(ast::Expr::Literal(ast::Literal::Char('a'))))),
        },
    });

    let ty = checker.infer_expr(&region_expr);
    assert_eq!(ty, Type::Char);
    assert!(checker.errors.is_empty());
}

// Infrastructure & Context Management

#[test]
fn verify_function_registry() {
    let mut checker = Checker::new();
    checker.register_function(
        "add".into(),
        vec![Type::Int, Type::Int],
        Type::Int,
    );

    let result = checker.get_function("add");
    assert!(result.is_some());
    let (params, ret) = result.unwrap();
    assert_eq!(params, vec![Type::Int, Type::Int]);
    assert_eq!(ret, Type::Int);
}

#[test]
fn verify_type_registry() {
    let mut checker = Checker::new();
    checker.register_type(
        "Point".into(),
        ast::TypeBody::Record(vec![]),
    );

    let result = checker.get_type("Point");
    assert!(result.is_some());
}

#[test]
fn verify_const_registry() {
    let mut checker = Checker::new();
    checker.register_const("PI".into(), Type::Float);

    let result = checker.get_const("PI");
    assert!(result.is_some());
    assert_eq!(result.unwrap(), Type::Float);
}

#[test]
fn verify_pattern_bind() {
    let mut checker = Checker::new();
    let pattern = sp(Pattern::Bind("x".into()));
    checker.check_pattern(&pattern, &Type::Int, d_span());

    let var_ty = checker.get_var("x", false, d_span());
    assert_eq!(var_ty, Type::Int);
}

#[test]
fn verify_pattern_wildcard() {
    let mut checker = Checker::new();
    let pattern = sp(Pattern::Wildcard);
    checker.check_pattern(&pattern, &Type::String, d_span());
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_pattern_literal_match() {
    let mut checker = Checker::new();
    let pattern = sp(Pattern::Literal(ast::Literal::Int(42)));
    checker.check_pattern(&pattern, &Type::Int, d_span());
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_pattern_literal_mismatch() {
    let mut checker = Checker::new();
    let pattern = sp(Pattern::Literal(ast::Literal::Int(42)));
    checker.check_pattern(&pattern, &Type::Bool, d_span());
    assert_eq!(checker.errors.len(), 1);
    assert!(checker.errors[0].message.contains("Pattern type mismatch"));
}

#[test]
fn verify_pattern_tuple_match() {
    let mut checker = Checker::new();
    let pattern = sp(Pattern::Tuple(vec![
        sp(Pattern::Bind("x".into())),
        sp(Pattern::Bind("y".into())),
    ]));
    let tuple_ty = Type::Tuple(vec![Type::Int, Type::Bool]);
    checker.check_pattern(&pattern, &tuple_ty, d_span());

    assert_eq!(checker.get_var("x", false, d_span()), Type::Int);
    assert_eq!(checker.get_var("y", false, d_span()), Type::Bool);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_pattern_tuple_length_mismatch() {
    let mut checker = Checker::new();
    let pattern = sp(Pattern::Tuple(vec![
        sp(Pattern::Bind("x".into())),
        sp(Pattern::Bind("y".into())),
    ]));
    let tuple_ty = Type::Tuple(vec![Type::Int]);
    checker.check_pattern(&pattern, &tuple_ty, d_span());
    assert_eq!(checker.errors.len(), 1);
    assert!(checker.errors[0].message.contains("Tuple pattern length mismatch"));
}

#[test]
fn verify_pattern_or() {
    let mut checker = Checker::new();
    let pattern = sp(Pattern::Or(vec![
        sp(Pattern::Bind("a".into())),
        sp(Pattern::Bind("b".into())),
    ]));
    checker.check_pattern(&pattern, &Type::Int, d_span());

    assert_eq!(checker.get_var("a", false, d_span()), Type::Int);
    assert_eq!(checker.get_var("b", false, d_span()), Type::Int);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_pattern_range_int() {
    let mut checker = Checker::new();
    let pattern = sp(Pattern::Range {
        start: Some(ast::Literal::Int(0)),
        end: Some(ast::Literal::Int(10)),
        inclusive: false,
    });
    checker.check_pattern(&pattern, &Type::Int, d_span());
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_pattern_range_non_int() {
    let mut checker = Checker::new();
    let pattern = sp(Pattern::Range {
        start: Some(ast::Literal::Int(0)),
        end: Some(ast::Literal::Int(10)),
        inclusive: false,
    });
    checker.check_pattern(&pattern, &Type::Bool, d_span());
    assert_eq!(checker.errors.len(), 1);
    assert!(checker.errors[0].message.contains("Range pattern requires Int"));
}

#[test]
fn verify_pattern_array() {
    let mut checker = Checker::new();
    let pattern = sp(Pattern::Array(vec![
        sp(Pattern::Bind("x".into())),
        sp(Pattern::Bind("y".into())),
    ]));
    let array_ty = Type::Named("Array<Int>".into());
    checker.check_pattern(&pattern, &array_ty, d_span());
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_pattern_array_wrong_type() {
    let mut checker = Checker::new();
    let pattern = sp(Pattern::Array(vec![sp(Pattern::Wildcard)]));
    checker.check_pattern(&pattern, &Type::Int, d_span());
    assert_eq!(checker.errors.len(), 1);
    assert!(checker.errors[0].message.contains("Expected array pattern"));
}

#[test]
fn verify_pattern_ref() {
    let mut checker = Checker::new();
    let pattern = sp(Pattern::Ref(Box::new(sp(Pattern::Bind("x".into())))));
    let ref_ty = Type::Reference { is_mut: false, inner: Box::new(Type::String) };
    checker.check_pattern(&pattern, &ref_ty, d_span());

    assert_eq!(checker.get_var("x", false, d_span()), Type::String);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_pattern_ref_non_reference() {
    let mut checker = Checker::new();
    let pattern = sp(Pattern::Ref(Box::new(sp(Pattern::Wildcard))));
    checker.check_pattern(&pattern, &Type::Int, d_span());
    assert_eq!(checker.errors.len(), 1);
    assert!(checker.errors[0].message.contains("Expected reference pattern"));
}

#[test]
fn verify_let_with_tuple_pattern() {
    let mut checker = Checker::new();
    let stmt = sp(ast::Stmt::Let {
        pattern: sp(Pattern::Tuple(vec![
            sp(Pattern::Bind("x".into())),
            sp(Pattern::Bind("y".into())),
        ])),
        is_mut: false,
        ty: None,
        value: sp(ast::Expr::Tuple(vec![
            sp(ast::Expr::Literal(ast::Literal::Int(1))),
            sp(ast::Expr::Literal(ast::Literal::Bool(true))),
        ])),
    });

    checker.check_stmt(&stmt);

    assert_eq!(checker.get_var("x", false, d_span()), Type::Int);
    assert_eq!(checker.get_var("y", false, d_span()), Type::Bool);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_pattern_record() {
    let mut checker = Checker::new();
    let pattern = sp(Pattern::Record {
        ty: vec!["Point".into()],
        fields: vec![
            ast::FieldPattern {
                name: "x".into(),
                pattern: Some(sp(Pattern::Bind("px".into()))),
            },
        ],
        rest: false,
    });
    checker.check_pattern(&pattern, &Type::Named("Point".into()), d_span());
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_pattern_variant() {
    let mut checker = Checker::new();
    let pattern = sp(Pattern::Variant {
        ty: vec!["Option".into()],
        args: vec![sp(Pattern::Bind("val".into()))],
    });
    checker.check_pattern(&pattern, &Type::Named("Option".into()), d_span());
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_nested_pattern_tuple_and_bind() {
    let mut checker = Checker::new();
    let pattern = sp(Pattern::Tuple(vec![
        sp(Pattern::Bind("x".into())),
        sp(Pattern::Tuple(vec![
            sp(Pattern::Bind("a".into())),
            sp(Pattern::Bind("b".into())),
        ])),
    ]));
    let tuple_ty = Type::Tuple(vec![
        Type::Int,
        Type::Tuple(vec![Type::Bool, Type::String]),
    ]);
    checker.check_pattern(&pattern, &tuple_ty, d_span());

    assert_eq!(checker.get_var("x", false, d_span()), Type::Int);
    assert_eq!(checker.get_var("a", false, d_span()), Type::Bool);
    assert_eq!(checker.get_var("b", false, d_span()), Type::String);
    assert!(checker.errors.is_empty());
}

// Region Escape & Borrow Checking Tests

#[test]
fn verify_push_pop_region() {
    let mut checker = Checker::new();

    assert!(checker.get_current_region().is_none());

    checker.push_region("region_a".into());
    assert_eq!(checker.get_current_region(), Some("region_a"));

    checker.push_region("region_b".into());
    assert_eq!(checker.get_current_region(), Some("region_b"));

    checker.pop_region();
    assert_eq!(checker.get_current_region(), Some("region_a"));
}

#[test]
fn verify_bind_reference_lifetime() {
    let mut checker = Checker::new();

    checker.bind_reference_lifetime("ref_x".into(), "region_a".into());

    let lifetime = checker.get_reference_lifetime("ref_x");
    assert_eq!(lifetime, Some("region_a".into()));
}

#[test]
fn verify_reference_lifetime_not_found() {
    let checker = Checker::new();

    let lifetime = checker.get_reference_lifetime("unknown_ref");
    assert!(lifetime.is_none());
}

#[test]
fn verify_check_escape_analysis_same_region() {
    let mut checker = Checker::new();

    let valid = checker.check_escape_analysis(Some("region_a"), Some("region_a"), d_span());
    assert!(valid);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_check_escape_analysis_different_regions() {
    let mut checker = Checker::new();

    let valid = checker.check_escape_analysis(Some("region_a"), Some("region_b"), d_span());
    assert!(!valid);
    assert!(!checker.errors.is_empty());
}

#[test]
fn verify_check_escape_analysis_none_regions() {
    let mut checker = Checker::new();

    let valid = checker.check_escape_analysis(None, None, d_span());
    assert!(valid);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_register_pattern_borrow() {
    let mut checker = Checker::new();

    checker.register_pattern_borrow("x".into(), "immut".into());
    checker.register_pattern_borrow("y".into(), "mut".into());

    let borrows_x = checker.get_pattern_borrows("x");
    let borrows_y = checker.get_pattern_borrows("y");

    assert!(borrows_x.is_some());
    assert!(borrows_y.is_some());
}

#[test]
fn verify_check_pattern_borrow_exclusivity_compatible() {
    let mut checker = Checker::new();

    checker.register_pattern_borrow("x".into(), "immut".into());
    checker.register_pattern_borrow("y".into(), "immut".into());

    let exclusive = checker.check_pattern_borrow_exclusivity(&["x", "y"]);
    assert!(exclusive);
}

#[test]
fn verify_check_pattern_borrow_exclusivity_conflict() {
    let mut checker = Checker::new();

    checker.register_pattern_borrow("x".into(), "mut".into());
    checker.register_pattern_borrow("y".into(), "mut".into());

    let exclusive = checker.check_pattern_borrow_exclusivity(&["x", "y"]);
    assert!(!exclusive);
}

#[test]
fn verify_check_pattern_borrow_exclusivity_immut_and_mut() {
    let mut checker = Checker::new();

    checker.register_pattern_borrow("x".into(), "immut".into());
    checker.register_pattern_borrow("y".into(), "mut".into());

    let exclusive = checker.check_pattern_borrow_exclusivity(&["x", "y"]);
    assert!(exclusive);
}

#[test]
fn verify_validate_reference_escape_same_region() {
    let mut checker = Checker::new();

    checker.bind_reference_lifetime("ref_x".into(), "region_a".into());
    let valid = checker.validate_reference_escape("ref_x", Some("region_a"));
    assert!(valid);
}

#[test]
fn verify_validate_reference_escape_different_regions() {
    let mut checker = Checker::new();

    checker.bind_reference_lifetime("ref_x".into(), "region_a".into());
    let valid = checker.validate_reference_escape("ref_x", Some("region_b"));
    assert!(!valid);
}

#[test]
fn verify_validate_reference_escape_none_lifetime() {
    let checker = Checker::new();

    let valid = checker.validate_reference_escape("unknown_ref", Some("region_a"));
    assert!(valid);
}

#[test]
fn verify_clear_region_context() {
    let mut checker = Checker::new();

    checker.push_region("region_a".into());
    checker.bind_reference_lifetime("ref_x".into(), "region_a".into());
    checker.register_pattern_borrow("x".into(), "immut".into());

    assert_eq!(checker.get_current_region(), Some("region_a"));
    assert!(checker.get_reference_lifetime("ref_x").is_some());
    assert!(checker.get_pattern_borrows("x").is_some());

    checker.clear_region_context();

    assert!(checker.get_current_region().is_none());
    assert!(checker.get_reference_lifetime("ref_x").is_none());
    assert!(checker.get_pattern_borrows("x").is_none());
}

#[test]
fn verify_region_stack_multiple_levels() {
    let mut checker = Checker::new();

    checker.push_region("outer".into());
    checker.push_region("middle".into());
    checker.push_region("inner".into());

    assert_eq!(checker.get_current_region(), Some("inner"));

    checker.pop_region();
    assert_eq!(checker.get_current_region(), Some("middle"));

    checker.pop_region();
    assert_eq!(checker.get_current_region(), Some("outer"));

    checker.pop_region();
    assert!(checker.get_current_region().is_none());
}

// Visibility & Module Scoping

#[test]
fn verify_push_pop_module() {
    let mut checker = Checker::new();

    assert_eq!(checker.get_current_module(), vec!["root"]);

    checker.push_module("io".into());
    assert_eq!(checker.get_current_module(), vec!["root", "io"]);

    checker.push_module("file".into());
    assert_eq!(checker.get_current_module(), vec!["root", "io", "file"]);

    checker.pop_module();
    assert_eq!(checker.get_current_module(), vec!["root", "io"]);

    checker.pop_module();
    assert_eq!(checker.get_current_module(), vec!["root"]);
}

#[test]
fn verify_pop_module_does_not_pop_root() {
    let mut checker = Checker::new();

    assert_eq!(checker.get_current_module(), vec!["root"]);

    checker.pop_module();
    assert_eq!(checker.get_current_module(), vec!["root"]);
}

#[test]
fn verify_set_current_module() {
    let mut checker = Checker::new();

    checker.set_current_module(vec!["network".into(), "http".into()]);
    assert_eq!(checker.get_current_module(), vec!["network", "http"]);
}

#[test]
fn verify_mark_public() {
    let mut checker = Checker::new();
    checker.push_module("io".into());

    checker.mark_public("Read".into());

    let public_items = checker.get_public_items();
    assert!(public_items.iter().any(|item| item.contains("Read")));
}

#[test]
fn verify_mark_private() {
    let mut checker = Checker::new();
    checker.push_module("io".into());

    checker.mark_private("read_impl".into());

    let private_items = checker.get_private_items();
    assert!(private_items.iter().any(|item| item.contains("read_impl")));
}

#[test]
fn verify_is_public_in_same_module() {
    let mut checker = Checker::new();
    checker.push_module("io".into());

    checker.mark_public("Read".into());

    assert!(checker.is_public("Read"));
}

#[test]
fn verify_is_public_from_root() {
    let mut checker = Checker::new();
    checker.push_module("io".into());

    checker.mark_public("Read".into());

    // Switch to root module
    checker.set_current_module(vec!["root".into()]);

    assert!(checker.is_public("Read"));
}

#[test]
fn verify_is_private_item() {
    let mut checker = Checker::new();
    checker.push_module("io".into());

    checker.mark_private("internal_buffer".into());

    assert!(!checker.is_public("internal_buffer"));
}

#[test]
fn verify_is_accessible_same_module() {
    let mut checker = Checker::new();
    checker.set_current_module(vec!["io".into()]);

    let item_module = vec!["io".into()];
    assert!(checker.is_accessible("Read", &item_module));
}

#[test]
fn verify_is_accessible_public_item() {
    let mut checker = Checker::new();
    checker.push_module("io".into());
    checker.mark_public("Read".into());

    // Switch to different module
    checker.set_current_module(vec!["root".into(), "net".into()]);

    assert!(checker.is_accessible("Read", &["root".into(), "io".into()]));
}

#[test]
fn verify_is_accessible_private_item_different_module() {
    let mut checker = Checker::new();
    checker.push_module("io".into());
    checker.mark_private("internal_buffer".into());

    // Switch to different module
    checker.set_current_module(vec!["root".into(), "net".into()]);

    assert!(!checker.is_accessible("internal_buffer", &["root".into(), "io".into()]));
}

#[test]
fn verify_validate_visibility_public() {
    let mut checker = Checker::new();
    checker.push_module("io".into());
    checker.mark_public("Read".into());

    let span = Span::new(1, 1);
    let result = checker.validate_visibility("Read", &["root".into(), "io".into()], span);

    assert!(result);
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_validate_visibility_private_from_different_module() {
    let mut checker = Checker::new();
    checker.push_module("io".into());
    checker.mark_private("internal_buffer".into());

    // Switch to different module
    checker.set_current_module(vec!["net".into()]);

    let span = Span::new(1, 1);
    let result = checker.validate_visibility("internal_buffer", &["io".into()], span);

    assert!(!result);
    assert_eq!(checker.errors.len(), 1);
}

#[test]
fn verify_get_public_items() {
    let mut checker = Checker::new();
    checker.push_module("io".into());

    checker.mark_public("Read".into());
    checker.mark_public("Write".into());

    let public_items = checker.get_public_items();
    assert_eq!(public_items.len(), 2);
}

#[test]
fn verify_get_private_items() {
    let mut checker = Checker::new();
    checker.push_module("io".into());

    checker.mark_private("buffer_impl".into());
    checker.mark_private("internal_read".into());

    let private_items = checker.get_private_items();
    assert_eq!(private_items.len(), 2);
}

#[test]
fn verify_clear_visibility_context() {
    let mut checker = Checker::new();
    checker.push_module("io".into());
    checker.mark_public("Read".into());
    checker.mark_private("buffer".into());

    assert_eq!(checker.get_current_module(), vec!["root", "io"]);
    assert_eq!(checker.get_public_items().len(), 1);
    assert_eq!(checker.get_private_items().len(), 1);

    checker.clear_visibility_context();

    assert_eq!(checker.get_current_module(), vec!["root"]);
    assert_eq!(checker.get_public_items().len(), 0);
    assert_eq!(checker.get_private_items().len(), 0);
}

#[test]
fn verify_module_hierarchy() {
    let mut checker = Checker::new();

    checker.push_module("io".into());
    assert_eq!(checker.get_current_module(), vec!["root", "io"]);

    checker.push_module("file".into());
    assert_eq!(checker.get_current_module(), vec!["root", "io", "file"]);

    checker.mark_public("FileReader".into());

    checker.pop_module();
    let module = checker.get_current_module();
    assert_eq!(module.len(), 2);
}

#[test]
fn verify_multiple_modules_visibility() {
    let mut checker = Checker::new();

    // Module io
    checker.push_module("io".into());
    checker.mark_public("Read".into());
    checker.pop_module();

    // Module net
    checker.push_module("net".into());
    checker.mark_public("Connection".into());
    checker.pop_module();

    let public_items = checker.get_public_items();
    assert_eq!(public_items.len(), 2);
}

#[test]
fn verify_accessibility_within_module_hierarchy() {
    let mut checker = Checker::new();

    // Create module hierarchy: root -> io
    checker.set_current_module(vec!["root".into(), "io".into()]);

    // Mark item as public in same module
    let io_module = vec!["root".into(), "io".into()];

    // Check accessibility (items in same module are always accessible)
    assert!(checker.is_accessible("SomeFile", &io_module));
}

#[test]
fn verify_visibility_with_qualified_names() {
    let mut checker = Checker::new();

    checker.push_module("io".into());
    checker.mark_public("BufferedReader".into());

    // Verify qualified name
    let public_items = checker.get_public_items();
    let has_qualified = public_items.iter()
        .any(|item| item.contains("io") && item.contains("BufferedReader"));

    assert!(has_qualified);
}

#[test]
fn verify_override_private_to_public() {
    let mut checker = Checker::new();
    checker.push_module("io".into());

    // First mark as private
    checker.mark_private("Item".into());
    let private_items = checker.get_private_items();
    assert!(private_items.len() > 0);

    // Then mark as public (should override)
    checker.mark_public("Item".into());
    let private_items_after = checker.get_private_items();
    assert!(private_items_after.len() == 0);

    let public_items = checker.get_public_items();
    assert!(public_items.len() > 0);
}

// Qualified Name Resolution

#[test]
fn verify_register_qualified_name() {
    let mut checker = Checker::new();

    let path = vec!["root".into(), "io".into(), "File".into()];
    checker.register_qualified_name("File".into(), path.clone());

    let resolutions = checker.get_all_resolutions("File");
    assert_eq!(resolutions.len(), 1);
    assert_eq!(resolutions[0], path);
}

#[test]
fn verify_resolve_qualified_name_fully_qualified() {
    let mut checker = Checker::new();

    let path = vec!["root".into(), "io".into(), "File".into()];
    checker.register_qualified_name("File".into(), path.clone());

    // Resolve fully qualified name from root
    let resolved = checker.resolve_qualified_name(&path);
    assert_eq!(resolved, Some(path.clone()));
}

#[test]
fn verify_resolve_qualified_name_relative() {
    let mut checker = Checker::new();

    // Register File in root.io.File
    let path = vec!["root".into(), "io".into(), "File".into()];
    checker.register_qualified_name("File".into(), path.clone());

    // Set current module to root
    checker.set_current_module(vec!["root".into()]);

    // Resolve relative path "io.File" from root
    let resolved = checker.resolve_qualified_name(&["io".into(), "File".into()]);
    assert_eq!(resolved, Some(path));
}

#[test]
fn verify_resolve_qualified_name_from_submodule() {
    let mut checker = Checker::new();

    // Register types in module hierarchy
    let file_path = vec!["root".into(), "io".into(), "File".into()];
    checker.register_qualified_name("File".into(), file_path.clone());

    // Set current module to root.io
    checker.set_current_module(vec!["root".into(), "io".into()]);

    // Try to resolve just "File" from root.io
    let resolved = checker.resolve_name("File");
    assert_eq!(resolved, Some(file_path));
}

#[test]
fn verify_resolve_name_simple() {
    let mut checker = Checker::new();

    let path = vec!["root".into(), "io".into(), "Read".into()];
    checker.register_qualified_name("Read".into(), path.clone());

    let resolved = checker.resolve_name("Read");
    assert_eq!(resolved, Some(path));
}

#[test]
fn verify_resolve_name_not_found() {
    let checker = Checker::new();

    let resolved = checker.resolve_name("NonExistent");
    assert_eq!(resolved, None);
}

#[test]
fn verify_is_name_resolvable() {
    let mut checker = Checker::new();

    let path = vec!["root".into(), "io".into(), "Error".into()];
    checker.register_qualified_name("Error".into(), path.clone());

    assert!(checker.is_name_resolvable(&["root".into(), "io".into(), "Error".into()]));
}

#[test]
fn verify_is_name_resolvable_false() {
    let checker = Checker::new();

    assert!(!checker.is_name_resolvable(&["unknown".into(), "Type".into()]));
}

#[test]
fn verify_qualified_name_resolution_multiple_paths() {
    let mut checker = Checker::new();

    // Register same simple name with different paths (overloading)
    let path1 = vec!["root".into(), "io".into(), "Error".into()];
    let path2 = vec!["root".into(), "net".into(), "Error".into()];

    checker.register_qualified_name("Error".into(), path1.clone());
    checker.register_qualified_name("Error".into(), path2.clone());

    let resolutions = checker.get_all_resolutions("Error");
    assert_eq!(resolutions.len(), 2);
}

#[test]
fn verify_resolve_qualified_name_nested_path() {
    let mut checker = Checker::new();

    let path = vec!["root".into(), "io".into(), "file".into(), "Reader".into()];
    checker.register_qualified_name("Reader".into(), path.clone());

    let resolved = checker.resolve_qualified_name(&["io".into(), "file".into(), "Reader".into()]);
    assert!(resolved.is_some());
}

#[test]
fn verify_qualified_name_with_module_context() {
    let mut checker = Checker::new();

    // Register in root.io
    let path = vec!["root".into(), "io".into(), "Write".into()];
    checker.register_qualified_name("Write".into(), path.clone());

    // Access from root module
    checker.set_current_module(vec!["root".into()]);
    let resolved = checker.resolve_name("Write");
    assert_eq!(resolved, Some(path.clone()));

    // Access from root.io module
    checker.set_current_module(vec!["root".into(), "io".into()]);
    let resolved = checker.resolve_name("Write");
    assert_eq!(resolved, Some(path.clone()));
}

#[test]
fn verify_resolve_qualified_name_from_different_module() {
    let mut checker = Checker::new();

    // Register in root.io
    let io_path = vec!["root".into(), "io".into(), "Stream".into()];
    checker.register_qualified_name("Stream".into(), io_path.clone());

    // From root.net module, simple name resolution returns the registered path
    checker.set_current_module(vec!["root".into(), "net".into()]);

    let resolved = checker.resolve_name("Stream");
    // Simple name resolution returns the first registered path
    assert_eq!(resolved, Some(io_path));
}

#[test]
fn verify_resolve_full_path_from_different_module() {
    let mut checker = Checker::new();

    // Register in root.io
    let io_path = vec!["root".into(), "io".into(), "Connection".into()];
    checker.register_qualified_name("Connection".into(), io_path.clone());

    // From root.net, can still resolve with full path
    checker.set_current_module(vec!["root".into(), "net".into()]);
    let resolved = checker.resolve_qualified_name(&["root".into(), "io".into(), "Connection".into()]);
    assert_eq!(resolved, Some(io_path));
}

#[test]
fn verify_clear_name_resolution() {
    let mut checker = Checker::new();

    let path = vec!["root".into(), "io".into(), "File".into()];
    checker.register_qualified_name("File".into(), path);

    assert!(checker.resolve_name("File").is_some());

    checker.clear_name_resolution();

    assert_eq!(checker.resolve_name("File"), None);
}

#[test]
fn verify_qualified_name_hierarchy_traversal() {
    let mut checker = Checker::new();

    // Create a hierarchy: root.std.io
    let file_path = vec!["root".into(), "std".into(), "io".into(), "File".into()];
    checker.register_qualified_name("File".into(), file_path.clone());

    // Set current to root.std
    checker.set_current_module(vec!["root".into(), "std".into()]);

    // Resolve relative to current module
    let resolved = checker.resolve_qualified_name(&["io".into(), "File".into()]);
    assert_eq!(resolved, Some(file_path));
}

#[test]
fn verify_multiple_qualified_names_same_module() {
    let mut checker = Checker::new();

    // Register multiple items in root.io
    let read_path = vec!["root".into(), "io".into(), "Read".into()];
    let write_path = vec!["root".into(), "io".into(), "Write".into()];

    checker.register_qualified_name("Read".into(), read_path.clone());
    checker.register_qualified_name("Write".into(), write_path.clone());

    // Both should be resolvable
    assert_eq!(checker.resolve_name("Read"), Some(read_path));
    assert_eq!(checker.resolve_name("Write"), Some(write_path));
}

#[test]
fn verify_qualified_name_resolution_order() {
    let mut checker = Checker::new();

    // Register same name with different fully qualified paths
    let path1 = vec!["root".into(), "io".into(), "Error".into()];
    let path2 = vec!["root".into(), "net".into(), "Error".into()];

    checker.register_qualified_name("Error".into(), path1.clone());
    checker.register_qualified_name("Error".into(), path2);

    // First registered should be returned by resolve_name
    let resolved = checker.resolve_name("Error");
    assert_eq!(resolved, Some(path1));
}

#[test]
fn verify_resolve_with_deeply_nested_module() {
    let mut checker = Checker::new();

    let path = vec![
        "root".into(),
        "sys".into(),
        "io".into(),
        "file".into(),
        "Reader".into(),
    ];
    checker.register_qualified_name("Reader".into(), path.clone());

    // Set to root.sys.io
    checker.set_current_module(vec!["root".into(), "sys".into(), "io".into()]);

    // Resolve relative path
    let resolved = checker.resolve_qualified_name(&["file".into(), "Reader".into()]);
    assert_eq!(resolved, Some(path));
}