use ect::ast::{BinaryOp, EffectItem, Expr, Literal, Span, Spanned, self};
use ect::ty::Type;
use ect::typeck::Checker;

fn d_span() -> Span { Span::new(0, 0) }
fn sp<T>(node: T) -> Spanned<T> { Spanned { node, span: d_span() } }

// Effects System Tests

#[test]
fn verify_effect_registration() {
    let mut checker = Checker::new();
    checker.register_effect("io".into(), vec!["read".into(), "write".into()]);

    let effect = checker.get_effect("io");
    assert!(effect.is_some());
    assert_eq!(effect.unwrap(), vec!["read", "write"]);
}

#[test]
fn verify_effect_alias_registration() {
    let mut checker = Checker::new();
    use ect::ty::Effect;

    let effects = vec![Effect::Async, Effect::Alloc];
    checker.register_effect_alias("concurrent".into(), effects.clone());

    let alias = checker.get_effect_alias("concurrent");
    assert!(alias.is_some());
    assert_eq!(alias.unwrap().len(), 2);
}

#[test]
fn verify_push_and_pop_effect() {
    let mut checker = Checker::new();
    use ect::ty::Effect;

    checker.push_effect(Effect::Async);
    checker.push_effect(Effect::Alloc);

    let expected = vec![Effect::Async, Effect::Alloc];
    assert!(checker.effects_compatible(&expected, &expected));

    checker.pop_effect();
    let expected2 = vec![Effect::Async];
    assert!(checker.effects_compatible(&expected2, &expected2));
}

#[test]
fn verify_effects_equal_total() {
    let checker = Checker::new();
    use ect::ty::Effect;

    assert!(checker.effects_equal(&Effect::Total, &Effect::Total));
    assert!(!checker.effects_equal(&Effect::Total, &Effect::Async));
}

#[test]
fn verify_effects_equal_async() {
    let checker = Checker::new();
    use ect::ty::Effect;

    assert!(checker.effects_equal(&Effect::Async, &Effect::Async));
}

#[test]
fn verify_effects_equal_alloc() {
    let checker = Checker::new();
    use ect::ty::Effect;

    assert!(checker.effects_equal(&Effect::Alloc, &Effect::Alloc));
}

#[test]
fn verify_effects_equal_nondet() {
    let checker = Checker::new();
    use ect::ty::Effect;

    assert!(checker.effects_equal(&Effect::Nondet, &Effect::Nondet));
}

#[test]
fn verify_effects_equal_exn_same_type() {
    let checker = Checker::new();
    use ect::ty::Effect;

    let exn1 = Effect::Exn(Box::new(Type::String));
    let exn2 = Effect::Exn(Box::new(Type::String));

    assert!(checker.effects_equal(&exn1, &exn2));
}

#[test]
fn verify_effects_equal_exn_different_type() {
    let checker = Checker::new();
    use ect::ty::Effect;

    let exn1 = Effect::Exn(Box::new(Type::String));
    let exn2 = Effect::Exn(Box::new(Type::Int));

    assert!(!checker.effects_equal(&exn1, &exn2));
}

#[test]
fn verify_effects_compatible_empty() {
    let checker = Checker::new();

    assert!(checker.effects_compatible(&[], &[]));
    assert!(checker.effects_compatible(&[], &vec![ect::ty::Effect::Async]));
}

#[test]
fn verify_effects_compatible_single_effect() {
    let checker = Checker::new();
    use ect::ty::Effect;

    let expected = vec![Effect::Async];
    let actual = vec![Effect::Async];

    assert!(checker.effects_compatible(&expected, &actual));
}

#[test]
fn verify_effects_compatible_subset() {
    let checker = Checker::new();
    use ect::ty::Effect;

    let expected = vec![Effect::Async];
    let actual = vec![Effect::Async, Effect::Alloc];

    assert!(checker.effects_compatible(&expected, &actual));
}

#[test]
fn verify_effects_compatible_missing_effect() {
    let checker = Checker::new();
    use ect::ty::Effect;

    let expected = vec![Effect::Async, Effect::Alloc];
    let actual = vec![Effect::Async];

    assert!(!checker.effects_compatible(&expected, &actual));
}

#[test]
fn verify_convert_effect_io() {
    let checker = Checker::new();
    use ect::ty::Effect;

    let effect_item = ast::EffectItem {
        name: vec!["io".into()],
        arg: None,
    };

    let converted = checker.convert_effect(&effect_item);
    assert!(converted.is_some());
    assert!(matches!(converted.unwrap(), Effect::Alloc));
}

#[test]
fn verify_convert_effect_async() {
    let checker = Checker::new();
    use ect::ty::Effect;

    let effect_item = ast::EffectItem {
        name: vec!["async".into()],
        arg: None,
    };

    let converted = checker.convert_effect(&effect_item);
    assert!(converted.is_some());
    assert!(matches!(converted.unwrap(), Effect::Async));
}

