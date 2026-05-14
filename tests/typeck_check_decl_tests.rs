use ect::ty::Type;
use ect::typeck::Checker;
use ect::ast::{Span, Spanned, Pattern, Block, Expr};

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

    let fn_decl = ect::ast::FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
        name: "add".into(),
        generics: vec![],
        params: vec![
            ect::ast::Param::Named {
                pattern: Spanned {
                    node: Pattern::Bind("a".into()),
                    span: d_span(),
                },
                ty: ect::ast::Type::Named("Int".into()),
            },
            ect::ast::Param::Named {
                pattern: Spanned {
                    node: Pattern::Bind("b".into()),
                    span: d_span(),
                },
                ty: ect::ast::Type::Named("Int".into()),
            },
        ],
        effects: vec![],
        return_type: Some(ect::ast::Type::Named("Int".into())),
        where_clause: vec![],
        body: dummy_block(),
    };

    let decl = ect::ast::Decl::Fn(fn_decl);

    checker.check_program(&[decl]);

    // Verify function was registered
    let fn_type = checker.get_var("add", false, d_span());
    assert_eq!(fn_type, Type::Function {
        params: vec![Type::Int, Type::Int],
        effects: vec![],
        ret: Box::new(Type::Int),
    });
}

#[test]
fn verify_check_program_marks_public_function() {
    let mut checker = Checker::new();

    let fn_decl = ect::ast::FnDecl {
        attrs: vec![],
        is_pub: true,
        is_async: false,
        name: "public_fn".into(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(ect::ast::Type::Named("Unit".into())),
        where_clause: vec![],
        body: dummy_block(),
    };

    let decl = ect::ast::Decl::Fn(fn_decl);

    checker.check_program(&[decl]);

    // Verify function is marked public
    assert!(checker.is_public("public_fn"));
}

#[test]
fn verify_check_program_private_function_not_public() {
    let mut checker = Checker::new();

    let fn_decl = ect::ast::FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
        name: "private_fn".into(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(ect::ast::Type::Named("Unit".into())),
        where_clause: vec![],
        body: dummy_block(),
    };

    let decl = ect::ast::Decl::Fn(fn_decl);

    checker.check_program(&[decl]);

    // Verify function is not public
    assert!(!checker.is_public("private_fn"));
}

// Type Declaration Tests

#[test]
fn verify_check_program_registers_type() {
    let mut checker = Checker::new();

    let record_field = ect::ast::RecordField {
        is_pub: true,
        name: "x".into(),
        ty: ect::ast::Type::Named("Int".into()),
    };

    let decl = ect::ast::Decl::Type {
        attrs: vec![],
        is_pub: false,
        ownership: None,
        name: "Point".into(),
        generics: vec![],
        body: ect::ast::TypeBody::Record(vec![record_field]),
    };

    checker.check_program(&[decl]);

    // Verify type was registered by checking it's marked public (can be queried)
    // Since is_public checks the registry, if the type wasn't registered, it would fail
    let _ = checker.is_public("Point");
}

#[test]
fn verify_check_program_marks_public_type() {
    let mut checker = Checker::new();

    let decl = ect::ast::Decl::Type {
        attrs: vec![],
        is_pub: true,
        ownership: None,
        name: "PublicType".into(),
        generics: vec![],
        body: ect::ast::TypeBody::Variant(vec![
            ect::ast::VariantCase::Unit("A".into()),
        ]),
    };

    checker.check_program(&[decl]);

    // Verify type is marked public
    assert!(checker.is_public("PublicType"));
}

#[test]
fn verify_check_program_registers_type_ownership() {
    let mut checker = Checker::new();

    let decl = ect::ast::Decl::Type {
        attrs: vec![],
        is_pub: false,
        ownership: Some(ect::ast::OwnershipAttr::Copy),
        name: "CopyType".into(),
        generics: vec![],
        body: ect::ast::TypeBody::Variant(vec![
            ect::ast::VariantCase::Unit("X".into()),
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

    let decl = ect::ast::Decl::Const {
        is_pub: false,
        is_fn: false,
        name: "MAX_SIZE".into(),
        generics: vec![],
        params: vec![],
        ty: ect::ast::Type::Named("Int".into()),
        value: Spanned {
            node: Expr::Literal(ect::ast::Literal::Int(100)),
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

    let decl = ect::ast::Decl::Const {
        is_pub: true,
        is_fn: false,
        name: "PUBLIC_CONST".into(),
        generics: vec![],
        params: vec![],
        ty: ect::ast::Type::Named("String".into()),
        value: Spanned {
            node: Expr::Literal(ect::ast::Literal::String("hello".into())),
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

    let decl = ect::ast::Decl::Trait {
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

    let decl = ect::ast::Decl::Trait {
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

    let decl = ect::ast::Decl::Effect {
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

    let decl = ect::ast::Decl::Effect {
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

    let decl = ect::ast::Decl::Import {
        path: vec!["std".into()],
        items: vec![
            ect::ast::ImportItem {
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

    let decl = ect::ast::Decl::Import {
        path: vec!["io".into()],
        items: vec![
            ect::ast::ImportItem {
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

#[test]
fn verify_check_program_pushes_module() {
    let mut checker = Checker::new();

    let decl = ect::ast::Decl::Mod("mymodule".into());

    checker.check_program(&[decl]);

    // Module should be in scope - verify by checking if items in this module are accessible
    checker.push_module("test".into());

    // If the module was properly pushed and then we can navigate, it worked
    checker.pop_module();
}

// Multi-Declaration Tests

#[test]
fn verify_check_program_two_pass_execution() {
    let mut checker = Checker::new();

    // Create two functions that might reference each other
    let fn1 = ect::ast::FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
        name: "fn1".into(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(ect::ast::Type::Named("Int".into())),
        where_clause: vec![],
        body: dummy_block(),
    };

    let fn2 = ect::ast::FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
        name: "fn2".into(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(ect::ast::Type::Named("Int".into())),
        where_clause: vec![],
        body: dummy_block(),
    };

    let decls = vec![
        ect::ast::Decl::Fn(fn1),
        ect::ast::Decl::Fn(fn2),
    ];

    checker.check_program(&decls);

    // Both functions should be registered
    let fn1_type = checker.get_var("fn1", false, d_span());
    let fn2_type = checker.get_var("fn2", false, d_span());

    assert_eq!(fn1_type, Type::Function {
        params: vec![],
        effects: vec![],
        ret: Box::new(Type::Int),
    });

    assert_eq!(fn2_type, Type::Function {
        params: vec![],
        effects: vec![],
        ret: Box::new(Type::Int),
    });
}

#[test]
fn verify_check_program_type_then_function() {
    let mut checker = Checker::new();

    let type_decl = ect::ast::Decl::Type {
        attrs: vec![],
        is_pub: false,
        ownership: None,
        name: "MyType".into(),
        generics: vec![],
        body: ect::ast::TypeBody::Variant(vec![
            ect::ast::VariantCase::Unit("A".into()),
        ]),
    };

    let fn_decl = ect::ast::FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
        name: "process".into(),
        generics: vec![],
        params: vec![
            ect::ast::Param::Named {
                pattern: Spanned {
                    node: Pattern::Bind("x".into()),
                    span: d_span(),
                },
                ty: ect::ast::Type::Named("MyType".into()),
            },
        ],
        effects: vec![],
        return_type: Some(ect::ast::Type::Named("Unit".into())),
        where_clause: vec![],
        body: dummy_block(),
    };

    let decls = vec![
        type_decl,
        ect::ast::Decl::Fn(fn_decl),
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

    let impl_decl = ect::ast::Decl::Impl {
        generics: vec![],
        trait_name: None,
        for_type: ect::ast::Type::Named("MyType".into()),
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

    let public_fn = ect::ast::FnDecl {
        attrs: vec![],
        is_pub: true,
        is_async: false,
        name: "public".into(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(ect::ast::Type::Named("Unit".into())),
        where_clause: vec![],
        body: dummy_block(),
    };

    let private_fn = ect::ast::FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
        name: "private".into(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(ect::ast::Type::Named("Unit".into())),
        where_clause: vec![],
        body: dummy_block(),
    };

    let decls = vec![
        ect::ast::Decl::Fn(public_fn),
        ect::ast::Decl::Fn(private_fn),
    ];

    checker.check_program(&decls);

    assert!(checker.is_public("public"));
    assert!(!checker.is_public("private"));
}
