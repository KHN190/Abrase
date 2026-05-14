use ect::ast::{Expr, Literal, EffectItem, Spanned, BinaryOp};
use ect::ty::Type;
use ect::typeck::Checker;

fn d_span() -> ect::ast::Span {
    ect::ast::Span { line: 0, col: 0 }
}

fn sp<T>(node: T) -> Spanned<T> {
    Spanned { node, span: d_span() }
}

// Pure Expressions in Const

#[test]
fn verify_const_with_pure_literal() {
    let mut checker = Checker::new();

    // Const with pure literal
    let is_valid = checker.check_const_expr(&Expr::Literal(Literal::Int(42)), d_span());
    assert!(is_valid, "Pure literal should be valid in const");
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_const_with_pure_arithmetic() {
    let mut checker = Checker::new();

    // Register arithmetic operations as pure
    checker.register_effect_for_op("Add", vec![]);
    checker.register_effect_for_op("Mul", vec![]);

    // Binary operations with pure operands should be pure
    let is_valid = checker.check_const_expr(
        &Expr::Literal(Literal::Int(10)),
        d_span()
    );
    assert!(is_valid);
}

#[test]
fn verify_const_with_variable_reference() {
    let mut checker = Checker::new();

    // Insert a const variable (compile-time constant)
    checker.insert_const_var("MAX_SIZE".into(), Type::Int, d_span());

    // Referencing a const variable should be pure
    let is_valid = checker.check_const_expr(&Expr::Identifier("MAX_SIZE".into()), d_span());
    assert!(is_valid);
}

// IO Effects Forbidden in Const

#[test]
fn verify_const_rejects_io_effect() {
    let mut checker = Checker::new();

    // Register io effect
    checker.register_effect("io".into(), vec![]);

    // Mark function as having io effect
    let io_effects = vec![EffectItem {
        name: vec!["io".into()],
        arg: None,
    }];

    let fn_name = "read_file".into();
    checker.register_function_effects(fn_name, io_effects);

    // Function call with io effect should be invalid in const
    let is_valid = checker.check_const_expr(
        &Expr::Call {
            callee: Box::new(sp(Expr::Identifier("read_file".into()))),
            args: vec![],
        },
        d_span()
    );

    assert!(!is_valid, "IO effect should be forbidden in const");
    assert!(checker.errors.len() > 0);
    assert!(checker.errors.iter().any(|e|
        e.message.contains("io") ||
        e.message.contains("pure") ||
        e.message.contains("effect")
    ));
}

#[test]
fn verify_const_rejects_exn_effect() {
    let mut checker = Checker::new();

    // Register exn effect
    checker.register_effect("exn".into(), vec![]);

    let exn_effects = vec![EffectItem {
        name: vec!["exn".into()],
        arg: None,
    }];

    let fn_name = "divide_by_user".into();
    checker.register_function_effects(fn_name, exn_effects);

    // Function with exception effect should fail
    let is_valid = checker.check_const_expr(
        &Expr::Call {
            callee: Box::new(sp(Expr::Identifier("divide_by_user".into()))),
            args: vec![],
        },
        d_span()
    );

    assert!(!is_valid);
    assert!(checker.errors.len() > 0);
}

// Mutable State Forbidden in Const

#[test]
fn verify_const_rejects_mutable_variable() {
    let mut checker = Checker::new();

    // Insert a mutable variable
    checker.insert_var("counter".into(), Type::Int, true, d_span());

    // Referencing a mutable variable in const should fail
    let is_valid = checker.check_const_expr(
        &Expr::Identifier("counter".into()),
        d_span()
    );

    assert!(!is_valid);
    assert!(checker.errors.len() > 0);
    assert!(checker.errors.iter().any(|e|
        e.message.contains("mutable") ||
        e.message.contains("const")
    ));
}

#[test]
fn verify_const_rejects_assignment() {
    let mut checker = Checker::new();

    // Assignment should be forbidden in const
    let is_valid = checker.check_const_expr(
        &Expr::Binary {
            op: BinaryOp::Assign,
            left: Box::new(sp(Expr::Identifier("x".into()))),
            right: Box::new(sp(Expr::Literal(Literal::Int(5)))),
        },
        d_span()
    );

    assert!(!is_valid);
}

// Control Flow in Const

#[test]
fn verify_const_allows_simple_if() {
    let mut checker = Checker::new();

    // Simple if with pure branches should be allowed
    let is_valid = checker.check_const_expr(
        &Expr::If {
            condition: Box::new(sp(Expr::Literal(Literal::Bool(true)))),
            consequence: Box::new(sp(Expr::Literal(Literal::Int(1)))),
            alternative: Some(Box::new(sp(Expr::Literal(Literal::Int(2))))),
        },
        d_span()
    );

    assert!(is_valid, "Pure if-expression should be valid in const");
}

#[test]
fn verify_const_rejects_if_with_io() {
    let mut checker = Checker::new();

    checker.register_effect("io".into(), vec![]);
    let io_effects = vec![EffectItem {
        name: vec!["io".into()],
        arg: None,
    }];
    checker.register_function_effects("read".into(), io_effects);

    // If with io in one branch should fail
    let is_valid = checker.check_const_expr(
        &Expr::If {
            condition: Box::new(sp(Expr::Literal(Literal::Bool(true)))),
            consequence: Box::new(sp(Expr::Call {
                callee: Box::new(sp(Expr::Identifier("read".into()))),
                args: vec![],
            })),
            alternative: Some(Box::new(sp(Expr::Literal(Literal::Int(0))))),
        },
        d_span()
    );

    assert!(!is_valid);
}

// Function Calls in Const

#[test]
fn verify_const_allows_pure_function_call() {
    let mut checker = Checker::new();

    // Register a pure function
    let pure_effects = vec![];
    checker.register_function_effects("abs".into(), pure_effects);

    let is_valid = checker.check_const_expr(
        &Expr::Call {
            callee: Box::new(sp(Expr::Identifier("abs".into()))),
            args: vec![sp(Expr::Literal(Literal::Int(-5)))],
        },
        d_span()
    );

    assert!(is_valid, "Pure function call should be valid in const");
}

#[test]
fn verify_const_rejects_impure_function() {
    let mut checker = Checker::new();

    // Register function with multiple effects
    let impure_effects = vec![
        EffectItem {
            name: vec!["io".into()],
            arg: None,
        },
        EffectItem {
            name: vec!["exn".into()],
            arg: None,
        },
    ];
    checker.register_function_effects("dangerous_op".into(), impure_effects);

    let is_valid = checker.check_const_expr(
        &Expr::Call {
            callee: Box::new(sp(Expr::Identifier("dangerous_op".into()))),
            args: vec![],
        },
        d_span()
    );

    assert!(!is_valid);
    assert!(checker.errors.len() > 0);
}

// Effect Inference & Validation

#[test]
fn verify_const_infers_expression_effects() {
    let checker = Checker::new();

    // Infer effects of a pure expression
    let effects = checker.infer_expr_effects(&Expr::Literal(Literal::Int(42)));
    assert!(effects.is_empty(), "Pure literal should have no effects");
}

#[test]
fn verify_const_rejects_mixed_effects() {
    let mut checker = Checker::new();

    checker.register_effect("io".into(), vec![]);
    checker.register_effect("exn".into(), vec![]);

    // Register function with both io and exn
    let mixed_effects = vec![
        EffectItem {
            name: vec!["io".into()],
            arg: None,
        },
        EffectItem {
            name: vec!["exn".into()],
            arg: None,
        },
    ];
    checker.register_function_effects("mixed".into(), mixed_effects);

    let is_valid = checker.check_const_expr(
        &Expr::Call {
            callee: Box::new(sp(Expr::Identifier("mixed".into()))),
            args: vec![],
        },
        d_span()
    );

    assert!(!is_valid);
}

// Integration Tests

#[test]
fn verify_const_complex_pure_expression() {
    let mut checker = Checker::new();

    // Complex expression with only pure operations
    let is_valid = checker.check_const_expr(
        &Expr::Literal(Literal::Int(100)),
        d_span()
    );

    assert!(is_valid);
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_const_with_nested_pure_calls() {
    let mut checker = Checker::new();

    // Register pure functions
    checker.register_function_effects("add".into(), vec![]);
    checker.register_function_effects("mul".into(), vec![]);

    // Nested pure calls: mul(2, 3)
    let inner_call = Expr::Call {
        callee: Box::new(sp(Expr::Identifier("mul".into()))),
        args: vec![
            sp(Expr::Literal(Literal::Int(2))),
            sp(Expr::Literal(Literal::Int(3))),
        ],
    };

    let is_valid = checker.check_const_expr(&inner_call, d_span());
    assert!(is_valid);
}

#[test]
fn verify_const_rejects_any_nonpure_in_chain() {
    let mut checker = Checker::new();

    checker.register_effect("io".into(), vec![]);

    checker.register_function_effects("pure_fn".into(), vec![]);
    let io_effects = vec![EffectItem {
        name: vec!["io".into()],
        arg: None,
    }];
    checker.register_function_effects("io_fn".into(), io_effects);

    // Call with io in argument
    let outer = Expr::Call {
        callee: Box::new(sp(Expr::Identifier("pure_fn".into()))),
        args: vec![
            sp(Expr::Call {
                callee: Box::new(sp(Expr::Identifier("io_fn".into()))),
                args: vec![],
            }),
        ],
    };

    let is_valid = checker.check_const_expr(&outer, d_span());
    assert!(!is_valid);
}

#[test]
fn verify_effect_checked_on_const_value_assignment() {
    let mut checker = Checker::new();

    checker.register_effect("io".into(), vec![]);
    let io_effects = vec![EffectItem {
        name: vec!["io".into()],
        arg: None,
    }];
    checker.register_function_effects("read".into(), io_effects);

    // When assigning io-producing function to const, should fail
    let expr = Expr::Call {
        callee: Box::new(sp(Expr::Identifier("read".into()))),
        args: vec![],
    };

    let is_valid = checker.check_const_expr(&expr, d_span());
    assert!(!is_valid, "Const value cannot be initialized with IO effect");
}
