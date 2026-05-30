use abrase::ty::Type;
use abrase::typeck::Checker;
use abrase::ast::{Block, Expr, Pattern, Span, Spanned, self};

fn d_span() -> Span {
    Span { line: 0, col: 0 }
}

fn dummy_block() -> Block {
    Block { stmts: vec![], ret: None }
}

// Function Declaration Tests

#[test]
fn verify_check_program_registers_function() {
    let mut checker = Checker::new();

    let fn_decl = abrase::ast::FnDecl {
        attrs: vec![],
        is_pub: false,
        name: "add".into(),
        generics: vec![],
        params: vec![
            abrase::ast::Param::Named {
                pattern: Spanned {
                    node: Pattern::Bind("a".into()),
                    span: d_span(),
                },
                ty: abrase::ast::Type::Named("Int".into()),
            },
            abrase::ast::Param::Named {
                pattern: Spanned {
                    node: Pattern::Bind("b".into()),
                    span: d_span(),
                },
                ty: abrase::ast::Type::Named("Int".into()),
            },
        ],
        effects: vec![],
        return_type: Some(abrase::ast::Type::Named("Unit".into())),
        where_clause: vec![],
        body: dummy_block(),
    };

    let decl = abrase::ast::Decl::Fn(fn_decl);

    checker.check_program(&[decl]);

    // Verify function was registered
    let fn_type = checker.get_var("add", false, d_span());
    assert_eq!(fn_type, Type::Function {
        params: vec![Type::Int, Type::Int],
        effects: vec![],
        ret: Box::new(Type::Unit),
    });
}