#[test]
fn verify_convert_effect_exn_no_arg() {
    let checker = Checker::new();
    use ect::ty::Effect;

    let effect_item = ast::EffectItem {
        name: vec!["exn".into()],
        arg: None,
    };

    let converted = checker.convert_effect(&effect_item);
    assert!(converted.is_some());
    match converted.unwrap() {
        Effect::Exn(exc_ty) => {
            assert_eq!(*exc_ty, Type::Named("Exception".into()));
        },
        _ => panic!("Expected Exn effect"),
    }
}

#[test]
fn verify_convert_effect_exn_with_arg() {
    let checker = Checker::new();
    use ect::ty::Effect;

    let effect_item = ast::EffectItem {
        name: vec!["exn".into()],
        arg: Some(Box::new(ast::Type::Named("CustomError".into()))),
    };

    let converted = checker.convert_effect(&effect_item);
    assert!(converted.is_some());
    match converted.unwrap() {
        Effect::Exn(exc_ty) => {
            assert_eq!(*exc_ty, Type::Named("CustomError".into()));
        },
        _ => panic!("Expected Exn effect"),
    }
}

#[test]
fn verify_convert_effect_nondet() {
    let checker = Checker::new();
    use ect::ty::Effect;

    let effect_item = ast::EffectItem {
        name: vec!["nondet".into()],
        arg: None,
    };

    let converted = checker.convert_effect(&effect_item);
    assert!(converted.is_some());
    assert!(matches!(converted.unwrap(), Effect::Nondet));
}

#[test]
fn verify_function_type_with_effects() {
    let _checker = Checker::new();
    use ect::ty::Effect;

    let fn_type = Type::Function {
        params: vec![Type::Int],
        effects: vec![Effect::Async],
        ret: Box::new(Type::Bool),
    };

    match fn_type {
        Type::Function { effects, .. } => {
            assert_eq!(effects.len(), 1);
            assert!(matches!(&effects[0], Effect::Async));
        },
        _ => panic!("Expected function type"),
    }
}

#[test]
fn verify_function_type_multiple_effects() {
    let _checker = Checker::new();
    use ect::ty::Effect;

    let fn_type = Type::Function {
        params: vec![Type::Int],
        effects: vec![Effect::Async, Effect::Alloc],
        ret: Box::new(Type::Bool),
    };

    match fn_type {
        Type::Function { effects, .. } => {
            assert_eq!(effects.len(), 2);
        },
        _ => panic!("Expected function type"),
    }
}

#[test]
fn verify_effect_total() {
    let _checker = Checker::new();
    use ect::ty::Effect;

    assert!(matches!(Effect::Total, Effect::Total));
}

#[test]
fn verify_convert_effect_alloc() {
    let checker = Checker::new();
    use ect::ty::Effect;

    let effect_item = ast::EffectItem {
        name: vec!["alloc".into()],
        arg: None,
    };

    let converted = checker.convert_effect(&effect_item);
    assert!(converted.is_some());
    assert!(matches!(converted.unwrap(), Effect::Alloc));
}

#[test]
fn verify_effect_compatibility_with_multiple_effects() {
    let checker = Checker::new();
    use ect::ty::Effect;

    let expected = vec![Effect::Async, Effect::Alloc];
    let actual = vec![Effect::Async, Effect::Alloc, Effect::Nondet];

    assert!(checker.effects_compatible(&expected, &actual));
}

#[test]
fn verify_effect_compatibility_order_independent() {
    let checker = Checker::new();
    use ect::ty::Effect;

    let expected = vec![Effect::Alloc, Effect::Async];
    let actual = vec![Effect::Async, Effect::Alloc];

    assert!(checker.effects_compatible(&expected, &actual));
}

// Effect Unification & Inference Tests

#[test]
fn verify_set_fn_declared_effects() {
    let mut checker = Checker::new();
    use ect::ty::Effect;

    let effects = vec![Effect::Async, Effect::Alloc];
    checker.set_fn_declared_effects(effects.clone());

    let declared = checker.get_fn_declared_effects();
    assert_eq!(declared.len(), 2);
}

#[test]
fn verify_add_required_effect() {
    let mut checker = Checker::new();
    use ect::ty::Effect;

    checker.add_required_effect(Effect::Async);
    checker.add_required_effect(Effect::Alloc);

    let required = checker.get_fn_required_effects();
    assert_eq!(required.len(), 2);
}

#[test]
fn verify_add_required_effect_no_duplicates() {
    let mut checker = Checker::new();
    use ect::ty::Effect;

    checker.add_required_effect(Effect::Async);
    checker.add_required_effect(Effect::Async);

    let required = checker.get_fn_required_effects();
    assert_eq!(required.len(), 1);
}

