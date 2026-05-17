use abrase::ast::{Expr, Literal, Pattern, Span, Spanned, HandleArm, HandleArmKind};
use abrase::typeck::Checker;

fn sp<T>(node: T) -> Spanned<T> {
    Spanned { node, span: Span { line: 0, col: 0 } }
}

fn lit(n: i64) -> Spanned<Expr> { sp(Expr::Literal(Literal::Int(n))) }

fn check(expr: &Spanned<Expr>) -> Vec<String> {
    let mut checker = Checker::new();
    checker.infer_expr(expr);
    checker.errors.into_iter().map(|e| e.message).collect()
}

#[test]
fn single_effect_arm_accepted() {
    // `handle 1 { logger.log msg => 0 }`
    let expr = sp(Expr::Handle {
        expr: Box::new(lit(1)),
        arms: vec![
            HandleArm {
                kind: HandleArmKind::Effect(vec!["logger".into(), "log".into()]),
                pattern: Some(sp(Pattern::Bind("msg".into()))),
                body: sp(Expr::Literal(Literal::Int(0))),
            },
        ],
    });

    let errors = check(&expr);
    assert!(
        !errors.iter().any(|e| e.contains("single effect")),
        "single effect arm should not error, got: {:?}", errors,
    );
}

#[test]
fn multiple_arms_same_effect_accepted() {
    // `handle 1 { logger.log msg => 0, logger.warn msg => 1 }`
    let expr = sp(Expr::Handle {
        expr: Box::new(lit(1)),
        arms: vec![
            HandleArm {
                kind: HandleArmKind::Effect(vec!["logger".into(), "log".into()]),
                pattern: Some(sp(Pattern::Bind("msg".into()))),
                body: sp(Expr::Literal(Literal::Int(0))),
            },
            HandleArm {
                kind: HandleArmKind::Effect(vec!["logger".into(), "warn".into()]),
                pattern: Some(sp(Pattern::Bind("msg".into()))),
                body: sp(Expr::Literal(Literal::Int(1))),
            },
        ],
    });

    let errors = check(&expr);
    assert!(
        !errors.iter().any(|e| e.contains("single effect")),
        "multiple arms of same effect should not error, got: {:?}", errors,
    );
}

#[test]
fn mixed_effects_rejected() {
    // `handle 1 { logger.log msg => 0, file.read msg => 1 }`
    let expr = sp(Expr::Handle {
        expr: Box::new(lit(1)),
        arms: vec![
            HandleArm {
                kind: HandleArmKind::Effect(vec!["logger".into(), "log".into()]),
                pattern: Some(sp(Pattern::Bind("msg".into()))),
                body: sp(Expr::Literal(Literal::Int(0))),
            },
            HandleArm {
                kind: HandleArmKind::Effect(vec!["file".into(), "read".into()]),
                pattern: Some(sp(Pattern::Bind("msg".into()))),
                body: sp(Expr::Literal(Literal::Int(1))),
            },
        ],
    });

    let errors = check(&expr);
    assert!(
        errors.iter().any(|e| e.contains("single effect") && e.contains("split into separate")),
        "mixed effects should error, got: {:?}", errors,
    );
}

#[test]
fn three_effects_first_two_same_still_rejected() {
    // `handle 1 { logger.log m => 0, logger.warn m => 1, file.read m => 2 }`
    let expr = sp(Expr::Handle {
        expr: Box::new(lit(1)),
        arms: vec![
            HandleArm {
                kind: HandleArmKind::Effect(vec!["logger".into(), "log".into()]),
                pattern: Some(sp(Pattern::Bind("m".into()))),
                body: sp(Expr::Literal(Literal::Int(0))),
            },
            HandleArm {
                kind: HandleArmKind::Effect(vec!["logger".into(), "warn".into()]),
                pattern: Some(sp(Pattern::Bind("m".into()))),
                body: sp(Expr::Literal(Literal::Int(1))),
            },
            HandleArm {
                kind: HandleArmKind::Effect(vec!["file".into(), "read".into()]),
                pattern: Some(sp(Pattern::Bind("m".into()))),
                body: sp(Expr::Literal(Literal::Int(2))),
            },
        ],
    });

    let errors = check(&expr);
    assert!(
        errors.iter().any(|e| e.contains("single effect")),
        "three different effects should error, got: {:?}", errors,
    );
}