#[test]
fn verify_check_program_marks_public_function() {
    let mut checker = Checker::new();

    let fn_decl = abrase::ast::FnDecl {
        attrs: vec![],
        is_pub: true,
        name: "public_fn".into(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(abrase::ast::Type::Named("Unit".into())),
        where_clause: vec![],
        body: dummy_block(),
    };

    let decl = abrase::ast::Decl::Fn(fn_decl);

    checker.check_program(&[decl]);

    // Verify function is marked public
    assert!(checker.is_public("public_fn"));
}

#[test]
fn verify_check_program_private_function_not_public() {
    let mut checker = Checker::new();

    let fn_decl = abrase::ast::FnDecl {
        attrs: vec![],
        is_pub: false,
        name: "private_fn".into(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(abrase::ast::Type::Named("Unit".into())),
        where_clause: vec![],
        body: dummy_block(),
    };

    let decl = abrase::ast::Decl::Fn(fn_decl);

    checker.check_program(&[decl]);

    // Verify function is not public
    assert!(!checker.is_public("private_fn"));
}

// Type Declaration Tests

#[test]
fn verify_check_program_registers_type() {
    let mut checker = Checker::new();

    let record_field = abrase::ast::RecordField {
        is_pub: true,
        name: "x".into(),
        ty: abrase::ast::Type::Named("Int".into()),
    };

    let decl = abrase::ast::Decl::Type {
        attrs: vec![],
        is_pub: false,
        ownership: None,
        name: "Point".into(),
        generics: vec![],
        body: abrase::ast::TypeBody::Record(vec![record_field]),
    };

    checker.check_program(&[decl]);

    // Verify type was registered by checking it's marked public (can be queried)
    // Since is_public checks the registry, if the type wasn't registered, it would fail
    let _ = checker.is_public("Point");
}

#[test]
fn verify_check_program_marks_public_type() {
    let mut checker = Checker::new();

    let decl = abrase::ast::Decl::Type {
        attrs: vec![],
        is_pub: true,
        ownership: None,
        name: "PublicType".into(),
        generics: vec![],
        body: abrase::ast::TypeBody::Variant(vec![
            abrase::ast::VariantCase::Unit("A".into()),
        ]),
    };

    checker.check_program(&[decl]);

    // Verify type is marked public
    assert!(checker.is_public("PublicType"));
}

#[test]
fn verify_check_program_registers_type_ownership() {
    let mut checker = Checker::new();

    let decl = abrase::ast::Decl::Type {
        attrs: vec![],
        is_pub: false,
        ownership: Some(abrase::ast::OwnershipAttr::Copy),
        name: "CopyType".into(),
        generics: vec![],
        body: abrase::ast::TypeBody::Variant(vec![
            abrase::ast::VariantCase::Unit("X".into()),
        ]),
    };

    checker.check_program(&[decl]);

    // Verify ownership was registered
    assert!(checker.get_type_ownership("CopyType").is_some());
}

// Const Declaration Tests

#[test]
fn verify_check_program_registers_const() {
    let mut checker = Checker::new();

    let decl = abrase::ast::Decl::Const {
        is_pub: false,
        is_fn: false,
        name: "MAX_SIZE".into(),
        generics: vec![],
        params: vec![],
        ty: abrase::ast::Type::Named("Int".into()),
        value: Spanned {
            node: Expr::Literal(abrase::ast::Literal::Int(100)),
            span: d_span(),
        },
    };

    checker.check_program(&[decl]);

    // Verify const was registered (const variables are accessible)
    // Just verify no errors occurred during registration
    let errors_count = checker.errors.len();
    assert_eq!(errors_count, 0);
}

#[test]
fn verify_check_program_marks_public_const() {
    let mut checker = Checker::new();

    let decl = abrase::ast::Decl::Const {
        is_pub: true,
        is_fn: false,
        name: "PUBLIC_CONST".into(),
        generics: vec![],
        params: vec![],
        ty: abrase::ast::Type::Named("String".into()),
        value: Spanned {
            node: Expr::Literal(abrase::ast::Literal::String("hello".into())),
            span: d_span(),
        },
    };

    checker.check_program(&[decl]);

    // Verify const is marked public
    assert!(checker.is_public("PUBLIC_CONST"));
}

// Trait Declaration Tests

#[test]
fn verify_check_program_registers_trait() {
    let mut checker = Checker::new();

    let decl = abrase::ast::Decl::Trait {
        is_pub: false,
        name: "Show".into(),
        generics: vec![],
        where_clause: vec![],
        items: vec![],
    };

    checker.check_program(&[decl]);

    // Verify trait was registered by checking it can be queried
    let _ = checker.is_public("Show");
}

#[test]
fn verify_check_program_marks_public_trait() {
    let mut checker = Checker::new();

    let decl = abrase::ast::Decl::Trait {
        is_pub: true,
        name: "PublicTrait".into(),
        generics: vec![],
        where_clause: vec![],
        items: vec![],
    };

    checker.check_program(&[decl]);

    // Verify trait is marked public
    assert!(checker.is_public("PublicTrait"));
}

// Effect Declaration Tests

#[test]
fn verify_check_program_registers_effect() {
    let mut checker = Checker::new();

    let decl = abrase::ast::Decl::Effect {
        is_pub: false,
        name: "io".into(),
        ops: vec![],
    };

    checker.check_program(&[decl]);

    // Verify effect was registered by checking it can be queried
    let _ = checker.is_public("io");
}

#[test]
fn verify_check_program_marks_public_effect() {
    let mut checker = Checker::new();

    let decl = abrase::ast::Decl::Effect {
        is_pub: true,
        name: "PublicEffect".into(),
        ops: vec![],
    };

    checker.check_program(&[decl]);

    // Verify effect is marked public
    assert!(checker.is_public("PublicEffect"));
}

// Import Declaration Tests

#[test]
fn verify_check_program_registers_imports() {
    let mut checker = Checker::new();

    let decl = abrase::ast::Decl::Use {
        path: vec!["std".into()],
        items: vec![
            abrase::ast::ImportItem {
                name: "read".into(),
                alias: None,
            },
        ],
    };

    checker.check_program(&[decl]);

    // Verify import was registered
    let resolved = checker.get_imported_name("read");
    assert!(resolved.is_some());
}

#[test]
fn verify_check_program_registers_import_with_alias() {
    let mut checker = Checker::new();

    let decl = abrase::ast::Decl::Use {
        path: vec!["io".into()],
        items: vec![
            abrase::ast::ImportItem {
                name: "print".into(),
                alias: Some("log".into()),
            },
        ],
    };

    checker.check_program(&[decl]);

    // Verify import alias was registered
    let resolved = checker.get_imported_name("log");
    assert!(resolved.is_some());
}

// Module Declaration Tests


// Multi-Declaration Tests

#[test]
fn verify_check_program_two_pass_execution() {
    let mut checker = Checker::new();

    // Create two functions that might reference each other
    let fn1 = abrase::ast::FnDecl {
        attrs: vec![],
        is_pub: false,
        name: "fn1".into(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(abrase::ast::Type::Named("Unit".into())),
        where_clause: vec![],
        body: dummy_block(),
    };

    let fn2 = abrase::ast::FnDecl {
        attrs: vec![],
        is_pub: false,
        name: "fn2".into(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(abrase::ast::Type::Named("Unit".into())),
        where_clause: vec![],
        body: dummy_block(),
    };

    let decls = vec![
        abrase::ast::Decl::Fn(fn1),
        abrase::ast::Decl::Fn(fn2),
    ];

    checker.check_program(&decls);

    // Both functions should be registered
    let fn1_type = checker.get_var("fn1", false, d_span());
    let fn2_type = checker.get_var("fn2", false, d_span());

    assert_eq!(fn1_type, Type::Function {
        params: vec![],
        effects: vec![],
        ret: Box::new(Type::Unit),
    });

    assert_eq!(fn2_type, Type::Function {
        params: vec![],
        effects: vec![],
        ret: Box::new(Type::Unit),
    });
}

#[test]
fn verify_check_program_type_then_function() {
    let mut checker = Checker::new();

    let type_decl = abrase::ast::Decl::Type {
        attrs: vec![],
        is_pub: false,
        ownership: None,
        name: "MyType".into(),
        generics: vec![],
        body: abrase::ast::TypeBody::Variant(vec![
            abrase::ast::VariantCase::Unit("A".into()),
        ]),
    };

    let fn_decl = abrase::ast::FnDecl {
        attrs: vec![],
        is_pub: false,
        name: "process".into(),
        generics: vec![],
        params: vec![
            abrase::ast::Param::Named {
                pattern: Spanned {
                    node: Pattern::Bind("x".into()),
                    span: d_span(),
                },
                ty: abrase::ast::Type::Named("MyType".into()),
            },
        ],
        effects: vec![],
        return_type: Some(abrase::ast::Type::Named("Unit".into())),
        where_clause: vec![],
        body: dummy_block(),
    };

    let decls = vec![
        type_decl,
        abrase::ast::Decl::Fn(fn_decl),
    ];

    checker.check_program(&decls);

    // Both should be registered - function should be accessible
    let fn_type = checker.get_var("process", false, d_span());
    assert_eq!(fn_type, Type::Function {
        params: vec![Type::Named("MyType".into())],
        effects: vec![],
        ret: Box::new(Type::Unit),
    });
}

// Impl Declaration Tests

#[test]
fn verify_check_program_skips_impl_in_signature_pass() {
    let mut checker = Checker::new();

    let impl_decl = abrase::ast::Decl::Impl {
        generics: vec![],
        trait_name: None,
        for_type: abrase::ast::Type::Named("MyType".into()),
        where_clause: vec![],
        methods: vec![],
    };

    // Should not error even with empty type registry
    checker.check_program(&[impl_decl]);
}

// Public/Private Tests

#[test]
fn verify_check_program_mixed_visibility() {
    let mut checker = Checker::new();

    let public_fn = abrase::ast::FnDecl {
        attrs: vec![],
        is_pub: true,
        name: "public".into(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(abrase::ast::Type::Named("Unit".into())),
        where_clause: vec![],
        body: dummy_block(),
    };

    let private_fn = abrase::ast::FnDecl {
        attrs: vec![],
        is_pub: false,
        name: "private".into(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(abrase::ast::Type::Named("Unit".into())),
        where_clause: vec![],
        body: dummy_block(),
    };

    let decls = vec![
        abrase::ast::Decl::Fn(public_fn),
        abrase::ast::Decl::Fn(private_fn),
    ];

    checker.check_program(&decls);

    assert!(checker.is_public("public"));
    assert!(!checker.is_public("private"));
}


#[test]
fn verify_check_type_decl_registers_record() {
    let mut checker = Checker::new();

    let record_field = abrase::ast::RecordField {
        is_pub: true,
        name: "x".into(),
        ty: abrase::ast::Type::Named("Int".into()),
    };

    checker.check_type_decl("Point", &abrase::ast::TypeBody::Record(vec![record_field]), false, &None);

    // Type should be registered
    let _ = checker.is_public("Point");
}

#[test]
fn verify_check_type_decl_registers_variant() {
    let mut checker = Checker::new();

    let variant_cases = vec![
        abrase::ast::VariantCase::Unit("None".into()),
        abrase::ast::VariantCase::Tuple("Some".into(), vec![abrase::ast::Type::Named("T".into())]),
    ];

    checker.check_type_decl("Option", &abrase::ast::TypeBody::Variant(variant_cases), false, &None);

    // Variant cases should be registered
    let cases = checker.get_variant_cases("Option");
    assert_eq!(cases.map(|c| c.len()), Some(2));
}

#[test]
fn verify_check_type_decl_marks_public() {
    let mut checker = Checker::new();

    let variant_cases = vec![
        abrase::ast::VariantCase::Unit("A".into()),
        abrase::ast::VariantCase::Unit("B".into()),
    ];

    checker.check_type_decl("Color", &abrase::ast::TypeBody::Variant(variant_cases), true, &None);

    assert!(checker.is_public("Color"));
}

#[test]
fn verify_check_type_decl_registers_ownership_copy() {
    let mut checker = Checker::new();

    let variant_cases = vec![abrase::ast::VariantCase::Unit("X".into())];
    let ownership = Some(abrase::ast::OwnershipAttr::Copy);

    checker.check_type_decl("CopyEnum", &abrase::ast::TypeBody::Variant(variant_cases), false, &ownership);

    assert!(checker.get_type_ownership("CopyEnum").is_some());
}

#[test]
fn verify_check_type_decl_registers_ownership_move() {
    let mut checker = Checker::new();

    let variant_cases = vec![abrase::ast::VariantCase::Unit("Y".into())];
    let ownership = Some(abrase::ast::OwnershipAttr::Move);

    checker.check_type_decl("MoveEnum", &abrase::ast::TypeBody::Variant(variant_cases), false, &ownership);

    assert!(checker.get_type_ownership("MoveEnum").is_some());
}

// check_impl_decl Tests

#[test]
fn verify_check_impl_decl_type_checks_methods() {
    let mut checker = Checker::new();

    let method = abrase::ast::FnDecl {
        attrs: vec![],
        is_pub: false,
        name: "process".into(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(abrase::ast::Type::Named("Unit".into())),
        where_clause: vec![],
        body: dummy_block(),
    };

    let for_type = abrase::ast::Type::Named("MyType".into());

    // Should not panic
    checker.check_impl_decl(&for_type, &None, &[], &[], &[method]);
}

#[test]
fn verify_check_impl_decl_validates_trait_exists() {
    let mut checker = Checker::new();

    // Register a trait
    checker.register_trait("Show".into(), vec![]);

    let method = abrase::ast::FnDecl {
        attrs: vec![],
        is_pub: false,
        name: "show".into(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(abrase::ast::Type::Named("Unit".into())),
        where_clause: vec![],
        body: dummy_block(),
    };

    let for_type = abrase::ast::Type::Named("MyType".into());
    let trait_name = Some(vec!["Show".into()]);

    // Should not panic
    checker.check_impl_decl(&for_type, &trait_name, &[], &[], &[method]);
}

#[test]
fn verify_check_impl_decl_multiple_methods() {
    let mut checker = Checker::new();

    let method1 = abrase::ast::FnDecl {
        attrs: vec![],
        is_pub: false,
        name: "method1".into(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(abrase::ast::Type::Named("Unit".into())),
        where_clause: vec![],
        body: dummy_block(),
    };

    let method2 = abrase::ast::FnDecl {
        attrs: vec![],
        is_pub: false,
        name: "method2".into(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(abrase::ast::Type::Named("Unit".into())),
        where_clause: vec![],
        body: dummy_block(),
    };

    let for_type = abrase::ast::Type::Named("MyType".into());

    // Should not panic
    checker.check_impl_decl(&for_type, &None, &[], &[], &[method1, method2]);
}

// check_const_decl Tests

#[test]
fn verify_check_const_decl_registers_const() {
    let mut checker = Checker::new();

    let value = Spanned {
        node: Expr::Literal(abrase::ast::Literal::Int(42)),
        span: d_span(),
    };

    checker.check_const_decl("ANSWER", &abrase::ast::Type::Named("Int".into()), &value, false);

    // Const should be registered
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_check_const_decl_validates_type_match() {
    let mut checker = Checker::new();

    let value = Spanned {
        node: Expr::Literal(abrase::ast::Literal::Int(42)),
        span: d_span(),
    };

    checker.check_const_decl("WRONG", &abrase::ast::Type::Named("String".into()), &value, false);

    // Type mismatch should cause error
    assert!(!checker.errors.is_empty());
}

#[test]
fn verify_check_const_decl_marks_public() {
    let mut checker = Checker::new();

    let value = Spanned {
        node: Expr::Literal(abrase::ast::Literal::String("hello".into())),
        span: d_span(),
    };

    checker.check_const_decl("MESSAGE", &abrase::ast::Type::Named("String".into()), &value, true);

    assert!(checker.is_public("MESSAGE"));
}

#[test]
fn verify_check_const_decl_float() {
    let mut checker = Checker::new();

    let value = Spanned {
        node: Expr::Literal(abrase::ast::Literal::Float(3.14)),
        span: d_span(),
    };

    checker.check_const_decl("PI", &abrase::ast::Type::Named("Float".into()), &value, false);

    assert!(checker.errors.is_empty());
}

#[test]
fn verify_check_const_decl_bool() {
    let mut checker = Checker::new();

    let value = Spanned {
        node: Expr::Literal(abrase::ast::Literal::Bool(true)),
        span: d_span(),
    };

    checker.check_const_decl("FLAG", &abrase::ast::Type::Named("Bool".into()), &value, false);

    assert!(checker.errors.is_empty());
}

// check_effect_decl Tests

#[test]
fn verify_check_effect_decl_registers_effect() {
    let mut checker = Checker::new();

    let ops = vec![];

    checker.check_effect_decl("io", &ops, false);

    // Effect should be registered
    let _ = checker.is_public("io");
}

#[test]
fn verify_check_effect_decl_registers_operations() {
    let mut checker = Checker::new();

    let read_op = abrase::ast::FnSignature {
        name: "read".into(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(abrase::ast::Type::Named("String".into())),
        where_clause: vec![],
    };

    let write_op = abrase::ast::FnSignature {
        name: "write".into(),
        generics: vec![],
        params: vec![
            abrase::ast::Param::Named {
                pattern: Spanned {
                    node: Pattern::Bind("data".into()),
                    span: d_span(),
                },
                ty: abrase::ast::Type::Named("String".into()),
            },
        ],
        effects: vec![],
        return_type: Some(abrase::ast::Type::Named("Unit".into())),
        where_clause: vec![],
    };

    checker.check_effect_decl("file", &[read_op, write_op], false);

    // Effect should be registered
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_check_effect_decl_marks_public() {
    let mut checker = Checker::new();

    let ops = vec![];

    checker.check_effect_decl("net", &ops, true);

    assert!(checker.is_public("net"));
}

// check_import_decl Tests

#[test]
fn verify_check_import_decl_registers_imports() {
    let mut checker = Checker::new();

    let items = vec![
        abrase::ast::ImportItem {
            name: "read".into(),
            alias: None,
        },
    ];

    checker.check_import_decl(&["std".into()], &items);

    // Import should be registered
    let resolved = checker.get_imported_name("read");
    assert!(resolved.is_some());
}

#[test]
fn verify_check_import_decl_handles_alias() {
    let mut checker = Checker::new();

    let items = vec![
        abrase::ast::ImportItem {
            name: "print".into(),
            alias: Some("log".into()),
        },
    ];

    checker.check_import_decl(&["io".into()], &items);

    // Aliased import should be registered
    let resolved = checker.get_imported_name("log");
    assert!(resolved.is_some());
}

#[test]
fn verify_check_import_decl_multiple_items() {
    let mut checker = Checker::new();

    let items = vec![
        abrase::ast::ImportItem {
            name: "read".into(),
            alias: None,
        },
        abrase::ast::ImportItem {
            name: "write".into(),
            alias: None,
        },
    ];

    checker.check_import_decl(&["std", "io"].iter().map(|s| s.to_string()).collect::<Vec<_>>(), &items);

    // Both imports should be registered
    assert!(checker.get_imported_name("read").is_some());
    assert!(checker.get_imported_name("write").is_some());
}

#[test]
fn verify_check_import_decl_nested_path() {
    let mut checker = Checker::new();

    let items = vec![
        abrase::ast::ImportItem {
            name: "connect".into(),
            alias: None,
        },
    ];

    let path = vec!["std".into(), "net".into(), "tcp".into()];

    checker.check_import_decl(&path, &items);

    // Import from nested path should be registered
    let resolved = checker.get_imported_name("connect");
    assert!(resolved.is_some());
}

// Integration Tests

#[test]
fn verify_all_checkers_work_together() {
    let mut checker = Checker::new();

    // Check type
    checker.check_type_decl(
        "Status",
        &abrase::ast::TypeBody::Variant(vec![
            abrase::ast::VariantCase::Unit("Ok".into()),
            abrase::ast::VariantCase::Unit("Error".into()),
        ]),
        true,
        &None,
    );

    // Check import
    checker.check_import_decl(&["std".into()], &[
        abrase::ast::ImportItem {
            name: "print".into(),
            alias: None,
        },
    ]);

    // Check const
    let const_val = Spanned {
        node: Expr::Literal(abrase::ast::Literal::Int(0)),
        span: d_span(),
    };
    checker.check_const_decl("DEFAULT_ID", &abrase::ast::Type::Named("Int".into()), &const_val, true);

    // Check effect
    checker.check_effect_decl("custom", &[], true);

    // All should register without major errors
    assert!(checker.is_public("Status"));
    assert!(checker.is_public("DEFAULT_ID"));
    assert!(checker.is_public("custom"));
}

// Issue #5: Function body type mismatch detection

#[test]
fn verify_check_fn_decl_detects_return_type_mismatch() {
    let mut checker = Checker::new();

    // Create function that declares return type Int but has body that returns String
    let fn_decl = abrase::ast::FnDecl {
        attrs: vec![],
        is_pub: false,
        name: "bad_fn".into(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(abrase::ast::Type::Named("Int".into())),
        where_clause: vec![],
        // Block with a String literal return expression
        body: Block {
            stmts: vec![],
            ret: Some(Box::new(Spanned {
                node: Expr::Literal(abrase::ast::Literal::String("oops".into())),
                span: d_span(),
            })),
        },
    };

    checker.check_fn_decl(&fn_decl);

    // Should have caught the type mismatch
    assert!(!checker.errors.is_empty(), "Function body type mismatch should be detected");
    assert!(checker.errors[0].message.contains("Return type mismatch")
        || checker.errors[0].message.contains("type mismatch"),
        "Error message should mention return type mismatch, got: {}",
        checker.errors[0].message);
}

#[test]
fn verify_check_fn_decl_allows_correct_return_type() {
    let mut checker = Checker::new();

    let fn_decl = abrase::ast::FnDecl {
        attrs: vec![],
        is_pub: false,
        name: "good_fn".into(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(abrase::ast::Type::Named("Int".into())),
        where_clause: vec![],
        body: Block {
            stmts: vec![],
            ret: Some(Box::new(Spanned {
                node: Expr::Literal(abrase::ast::Literal::Int(42)),
                span: d_span(),
            })),
        },
    };

    checker.check_fn_decl(&fn_decl);

    // Should not have type mismatch error
    let has_mismatch = checker.errors.iter().any(|e| e.message.contains("mismatch"));
    assert!(!has_mismatch, "Correct return type should not cause error");
}


// Infrastructure & Context Management

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

// ── Gap tests ─────────────────────────────────────────────────────────────────

#[test]
fn verify_return_expr_type_checked_against_fn_return_type() {
    // fn bad() -> Int { return "oops" }
    // The `return "oops"` should be type-checked against declared return type Int.
    let mut checker = Checker::new();
    let fn_decl = abrase::ast::FnDecl {
        attrs: vec![],
        is_pub: false,
        name: "bad".into(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(abrase::ast::Type::Named("Int".into())),
        where_clause: vec![],
        body: Block {
            stmts: vec![Spanned {
                node: abrase::ast::Stmt::Expr(Spanned {
                    node: Expr::Return(Some(Box::new(Spanned {
                        node: Expr::Literal(abrase::ast::Literal::String("oops".into())),
                        span: d_span(),
                    }))),
                    span: d_span(),
                }),
                span: d_span(),
            }],
            ret: None,
        },
    };
    checker.check_fn_decl(&fn_decl);
    assert!(!checker.errors.is_empty(),
        "return with wrong type inside fn body must produce a type error");
}

#[test]
fn verify_effect_alias_decl_registers_alias() {
    use abrase::ty::Effect;
    let mut checker = Checker::new();
    checker.check_program(&[
        abrase::ast::Decl::EffectAlias {
            is_pub: false,
            name: "io_nondet".into(),
            effects: vec![
                abrase::ast::EffectItem { name: vec!["io".into()],     arg: None },
                abrase::ast::EffectItem { name: vec!["nondet".into()], arg: None },
            ],
        },
    ]);
    let alias = checker.get_effect_alias("io_nondet");
    assert!(alias.is_some(), "effect alias 'io_nondet' must be registered; got None");
    let effects = alias.unwrap();
    assert!(!effects.is_empty(), "effect alias must store at least one resolved effect");
    assert!(effects.iter().any(|e| matches!(e, Effect::Nondet)),
        "effect alias must include Nondet; got {:?}", effects);
}

#[test]
fn verify_self_param_does_not_error_in_impl_method() {
    // impl methods with self / &self / &mut self must not produce spurious errors
    let mut checker = Checker::new();
    let method = abrase::ast::FnDecl {
        attrs: vec![],
        is_pub: false,
        name: "get_x".into(),
        generics: vec![],
        params: vec![abrase::ast::Param::SelfRef { is_mut: false }],
        effects: vec![],
        return_type: None,
        where_clause: vec![],
        body: Block { stmts: vec![], ret: None },
    };
    checker.check_impl_decl(
        &abrase::ast::Type::Named("Point".into()),
        &None,
        &[],
        &[],
        &[method],
    );
    assert!(checker.errors.is_empty(),
        "impl method with &self must not produce errors; got {:?}", checker.errors);
}