#[test]
fn verify_check_effect_compatibility_satisfied() {
    let mut checker = Checker::new();
    use ect::ty::Effect;

    checker.add_required_effect(Effect::Async);
    let provided = vec![Effect::Async, Effect::Alloc];

    let result = checker.check_effect_compatibility(&provided, d_span());
    assert!(result);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_check_effect_compatibility_unsatisfied() {
    let mut checker = Checker::new();
    use ect::ty::Effect;

    checker.add_required_effect(Effect::Async);
    let provided = vec![Effect::Alloc];

    let result = checker.check_effect_compatibility(&provided, d_span());
    assert!(!result);
    assert!(!checker.errors.is_empty());
}

#[test]
fn verify_unify_effects_no_duplicates() {
    let checker = Checker::new();
    use ect::ty::Effect;

    let effects1 = vec![Effect::Async, Effect::Alloc];
    let effects2 = vec![Effect::Async, Effect::Nondet];

    let unified = checker.unify_effects(&effects1, &effects2);
    assert_eq!(unified.len(), 3);
}

#[test]
fn verify_unify_effects_empty_left() {
    let checker = Checker::new();
    use ect::ty::Effect;

    let effects1 = vec![];
    let effects2 = vec![Effect::Async];

    let unified = checker.unify_effects(&effects1, &effects2);
    assert_eq!(unified.len(), 1);
}

#[test]
fn verify_unify_effects_empty_right() {
    let checker = Checker::new();
    use ect::ty::Effect;

    let effects1 = vec![Effect::Async];
    let effects2 = vec![];

    let unified = checker.unify_effects(&effects1, &effects2);
    assert_eq!(unified.len(), 1);
}

#[test]
fn verify_effects_subsume_all_provided() {
    let checker = Checker::new();
    use ect::ty::Effect;

    let required = vec![Effect::Async];
    let provided = vec![Effect::Async, Effect::Alloc];

    assert!(checker.effects_subsume(&required, &provided));
}

#[test]
fn verify_effects_subsume_missing() {
    let checker = Checker::new();
    use ect::ty::Effect;

    let required = vec![Effect::Async, Effect::Alloc];
    let provided = vec![Effect::Async];

    assert!(!checker.effects_subsume(&required, &provided));
}

#[test]
fn verify_effects_subsume_empty_required() {
    let checker = Checker::new();
    use ect::ty::Effect;

    let required = vec![];
    let provided = vec![Effect::Async];

    assert!(checker.effects_subsume(&required, &provided));
}

#[test]
fn verify_infer_closure_effects_with_declared() {
    let mut checker = Checker::new();
    use ect::ty::Effect;

    let declared = vec![Effect::Async];
    checker.set_fn_declared_effects(declared.clone());

    let body_effects = vec![Effect::Alloc];
    let inferred = checker.infer_closure_effects(&body_effects);

    assert_eq!(inferred, declared);
}

#[test]
fn verify_infer_closure_effects_without_declared() {
    let checker = Checker::new();
    use ect::ty::Effect;

    let body_effects = vec![Effect::Alloc, Effect::Async];
    let inferred = checker.infer_closure_effects(&body_effects);

    assert_eq!(inferred, body_effects);
}

#[test]
fn verify_convert_effect_items_single() {
    let checker = Checker::new();

    let items = vec![ast::EffectItem {
        name: vec!["async".into()],
        arg: None,
    }];

    let effects = checker.convert_effect_items(&items);
    assert_eq!(effects.len(), 1);
}

#[test]
fn verify_convert_effect_items_multiple() {
    let checker = Checker::new();

    let items = vec![
        ast::EffectItem {
            name: vec!["async".into()],
            arg: None,
        },
        ast::EffectItem {
            name: vec!["io".into()],
            arg: None,
        },
    ];

    let effects = checker.convert_effect_items(&items);
    assert_eq!(effects.len(), 2);
}

#[test]
fn verify_convert_effect_items_empty() {
    let checker = Checker::new();

    let items = vec![];
    let effects = checker.convert_effect_items(&items);

    assert!(effects.is_empty());
}

#[test]
fn verify_effect_inference_in_closure_type() {
    let mut checker = Checker::new();

    let closure_expr = sp(ast::Expr::Closure {
        is_move: false,
        params: vec![],
        effects: vec![],
        ret_ty: Some(ast::Type::Named("Int".into())),
        body: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(42)))),
    });

    let ty = checker.infer_expr(&closure_expr);
    match ty {
        Type::Function { ret, .. } => {
            assert!(matches!(*ret, Type::Int));
        },
        _ => panic!("Expected Function type"),
    }
}