#[test]
fn return_and_effect_arms_allowed() {
    // `handle 1 { logger.log msg => 0, return => 1 }`
    // Return arm can coexist with any single effect
    let expr = sp(Expr::Handle {
        expr: Box::new(lit(1)),
        arms: vec![
            HandleArm {
                kind: HandleArmKind::Effect(vec!["logger".into(), "log".into()]),
                pattern: Some(sp(Pattern::Bind("msg".into()))),
                body: sp(Expr::Literal(Literal::Int(0))),
            },
            HandleArm {
                kind: HandleArmKind::Return,
                pattern: None,
                body: sp(Expr::Literal(Literal::Int(1))),
            },
        ],
    });

    let errors = check(&expr);
    assert!(
        !errors.iter().any(|e| e.contains("single effect")),
        "return + single effect should not error, got: {:?}", errors,
    );
}

#[test]
fn nested_effect_paths_checked() {
    // `handle 1 { a.b.c msg => 0, a.b.d msg => 1 }` — both are a.b, should accept
    let expr = sp(Expr::Handle {
        expr: Box::new(lit(1)),
        arms: vec![
            HandleArm {
                kind: HandleArmKind::Effect(vec!["a".into(), "b".into(), "c".into()]),
                pattern: Some(sp(Pattern::Bind("msg".into()))),
                body: sp(Expr::Literal(Literal::Int(0))),
            },
            HandleArm {
                kind: HandleArmKind::Effect(vec!["a".into(), "b".into(), "d".into()]),
                pattern: Some(sp(Pattern::Bind("msg".into()))),
                body: sp(Expr::Literal(Literal::Int(1))),
            },
        ],
    });

    let errors = check(&expr);
    assert!(
        !errors.iter().any(|e| e.contains("single effect")),
        "same base effect should not error, got: {:?}", errors,
    );
}

#[test]
fn different_nested_effects_rejected() {
    // `handle 1 { a.b.c msg => 0, a.d.c msg => 1 }` — a.b vs a.d, should reject
    let expr = sp(Expr::Handle {
        expr: Box::new(lit(1)),
        arms: vec![
            HandleArm {
                kind: HandleArmKind::Effect(vec!["a".into(), "b".into(), "c".into()]),
                pattern: Some(sp(Pattern::Bind("msg".into()))),
                body: sp(Expr::Literal(Literal::Int(0))),
            },
            HandleArm {
                kind: HandleArmKind::Effect(vec!["a".into(), "d".into(), "c".into()]),
                pattern: Some(sp(Pattern::Bind("msg".into()))),
                body: sp(Expr::Literal(Literal::Int(1))),
            },
        ],
    });

    let errors = check(&expr);
    assert!(
        errors.iter().any(|e| e.contains("single effect")),
        "different base effects should error, got: {:?}", errors,
    );
}

#[test]
fn single_component_effect_ignored() {
    // Effects with len < 2 (malformed) should not be part of validation
    let expr = sp(Expr::Handle {
        expr: Box::new(lit(1)),
        arms: vec![
            HandleArm {
                kind: HandleArmKind::Effect(vec!["single".into()]),
                pattern: Some(sp(Pattern::Bind("msg".into()))),
                body: sp(Expr::Literal(Literal::Int(0))),
            },
            HandleArm {
                kind: HandleArmKind::Return,
                pattern: None,
                body: sp(Expr::Literal(Literal::Int(1))),
            },
        ],
    });

    let errors = check(&expr);
    // Single-component effects are malformed but shouldn't trigger the "single effect" error
    // (they're filtered by the len < 2 check in the validator)
    let mixed_error_count = errors.iter().filter(|e| e.contains("single effect")).count();
    assert_eq!(mixed_error_count, 0, "single-component effects should be ignored, got: {:?}", errors);
}