#[test]
fn verify_fn_declared_effects_cleared_after_closure() {
    let mut checker = Checker::new();
    use ect::ty::Effect;

    let initial_effects = vec![Effect::Async];
    checker.set_fn_declared_effects(initial_effects);

    let closure_expr = sp(ast::Expr::Closure {
        is_move: false,
        params: vec![],
        effects: vec![],
        ret_ty: None,
        body: Box::new(sp(ast::Expr::Literal(ast::Literal::Unit))),
    });

    checker.infer_expr(&closure_expr);

    // After closure inference, declared effects should be empty
    assert!(checker.get_fn_declared_effects().is_empty());
}

#[test]
fn verify_required_effects_accumulate() {
    let mut checker = Checker::new();
    use ect::ty::Effect;

    checker.add_required_effect(Effect::Async);
    checker.add_required_effect(Effect::Alloc);
    checker.add_required_effect(Effect::Nondet);

    let required = checker.get_fn_required_effects();
    assert_eq!(required.len(), 3);
}

#[test]
fn verify_effect_compatibility_with_exn() {
    let checker = Checker::new();
    use ect::ty::Effect;

    let required = vec![Effect::Exn(Box::new(Type::String))];
    let provided = vec![Effect::Exn(Box::new(Type::String)), Effect::Async];

    assert!(checker.effects_subsume(&required, &provided));
}

#[test]
fn verify_effect_compatibility_exn_type_mismatch() {
    let checker = Checker::new();
    use ect::ty::Effect;

    let required = vec![Effect::Exn(Box::new(Type::String))];
    let provided = vec![Effect::Exn(Box::new(Type::Int))];

    assert!(!checker.effects_subsume(&required, &provided));
}

// Effect Shadowing, Propagation & Scope Semantics Tests

#[test]
fn verify_mark_effect_handled() {
    let mut checker = Checker::new();

    checker.mark_effect_handled("async".into());
    checker.mark_effect_handled("io".into());

    let handled = checker.get_handled_effects();
    assert_eq!(handled.len(), 2);
    assert!(handled.contains(&"async".to_string()));
    assert!(handled.contains(&"io".to_string()));
}

#[test]
fn verify_mark_effect_handled_no_duplicates() {
    let mut checker = Checker::new();

    checker.mark_effect_handled("async".into());
    checker.mark_effect_handled("async".into());

    let handled = checker.get_handled_effects();
    assert_eq!(handled.len(), 1);
}

#[test]
fn verify_compute_unhandled_effects_all_handled() {
    let mut checker = Checker::new();
    use ect::ty::Effect;

    checker.mark_effect_handled("async".into());
    let all_effects = vec![Effect::Async];

    checker.compute_unhandled_effects(&all_effects);
    assert!(checker.get_unhandled_effects().is_empty());
}

#[test]
fn verify_compute_unhandled_effects_partial_handled() {
    let mut checker = Checker::new();
    use ect::ty::Effect;

    checker.mark_effect_handled("async".into());
    let all_effects = vec![Effect::Async, Effect::Alloc];

    checker.compute_unhandled_effects(&all_effects);
    let unhandled = checker.get_unhandled_effects();
    assert_eq!(unhandled.len(), 1);
    assert!(matches!(&unhandled[0], Effect::Alloc));
}

#[test]
fn verify_compute_unhandled_effects_none_handled() {
    let mut checker = Checker::new();
    use ect::ty::Effect;

    let all_effects = vec![Effect::Async, Effect::Alloc];

    checker.compute_unhandled_effects(&all_effects);
    let unhandled = checker.get_unhandled_effects();
    assert_eq!(unhandled.len(), 2);
}

#[test]
fn verify_compute_unhandled_effects_exn_handling() {
    let mut checker = Checker::new();
    use ect::ty::Effect;

    checker.mark_effect_handled("exn".into());
    let all_effects = vec![Effect::Exn(Box::new(Type::String))];

    checker.compute_unhandled_effects(&all_effects);
    assert!(checker.get_unhandled_effects().is_empty());
}

#[test]
fn verify_validate_parameterized_exn_handler_with_bind() {
    let checker = Checker::new();

    let pattern = Some(sp(ast::Pattern::Bind("e".into())));
    let valid = checker.validate_parameterized_exn_handler(&Type::String, &pattern);
    assert!(valid);
}

#[test]
fn verify_validate_parameterized_exn_handler_with_unknown() {
    let checker = Checker::new();

    let pattern = Some(sp(ast::Pattern::Bind("e".into())));
    let valid = checker.validate_parameterized_exn_handler(&Type::Unknown, &pattern);
    assert!(!valid);
}

#[test]
fn verify_validate_parameterized_exn_handler_no_pattern() {
    let checker = Checker::new();

    let valid = checker.validate_parameterized_exn_handler(&Type::String, &None);
    assert!(valid);
}

#[test]
fn verify_validate_scope_with_context_valid() {
    let checker = Checker::new();

    let valid = checker.validate_scope_with_context(&Type::Int);
    assert!(valid);
}

#[test]
fn verify_validate_scope_with_context_unknown() {
    let checker = Checker::new();

    let valid = checker.validate_scope_with_context(&Type::Unknown);
    assert!(!valid);
}

#[test]
fn verify_clear_handle_context() {
    let mut checker = Checker::new();

    checker.mark_effect_handled("async".into());
    assert!(!checker.get_handled_effects().is_empty());

    checker.clear_handle_context();
    assert!(checker.get_handled_effects().is_empty());
    assert!(checker.get_unhandled_effects().is_empty());
}

#[test]
fn verify_effect_propagation_accumulates_in_required() {
    let mut checker = Checker::new();
    use ect::ty::Effect;

    // Add some required effects first
    checker.add_required_effect(Effect::Async);

    // Mark some effects as handled and compute unhandled
    checker.mark_effect_handled("async".into());
    let all_effects = vec![Effect::Async, Effect::Alloc];
    checker.compute_unhandled_effects(&all_effects);

    // Propagate unhandled effects
    checker.propagate_effects_to_parent();

    // Alloc should now be in required effects
    let required = checker.get_fn_required_effects();
    assert!(required.iter().any(|e| matches!(e, Effect::Alloc)));
}

#[test]
fn verify_handle_expression_type_check() {
    let mut checker = Checker::new();

    let handle_expr = sp(ast::Expr::Handle {
        expr: Box::new(sp(ast::Expr::Literal(ast::Literal::Unit))),
        arms: vec![
            ast::HandleArm {
                kind: ast::HandleArmKind::Return,
                pattern: None,
                body: sp(ast::Expr::Literal(ast::Literal::Int(42))),
            },
        ],
    });

    let ty = checker.infer_expr(&handle_expr);
    // Handle expression should return the type of its arms
    assert_eq!(ty, Type::Int);
}

#[test]
fn verify_scope_with_expression_validated() {
    let mut checker = Checker::new();

    let scope_expr = sp(ast::Expr::Scope {
        label: None,
        options: Some(Box::new(sp(ast::Expr::Literal(ast::Literal::Int(42))))),
        body: ast::Block {
            stmts: vec![],
            ret: Some(Box::new(sp(ast::Expr::Literal(ast::Literal::Unit)))),
        },
    });

    let ty = checker.infer_expr(&scope_expr);
    assert_eq!(ty, Type::Unit);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_unhandled_effects_with_multiple_arms() {
    let mut checker = Checker::new();
    use ect::ty::Effect;

    // Mark multiple effects as handled
    checker.mark_effect_handled("async".into());
    checker.mark_effect_handled("exn".into());

    let all_effects = vec![Effect::Async, Effect::Alloc, Effect::Nondet];
    checker.compute_unhandled_effects(&all_effects);

    let unhandled = checker.get_unhandled_effects();
    assert_eq!(unhandled.len(), 2);
}

#[test]
fn verify_alloc_effect_matches_io_handler() {
    let mut checker = Checker::new();
    use ect::ty::Effect;

    // io handler should handle Alloc effect
    checker.mark_effect_handled("io".into());
    let all_effects = vec![Effect::Alloc];

    checker.compute_unhandled_effects(&all_effects);
    assert!(checker.get_unhandled_effects().is_empty());
}

// Effect Subsumption Tests (Function Type Compatibility)

#[test]
fn verify_effect_subsumption_pure_for_pure_exn() {
    let checker = Checker::new();
    use ect::ty::Effect;

    // Pure function can be used where <pure, exn> is expected
    // Function declares: <pure>
    let declared = vec![Effect::Total];
    // Context expects: <pure, exn>
    let expected = vec![Effect::Total, Effect::Exn(Box::new(Type::Unknown))];

    // Check if expected effects can cover declared effects
    assert!(checker.effects_subsume(&declared, &expected));
}

#[test]
fn verify_effect_subsumption_pure_for_multiple_effects() {
    let checker = Checker::new();
    use ect::ty::Effect;

    // Pure function can be used where multiple effects are expected
    let declared = vec![Effect::Total]; // <pure>
    let expected = vec![Effect::Total, Effect::Async, Effect::Alloc]; // <pure, async, alloc>

    assert!(checker.effects_subsume(&declared, &expected));
}

#[test]
fn verify_effect_subsumption_async_for_async_io() {
    let checker = Checker::new();
    use ect::ty::Effect;

    // Async function can be used where <async, alloc> is expected
    let declared = vec![Effect::Async]; // <async>
    let expected = vec![Effect::Async, Effect::Alloc]; // <async, alloc>

    assert!(checker.effects_subsume(&declared, &expected));
}

#[test]
fn verify_effect_subsumption_more_effects_not_subsumed() {
    let checker = Checker::new();
    use ect::ty::Effect;

    // Function with more effects cannot be used where fewer effects expected
    let declared = vec![Effect::Async, Effect::Alloc]; // <async, alloc>
    let expected = vec![Effect::Async]; // <async>

    assert!(!checker.effects_subsume(&declared, &expected));
}

#[test]
fn verify_effect_subsumption_different_exception_types() {
    let checker = Checker::new();
    use ect::ty::Effect;

    // Different exception types should not be subsumed
    let provided = vec![Effect::Exn(Box::new(Type::Int))];
    let required = vec![Effect::Exn(Box::new(Type::String))];

    assert!(!checker.effects_subsume(&provided, &required));
}

#[test]
fn verify_effect_subsumption_empty_to_any() {
    let checker = Checker::new();
    use ect::ty::Effect;

    // Empty (no effects) can be used anywhere
    let declared = vec![]; // no effects
    let expected = vec![Effect::Async, Effect::Alloc];

    assert!(checker.effects_subsume(&declared, &expected));
}

#[test]
fn verify_effect_subsumption_exact_match() {
    let checker = Checker::new();
    use ect::ty::Effect;

    // Exact match should subsume
    let declared = vec![Effect::Async, Effect::Alloc];
    let expected = vec![Effect::Async, Effect::Alloc];

    assert!(checker.effects_subsume(&declared, &expected));
}

#[test]
fn verify_function_type_subsumption_pure_for_async_exn() {
    let checker = Checker::new();
    use ect::ty::Effect;

    // Function type: () -> <pure> Int
    let provided_fn = Type::Function {
        params: vec![],
        effects: vec![Effect::Total],
        ret: Box::new(Type::Int),
    };

    // Expected: () -> <pure, async, exn> Int
    // Note: pure function can be used where pure + other effects are expected
    let required_fn = Type::Function {
        params: vec![],
        effects: vec![Effect::Total, Effect::Async, Effect::Exn(Box::new(Type::Unknown))],
        ret: Box::new(Type::Int),
    };

    // Check function compatibility - return types match, declared effects subsume expected
    match (&provided_fn, &required_fn) {
        (
            Type::Function { effects: prov_effects, ret: prov_ret, .. },
            Type::Function { effects: req_effects, ret: req_ret, .. }
        ) => {
            assert_eq!(prov_ret, req_ret); // return types must match
            assert!(checker.effects_subsume(prov_effects, req_effects)); // declared subsume expected
        },
        _ => panic!("Expected function types"),
    }
}

#[test]
fn verify_function_type_subsumption_incompatible_return() {
    let _checker = Checker::new();
    use ect::ty::Effect;

    // Function type: () -> <pure> Int
    let provided_fn = Type::Function {
        params: vec![],
        effects: vec![Effect::Total],
        ret: Box::new(Type::Int),
    };

    // Expected: () -> <pure> String
    let required_fn = Type::Function {
        params: vec![],
        effects: vec![Effect::Total],
        ret: Box::new(Type::String),
    };

    // Return types differ, so not compatible
    match (&provided_fn, &required_fn) {
        (
            Type::Function { ret: prov_ret, .. },
            Type::Function { ret: req_ret, .. }
        ) => {
            assert_ne!(prov_ret, req_ret);
        },
        _ => panic!("Expected function types"),
    }
}

#[test]
fn verify_effect_subsumption_with_nondet() {
    let checker = Checker::new();
    use ect::ty::Effect;

    // Function declares <nondet, alloc> but context expects <nondet>
    let declared = vec![Effect::Nondet, Effect::Alloc]; // <nondet, alloc>
    let expected = vec![Effect::Nondet]; // <nondet>

    assert!(!checker.effects_subsume(&declared, &expected));
}

#[test]
fn verify_effect_subsumption_nondet_subsumed() {
    let checker = Checker::new();
    use ect::ty::Effect;

    // Nondet function can be used where nondet + alloc is expected
    let declared = vec![Effect::Nondet]; // <nondet>
    let expected = vec![Effect::Nondet, Effect::Alloc]; // <nondet, alloc>

    assert!(checker.effects_subsume(&declared, &expected));
}

#[test]
fn verify_effect_subsumption_mixed_effects() {
    let checker = Checker::new();
    use ect::ty::Effect;

    // Function with subset of effects can be used where more effects expected
    let declared = vec![Effect::Async, Effect::Total]; // <async, pure>
    let expected = vec![Effect::Async, Effect::Total, Effect::Alloc]; // <async, pure, alloc>

    assert!(checker.effects_subsume(&declared, &expected));
}

#[test]
fn verify_effect_subsumption_missing_one_effect() {
    let checker = Checker::new();
    use ect::ty::Effect;

    // Function produces <async, alloc>, but code expects <async, alloc, nondet>
    // Since function's effects are subset of expected, it CAN be used
    let declared = vec![Effect::Async, Effect::Alloc]; // <async, alloc>
    let expected = vec![Effect::Async, Effect::Alloc, Effect::Nondet]; // <async, alloc, nondet>

    // Declared effects are subset of expected - function can be used
    assert!(checker.effects_subsume(&declared, &expected));
}

#[test]
fn verify_effect_subsumption_function_produces_more() {
    let checker = Checker::new();
    use ect::ty::Effect;

    // Function produces MORE effects than context expects to handle
    let declared = vec![Effect::Async, Effect::Alloc, Effect::Nondet]; // <async, alloc, nondet>
    let expected = vec![Effect::Async, Effect::Alloc]; // <async, alloc>

    // Function produces effects context doesn't expect - NOT compatible
    assert!(!checker.effects_subsume(&declared, &expected));
}

#[test]
fn verify_closure_with_pure_effects_subsumes_with_exn() {
    let mut checker = Checker::new();
    use ect::ty::Effect;

    // Closure declared as <pure>
    let closure_effects = vec![Effect::Total];
    checker.set_fn_declared_effects(closure_effects.clone());

    // Context expects <pure, exn>
    let expected_effects = vec![Effect::Total, Effect::Exn(Box::new(Type::Unknown))];

    // Pure closure effects should subsume expected effects
    assert!(checker.effects_subsume(&closure_effects, &expected_effects));
}

#[test]
fn verify_effect_subsumption_order_independent() {
    let checker = Checker::new();
    use ect::ty::Effect;

    // Order shouldn't matter for subsumption
    let declared_a = vec![Effect::Async, Effect::Total];
    let declared_b = vec![Effect::Total, Effect::Async];
    let expected = vec![Effect::Async, Effect::Total, Effect::Alloc];

    assert!(checker.effects_subsume(&declared_a, &expected));
    assert!(checker.effects_subsume(&declared_b, &expected));
}

// Closure Effect Declaration Validation

#[test]
fn verify_closure_declared_pure_valid() {
    let mut checker = Checker::new();
    use ect::ty::Effect;

    let declared = vec![Effect::Total]; // <pure>
    let inferred = vec![]; // body has no effects

    assert!(checker.validate_closure_effects(&declared, &inferred, d_span()));
}

#[test]
fn verify_closure_declared_pure_with_io_call_invalid() {
    let mut checker = Checker::new();
    use ect::ty::Effect;

    let declared = vec![Effect::Total]; // <pure>
    let inferred = vec![Effect::Alloc]; // body calls IO function

    assert!(!checker.validate_closure_effects(&declared, &inferred, d_span()));
    assert!(checker.errors.len() > 0);
}

#[test]
fn verify_closure_declared_async_valid() {
    let mut checker = Checker::new();
    use ect::ty::Effect;

    let declared = vec![Effect::Async];
    let inferred = vec![Effect::Async];

    assert!(checker.validate_closure_effects(&declared, &inferred, d_span()));
}

#[test]
fn verify_closure_declared_async_exact_match() {
    let mut checker = Checker::new();
    use ect::ty::Effect;

    let declared = vec![Effect::Async];
    let inferred = vec![Effect::Async];

    assert!(checker.validate_closure_effects(&declared, &inferred, d_span()));
}

#[test]
fn verify_closure_declared_multiple_effects_subset() {
    let mut checker = Checker::new();
    use ect::ty::Effect;

    // Closure declares <async, alloc>
    let declared = vec![Effect::Async, Effect::Alloc];
    // Body only uses async
    let inferred = vec![Effect::Async];

    assert!(checker.validate_closure_effects(&declared, &inferred, d_span()));
}

#[test]
fn verify_closure_declared_insufficient_effects() {
    let mut checker = Checker::new();
    use ect::ty::Effect;

    // Closure declares <async>
    let declared = vec![Effect::Async];
    // Body uses both async and alloc
    let inferred = vec![Effect::Async, Effect::Alloc];

    assert!(!checker.validate_closure_effects(&declared, &inferred, d_span()));
}

#[test]
fn verify_closure_no_declaration_accepts_any() {
    let mut checker = Checker::new();
    use ect::ty::Effect;

    let declared = vec![]; // No effects declared
    let inferred = vec![Effect::Async, Effect::Alloc]; // Body has effects

    assert!(checker.validate_closure_effects(&declared, &inferred, d_span()));
}

#[test]
fn verify_inferred_effects_exceed_declared_single() {
    let checker = Checker::new();
    use ect::ty::Effect;

    let declared = vec![Effect::Total]; // <pure>
    let inferred = vec![Effect::Alloc]; // has IO

    let exceeds = checker.inferred_effects_exceed_declared(&declared, &inferred);
    assert_eq!(exceeds.len(), 1);
}

#[test]
fn verify_inferred_effects_exceed_declared_multiple() {
    let checker = Checker::new();
    use ect::ty::Effect;

    let declared = vec![Effect::Total]; // <pure>
    let inferred = vec![Effect::Async, Effect::Alloc]; // has async and IO

    let exceeds = checker.inferred_effects_exceed_declared(&declared, &inferred);
    assert_eq!(exceeds.len(), 2);
}

#[test]
fn verify_inferred_effects_subset_of_declared() {
    let checker = Checker::new();
    use ect::ty::Effect;

    let declared = vec![Effect::Async, Effect::Alloc, Effect::Nondet];
    let inferred = vec![Effect::Async, Effect::Alloc];

    let exceeds = checker.inferred_effects_exceed_declared(&declared, &inferred);
    assert_eq!(exceeds.len(), 0);
}

#[test]
fn verify_all_effects_declared_true() {
    let checker = Checker::new();
    use ect::ty::Effect;

    let declared = vec![Effect::Async, Effect::Alloc];
    let inferred = vec![Effect::Async];

    assert!(checker.all_effects_declared(&declared, &inferred));
}

#[test]
fn verify_all_effects_declared_false() {
    let checker = Checker::new();
    use ect::ty::Effect;

    let declared = vec![Effect::Async];
    let inferred = vec![Effect::Async, Effect::Alloc];

    assert!(!checker.all_effects_declared(&declared, &inferred));
}

#[test]
fn verify_all_effects_declared_exact_match() {
    let checker = Checker::new();
    use ect::ty::Effect;

    let declared = vec![Effect::Async, Effect::Alloc];
    let inferred = vec![Effect::Async, Effect::Alloc];

    assert!(checker.all_effects_declared(&declared, &inferred));
}

#[test]
fn verify_closure_with_pure_declaration_io_call_error() {
    let mut checker = Checker::new();
    use ect::ty::Effect;

    // Closure declared as |x| -> <pure> but calls IO
    let declared = vec![Effect::Total];
    let inferred = vec![Effect::Alloc];

    let result = checker.validate_closure_effects(&declared, &inferred, d_span());
    assert!(!result);
    assert_eq!(checker.errors.len(), 1);
    assert!(checker.errors[0].message.contains("effect") &&
            checker.errors[0].message.contains("not"));
}

#[test]
fn verify_closure_declared_effects_with_exn_type() {
    let mut checker = Checker::new();
    use ect::ty::Effect;

    // Closure declared with specific exception type
    let declared = vec![Effect::Exn(Box::new(Type::String))];
    let inferred = vec![Effect::Exn(Box::new(Type::String))];

    assert!(checker.validate_closure_effects(&declared, &inferred, d_span()));
}

#[test]
fn verify_closure_exn_type_mismatch_in_declaration() {
    let mut checker = Checker::new();
    use ect::ty::Effect;

    // Closure declares <exn<String>> but body throws <exn<Int>>
    let declared = vec![Effect::Exn(Box::new(Type::String))];
    let inferred = vec![Effect::Exn(Box::new(Type::Int))];

    assert!(!checker.validate_closure_effects(&declared, &inferred, d_span()));
}

#[test]
fn verify_closure_mixed_declared_vs_inferred() {
    let mut checker = Checker::new();
    use ect::ty::Effect;

    // Closure declares <pure, async, alloc>
    let declared = vec![Effect::Total, Effect::Async, Effect::Alloc];
    // Body only uses <async, alloc>
    let inferred = vec![Effect::Async, Effect::Alloc];

    assert!(checker.validate_closure_effects(&declared, &inferred, d_span()));
}

#[test]
fn verify_closure_over_declared_single_extra_effect() {
    let mut checker = Checker::new();
    use ect::ty::Effect;

    // Closure declares <async, alloc>
    let declared = vec![Effect::Async, Effect::Alloc];
    // Body uses <async, alloc, nondet>
    let inferred = vec![Effect::Async, Effect::Alloc, Effect::Nondet];

    assert!(!checker.validate_closure_effects(&declared, &inferred, d_span()));
}

#[test]
fn verify_empty_declared_empty_inferred() {
    let mut checker = Checker::new();

    let declared = vec![];
    let inferred = vec![];

    assert!(checker.validate_closure_effects(&declared, &inferred, d_span()));
}

// --- typeck_const_effect_tests ---

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
    checker.insert_const_var("MAX_SIZE".into(), Type::Int);

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
