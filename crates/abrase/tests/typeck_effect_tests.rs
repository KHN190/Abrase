use abrase::ast::{BinaryOp, EffectItem, Expr, Literal, Span, Spanned, self};
use abrase::ty::Type;
use abrase::typeck::Checker;

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
    use abrase::ty::Effect;

    let effects = vec![Effect::Nondet, Effect::Alloc];
    checker.register_effect_alias("concurrent".into(), effects.clone());

    let alias = checker.get_effect_alias("concurrent");
    assert!(alias.is_some());
    assert_eq!(alias.unwrap().len(), 2);
}

#[test]
fn verify_push_and_pop_effect() {
    let mut checker = Checker::new();
    use abrase::ty::Effect;

    checker.push_effect(Effect::Nondet);
    checker.push_effect(Effect::Alloc);

    let expected = vec![Effect::Nondet, Effect::Alloc];
    assert!(checker.effects_compatible(&expected, &expected));

    checker.pop_effect();
    let expected2 = vec![Effect::Nondet];
    assert!(checker.effects_compatible(&expected2, &expected2));
}

#[test]
fn verify_effects_equal_total() {
    let checker = Checker::new();
    use abrase::ty::Effect;

    assert!(checker.effects_equal(&Effect::Total, &Effect::Total));
    assert!(!checker.effects_equal(&Effect::Total, &Effect::Nondet));
}

#[test]
fn verify_effects_equal_alloc() {
    let checker = Checker::new();
    use abrase::ty::Effect;

    assert!(checker.effects_equal(&Effect::Alloc, &Effect::Alloc));
}

#[test]
fn verify_effects_equal_nondet() {
    let checker = Checker::new();
    use abrase::ty::Effect;

    assert!(checker.effects_equal(&Effect::Nondet, &Effect::Nondet));
}

#[test]
fn verify_effects_equal_exn_same_type() {
    let checker = Checker::new();
    use abrase::ty::Effect;

    let exn1 = Effect::Exn(Box::new(Type::String));
    let exn2 = Effect::Exn(Box::new(Type::String));

    assert!(checker.effects_equal(&exn1, &exn2));
}

#[test]
fn verify_effects_equal_exn_different_type() {
    let checker = Checker::new();
    use abrase::ty::Effect;

    let exn1 = Effect::Exn(Box::new(Type::String));
    let exn2 = Effect::Exn(Box::new(Type::Int));

    assert!(!checker.effects_equal(&exn1, &exn2));
}

#[test]
fn verify_effects_compatible_empty() {
    let checker = Checker::new();

    assert!(checker.effects_compatible(&[], &[]));
    assert!(checker.effects_compatible(&[], &vec![abrase::ty::Effect::Nondet]));
}

#[test]
fn verify_effects_compatible_single_effect() {
    let checker = Checker::new();
    use abrase::ty::Effect;

    let expected = vec![Effect::Nondet];
    let actual = vec![Effect::Nondet];

    assert!(checker.effects_compatible(&expected, &actual));
}

#[test]
fn verify_effects_compatible_subset() {
    let checker = Checker::new();
    use abrase::ty::Effect;

    let expected = vec![Effect::Nondet];
    let actual = vec![Effect::Nondet, Effect::Alloc];

    assert!(checker.effects_compatible(&expected, &actual));
}

#[test]
fn verify_effects_compatible_missing_effect() {
    let checker = Checker::new();
    use abrase::ty::Effect;

    let expected = vec![Effect::Nondet, Effect::Alloc];
    let actual = vec![Effect::Nondet];

    assert!(!checker.effects_compatible(&expected, &actual));
}

#[test]
fn verify_convert_effect_io() {
    let checker = Checker::new();
    use abrase::ty::Effect;

    let effect_item = ast::EffectItem {
        name: vec!["io".into()],
        arg: None,
    };

    let converted = checker.convert_effect(&effect_item);
    assert!(converted.is_some());
    assert!(matches!(converted.unwrap(), Effect::Alloc));
}

#[test]
fn verify_convert_effect_unknown_returns_none() {
    let checker = Checker::new();

    let effect_item = ast::EffectItem {
        name: vec!["unknown".into()],
        arg: None,
    };

    assert!(checker.convert_effect(&effect_item).is_none());
}

#[test]
fn verify_convert_effect_exn_no_arg() {
    let checker = Checker::new();
    use abrase::ty::Effect;

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
    use abrase::ty::Effect;

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
    use abrase::ty::Effect;

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
    use abrase::ty::Effect;

    let fn_type = Type::Function {
        params: vec![Type::Int],
        effects: vec![Effect::Nondet],
        ret: Box::new(Type::Bool),
    };

    match fn_type {
        Type::Function { effects, .. } => {
            assert_eq!(effects.len(), 1);
            assert!(matches!(&effects[0], Effect::Nondet));
        },
        _ => panic!("Expected function type"),
    }
}

#[test]
fn verify_function_type_multiple_effects() {
    let _checker = Checker::new();
    use abrase::ty::Effect;

    let fn_type = Type::Function {
        params: vec![Type::Int],
        effects: vec![Effect::Nondet, Effect::Alloc],
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
    use abrase::ty::Effect;

    assert!(matches!(Effect::Total, Effect::Total));
}

#[test]
fn verify_convert_effect_alloc() {
    let checker = Checker::new();
    use abrase::ty::Effect;

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
    use abrase::ty::Effect;

    let expected = vec![Effect::Nondet, Effect::Alloc];
    let actual = vec![Effect::Nondet, Effect::Alloc, Effect::Nondet];

    assert!(checker.effects_compatible(&expected, &actual));
}

#[test]
fn verify_effect_compatibility_order_independent() {
    let checker = Checker::new();
    use abrase::ty::Effect;

    let expected = vec![Effect::Alloc, Effect::Nondet];
    let actual = vec![Effect::Nondet, Effect::Alloc];

    assert!(checker.effects_compatible(&expected, &actual));
}

// Effect Unification & Inference Tests

#[test]
fn verify_set_fn_declared_effects() {
    let mut checker = Checker::new();
    use abrase::ty::Effect;

    let effects = vec![Effect::Nondet, Effect::Alloc];
    checker.set_fn_declared_effects(effects.clone());

    let declared = checker.get_fn_declared_effects();
    assert_eq!(declared.len(), 2);
}

#[test]
fn verify_add_required_effect() {
    let mut checker = Checker::new();
    use abrase::ty::Effect;

    checker.add_required_effect(Effect::Nondet);
    checker.add_required_effect(Effect::Alloc);

    let required = checker.get_fn_required_effects();
    assert_eq!(required.len(), 2);
}

#[test]
fn verify_add_required_effect_no_duplicates() {
    let mut checker = Checker::new();
    use abrase::ty::Effect;

    checker.add_required_effect(Effect::Nondet);
    checker.add_required_effect(Effect::Nondet);

    let required = checker.get_fn_required_effects();
    assert_eq!(required.len(), 1);
}

#[test]
fn verify_check_effect_compatibility_satisfied() {
    let mut checker = Checker::new();
    use abrase::ty::Effect;

    checker.add_required_effect(Effect::Nondet);
    let provided = vec![Effect::Nondet, Effect::Alloc];

    let result = checker.check_effect_compatibility(&provided, d_span());
    assert!(result);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_check_effect_compatibility_unsatisfied() {
    let mut checker = Checker::new();
    use abrase::ty::Effect;

    checker.add_required_effect(Effect::Nondet);
    let provided = vec![Effect::Alloc];

    let result = checker.check_effect_compatibility(&provided, d_span());
    assert!(!result);
    assert!(!checker.errors.is_empty());
}

#[test]
fn verify_unify_effects_no_duplicates() {
    let checker = Checker::new();
    use abrase::ty::Effect;

    let effects1 = vec![Effect::Nondet, Effect::Alloc];
    let effects2 = vec![Effect::Nondet, Effect::Exn(Box::new(Type::Int))];

    let unified = checker.unify_effects(&effects1, &effects2);
    assert_eq!(unified.len(), 3);
}

#[test]
fn verify_unify_effects_empty_left() {
    let checker = Checker::new();
    use abrase::ty::Effect;

    let effects1 = vec![];
    let effects2 = vec![Effect::Nondet];

    let unified = checker.unify_effects(&effects1, &effects2);
    assert_eq!(unified.len(), 1);
}

#[test]
fn verify_unify_effects_empty_right() {
    let checker = Checker::new();
    use abrase::ty::Effect;

    let effects1 = vec![Effect::Nondet];
    let effects2 = vec![];

    let unified = checker.unify_effects(&effects1, &effects2);
    assert_eq!(unified.len(), 1);
}

#[test]
fn verify_effects_subsume_all_provided() {
    let checker = Checker::new();
    use abrase::ty::Effect;

    let required = vec![Effect::Nondet];
    let provided = vec![Effect::Nondet, Effect::Alloc];

    assert!(checker.effects_subsume(&required, &provided));
}

#[test]
fn verify_effects_subsume_missing() {
    let checker = Checker::new();
    use abrase::ty::Effect;

    let required = vec![Effect::Nondet, Effect::Alloc];
    let provided = vec![Effect::Nondet];

    assert!(!checker.effects_subsume(&required, &provided));
}

#[test]
fn verify_effects_subsume_empty_required() {
    let checker = Checker::new();
    use abrase::ty::Effect;

    let required = vec![];
    let provided = vec![Effect::Nondet];

    assert!(checker.effects_subsume(&required, &provided));
}

#[test]
fn verify_infer_closure_effects_with_declared() {
    let mut checker = Checker::new();
    use abrase::ty::Effect;

    let declared = vec![Effect::Nondet];
    checker.set_fn_declared_effects(declared.clone());

    let body_effects = vec![Effect::Alloc];
    let inferred = checker.infer_closure_effects(&body_effects);

    assert_eq!(inferred, declared);
}

#[test]
fn verify_infer_closure_effects_without_declared() {
    let checker = Checker::new();
    use abrase::ty::Effect;

    let body_effects = vec![Effect::Alloc, Effect::Nondet];
    let inferred = checker.infer_closure_effects(&body_effects);

    assert_eq!(inferred, body_effects);
}

#[test]
fn verify_convert_effect_items_single() {
    let checker = Checker::new();

    let items = vec![ast::EffectItem {
        name: vec!["nondet".into()],
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
            name: vec!["nondet".into()],
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
        return_type: Some(ast::Type::Named("Int".into())),
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
    use abrase::ty::Effect;

    let initial_effects = vec![Effect::Nondet];
    checker.set_fn_declared_effects(initial_effects);

    let closure_expr = sp(ast::Expr::Closure {
        is_move: false,
        params: vec![],
        effects: vec![],
        return_type: None,
        body: Box::new(sp(ast::Expr::Literal(ast::Literal::Unit))),
    });

    checker.infer_expr(&closure_expr);

    // After closure inference, declared effects should be empty
    assert!(checker.get_fn_declared_effects().is_empty());
}

#[test]
fn verify_required_effects_accumulate() {
    let mut checker = Checker::new();
    use abrase::ty::Effect;

    checker.add_required_effect(Effect::Nondet);
    checker.add_required_effect(Effect::Alloc);
    checker.add_required_effect(Effect::Exn(Box::new(Type::Int)));

    let required = checker.get_fn_required_effects();
    assert_eq!(required.len(), 3);
}

#[test]
fn verify_effect_compatibility_with_exn() {
    let checker = Checker::new();
    use abrase::ty::Effect;

    let required = vec![Effect::Exn(Box::new(Type::String))];
    let provided = vec![Effect::Exn(Box::new(Type::String)), Effect::Nondet];

    assert!(checker.effects_subsume(&required, &provided));
}

#[test]
fn verify_effect_compatibility_exn_type_mismatch() {
    let checker = Checker::new();
    use abrase::ty::Effect;

    let required = vec![Effect::Exn(Box::new(Type::String))];
    let provided = vec![Effect::Exn(Box::new(Type::Int))];

    assert!(!checker.effects_subsume(&required, &provided));
}

// Effect Shadowing, Propagation & Scope Semantics Tests

#[test]
fn verify_mark_effect_handled() {
    let mut checker = Checker::new();

    checker.mark_effect_handled("nondet".into());
    checker.mark_effect_handled("io".into());

    let handled = checker.get_handled_effects();
    assert_eq!(handled.len(), 2);
    assert!(handled.contains(&"nondet".to_string()));
    assert!(handled.contains(&"io".to_string()));
}

#[test]
fn verify_mark_effect_handled_no_duplicates() {
    let mut checker = Checker::new();

    checker.mark_effect_handled("nondet".into());
    checker.mark_effect_handled("nondet".into());

    let handled = checker.get_handled_effects();
    assert_eq!(handled.len(), 1);
}

#[test]
fn verify_compute_unhandled_effects_all_handled() {
    let mut checker = Checker::new();
    use abrase::ty::Effect;

    checker.mark_effect_handled("nondet".into());
    let all_effects = vec![Effect::Nondet];

    checker.compute_unhandled_effects(&all_effects);
    assert!(checker.get_unhandled_effects().is_empty());
}

#[test]
fn verify_compute_unhandled_effects_partial_handled() {
    let mut checker = Checker::new();
    use abrase::ty::Effect;

    checker.mark_effect_handled("nondet".into());
    let all_effects = vec![Effect::Nondet, Effect::Alloc];

    checker.compute_unhandled_effects(&all_effects);
    let unhandled = checker.get_unhandled_effects();
    assert_eq!(unhandled.len(), 1);
    assert!(matches!(&unhandled[0], Effect::Alloc));
}

#[test]
fn verify_compute_unhandled_effects_none_handled() {
    let mut checker = Checker::new();
    use abrase::ty::Effect;

    let all_effects = vec![Effect::Nondet, Effect::Alloc];

    checker.compute_unhandled_effects(&all_effects);
    let unhandled = checker.get_unhandled_effects();
    assert_eq!(unhandled.len(), 2);
}

#[test]
fn verify_compute_unhandled_effects_exn_handling() {
    let mut checker = Checker::new();
    use abrase::ty::Effect;

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

    checker.mark_effect_handled("nondet".into());
    assert!(!checker.get_handled_effects().is_empty());

    checker.clear_handle_context();
    assert!(checker.get_handled_effects().is_empty());
    assert!(checker.get_unhandled_effects().is_empty());
}

#[test]
fn verify_effect_propagation_accumulates_in_required() {
    let mut checker = Checker::new();
    use abrase::ty::Effect;

    // Add some required effects first
    checker.add_required_effect(Effect::Nondet);

    // Mark some effects as handled and compute unhandled
    checker.mark_effect_handled("nondet".into());
    let all_effects = vec![Effect::Nondet, Effect::Alloc];
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
fn verify_unhandled_effects_with_multiple_arms() {
    let mut checker = Checker::new();
    use abrase::ty::Effect;

    // Mark multiple effects as handled
    checker.mark_effect_handled("nondet".into());
    checker.mark_effect_handled("exn".into());

    let all_effects = vec![Effect::Nondet, Effect::Alloc, Effect::Exn(Box::new(Type::Int))];
    checker.compute_unhandled_effects(&all_effects);

    let unhandled = checker.get_unhandled_effects();
    assert_eq!(unhandled.len(), 1);
    assert!(matches!(&unhandled[0], Effect::Alloc));
}

#[test]
fn verify_alloc_effect_matches_io_handler() {
    let mut checker = Checker::new();
    use abrase::ty::Effect;

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
    use abrase::ty::Effect;

    // Pure function can be used where <pure, exn> is expected
    // Function declares: <pure>
    let declared = vec![Effect::Total];
    // Context expects: <pure, exn>
    let expected = vec![Effect::Total, Effect::Exn(Box::new(Type::Unknown))];

    // Check if expected effects can cover declared effects
    assert!(checker.effects_subsume(&declared, &expected));
}

#[test]
fn verify_effect_subsumption_different_exception_types() {
    let checker = Checker::new();
    use abrase::ty::Effect;

    // Different exception types should not be subsumed
    let provided = vec![Effect::Exn(Box::new(Type::Int))];
    let required = vec![Effect::Exn(Box::new(Type::String))];

    assert!(!checker.effects_subsume(&provided, &required));
}

#[test]
fn verify_effect_subsumption_empty_to_any() {
    let checker = Checker::new();
    use abrase::ty::Effect;

    // Empty (no effects) can be used anywhere
    let declared = vec![]; // no effects
    let expected = vec![Effect::Nondet, Effect::Alloc];

    assert!(checker.effects_subsume(&declared, &expected));
}

#[test]
fn verify_effect_subsumption_exact_match() {
    let checker = Checker::new();
    use abrase::ty::Effect;

    // Exact match should subsume
    let declared = vec![Effect::Nondet, Effect::Alloc];
    let expected = vec![Effect::Nondet, Effect::Alloc];

    assert!(checker.effects_subsume(&declared, &expected));
}

#[test]
fn verify_function_type_subsumption_incompatible_return() {
    let _checker = Checker::new();
    use abrase::ty::Effect;

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
    use abrase::ty::Effect;

    // Function declares <nondet, alloc> but context expects <nondet>
    let declared = vec![Effect::Nondet, Effect::Alloc]; // <nondet, alloc>
    let expected = vec![Effect::Nondet]; // <nondet>

    assert!(!checker.effects_subsume(&declared, &expected));
}

#[test]
fn verify_effect_subsumption_nondet_subsumed() {
    let checker = Checker::new();
    use abrase::ty::Effect;

    // Nondet function can be used where nondet + alloc is expected
    let declared = vec![Effect::Nondet]; // <nondet>
    let expected = vec![Effect::Nondet, Effect::Alloc]; // <nondet, alloc>

    assert!(checker.effects_subsume(&declared, &expected));
}

#[test]
fn verify_effect_subsumption_function_produces_more() {
    let checker = Checker::new();
    use abrase::ty::Effect;

    // Function produces MORE effects than context expects to handle
    let declared = vec![Effect::Nondet, Effect::Alloc, Effect::Exn(Box::new(Type::Int))];
    let expected = vec![Effect::Nondet, Effect::Alloc];

    // Function produces effects context doesn't expect - NOT compatible
    assert!(!checker.effects_subsume(&declared, &expected));
}

#[test]
fn verify_closure_with_pure_effects_subsumes_with_exn() {
    let mut checker = Checker::new();
    use abrase::ty::Effect;

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
    use abrase::ty::Effect;

    // Order shouldn't matter for subsumption
    let declared_a = vec![Effect::Nondet, Effect::Total];
    let declared_b = vec![Effect::Total, Effect::Nondet];
    let expected = vec![Effect::Nondet, Effect::Total, Effect::Alloc];

    assert!(checker.effects_subsume(&declared_a, &expected));
    assert!(checker.effects_subsume(&declared_b, &expected));
}

// Closure Effect Declaration Validation

#[test]
fn verify_closure_declared_pure_valid() {
    let mut checker = Checker::new();
    use abrase::ty::Effect;

    let declared = vec![Effect::Total]; // <pure>
    let inferred = vec![]; // body has no effects

    assert!(checker.validate_closure_effects(&declared, &inferred, d_span()));
}

#[test]
fn verify_closure_declared_pure_with_io_call_invalid() {
    let mut checker = Checker::new();
    use abrase::ty::Effect;

    let declared = vec![Effect::Total]; // <pure>
    let inferred = vec![Effect::Alloc]; // body calls IO function

    assert!(!checker.validate_closure_effects(&declared, &inferred, d_span()));
    assert!(checker.errors.len() > 0);
}

#[test]
fn verify_closure_no_declaration_accepts_any() {
    let mut checker = Checker::new();
    use abrase::ty::Effect;

    let declared = vec![]; // No effects declared
    let inferred = vec![Effect::Nondet, Effect::Alloc]; // Body has effects

    assert!(checker.validate_closure_effects(&declared, &inferred, d_span()));
}

#[test]
fn verify_inferred_effects_exceed_declared_single() {
    let checker = Checker::new();
    use abrase::ty::Effect;

    let declared = vec![Effect::Total]; // <pure>
    let inferred = vec![Effect::Alloc]; // has IO

    let exceeds = checker.inferred_effects_exceed_declared(&declared, &inferred);
    assert_eq!(exceeds.len(), 1);
}

#[test]
fn verify_inferred_effects_subset_of_declared() {
    let checker = Checker::new();
    use abrase::ty::Effect;

    let declared = vec![Effect::Nondet, Effect::Alloc, Effect::Nondet];
    let inferred = vec![Effect::Nondet, Effect::Alloc];

    let exceeds = checker.inferred_effects_exceed_declared(&declared, &inferred);
    assert_eq!(exceeds.len(), 0);
}

#[test]
fn verify_all_effects_declared_true() {
    let checker = Checker::new();
    use abrase::ty::Effect;

    let declared = vec![Effect::Nondet, Effect::Alloc];
    let inferred = vec![Effect::Nondet];

    assert!(checker.all_effects_declared(&declared, &inferred));
}

#[test]
fn verify_all_effects_declared_false() {
    let checker = Checker::new();
    use abrase::ty::Effect;

    let declared = vec![Effect::Nondet];
    let inferred = vec![Effect::Nondet, Effect::Alloc];

    assert!(!checker.all_effects_declared(&declared, &inferred));
}

#[test]
fn verify_all_effects_declared_exact_match() {
    let checker = Checker::new();
    use abrase::ty::Effect;

    let declared = vec![Effect::Nondet, Effect::Alloc];
    let inferred = vec![Effect::Nondet, Effect::Alloc];

    assert!(checker.all_effects_declared(&declared, &inferred));
}

#[test]
fn verify_closure_with_pure_declaration_io_call_error() {
    let mut checker = Checker::new();
    use abrase::ty::Effect;

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
    use abrase::ty::Effect;

    // Closure declared with specific exception type
    let declared = vec![Effect::Exn(Box::new(Type::String))];
    let inferred = vec![Effect::Exn(Box::new(Type::String))];

    assert!(checker.validate_closure_effects(&declared, &inferred, d_span()));
}

#[test]
fn verify_closure_exn_type_mismatch_in_declaration() {
    let mut checker = Checker::new();
    use abrase::ty::Effect;

    // Closure declares <exn<String>> but body throws <exn<Int>>
    let declared = vec![Effect::Exn(Box::new(Type::String))];
    let inferred = vec![Effect::Exn(Box::new(Type::Int))];

    assert!(!checker.validate_closure_effects(&declared, &inferred, d_span()));
}

#[test]
fn verify_closure_over_declared_single_extra_effect() {
    let mut checker = Checker::new();
    use abrase::ty::Effect;

    let declared = vec![Effect::Nondet, Effect::Alloc];
    let inferred = vec![Effect::Nondet, Effect::Alloc, Effect::Exn(Box::new(Type::Int))];

    assert!(!checker.validate_closure_effects(&declared, &inferred, d_span()));
}

#[test]
fn verify_empty_declared_empty_inferred() {
    let mut checker = Checker::new();

    let declared = vec![];
    let inferred = vec![];

    assert!(checker.validate_closure_effects(&declared, &inferred, d_span()));
}


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

// ── Gap tests ─────────────────────────────────────────────────────────────────

#[test]
fn verify_throw_adds_exn_to_required_effects() {
    use abrase::ty::Effect;
    let mut checker = Checker::new();
    let expr = sp(Expr::Throw(Box::new(sp(Expr::Literal(abrase::ast::Literal::String("oops".into()))))));
    checker.infer_expr(&expr);
    let required = checker.get_fn_required_effects();
    assert!(
        required.iter().any(|e| matches!(e, Effect::Exn(_))),
        "throw must add Exn to required effects; got {:?}", required
    );
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

fn throw_int_99() -> Spanned<ast::Expr> {
    sp(ast::Expr::Throw(Box::new(sp(ast::Expr::Literal(ast::Literal::Int(99))))))
}

#[test]
fn verify_undeclared_throw_in_pure_fn_errors() {
    // fn pure_throws() -> Int { throw 99 }
    let fn_decl = ast::FnDecl {
        attrs: vec![],
        is_pub: false,
        name: "pure_throws".into(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(ast::Type::Named("Int".into())),
        where_clause: vec![],
        body: ast::Block { stmts: vec![], ret: Some(Box::new(throw_int_99())) },
    };
    let mut checker = Checker::new();
    checker.check_fn_decl(&fn_decl);

    assert!(
        checker.errors.iter().any(|e|
            e.message.contains("pure_throws") && e.message.contains("does not declare")
        ),
        "expected leak error for undeclared exn; got {:?}",
        checker.errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
}

#[test]
fn verify_declared_throw_in_exn_fn_is_ok() {
    // fn raise() -> <exn<Int>> Int { throw 99 }
    let fn_decl = ast::FnDecl {
        attrs: vec![],
        is_pub: false,
        name: "raise".into(),
        generics: vec![],
        params: vec![],
        effects: vec![ast::EffectItem {
            name: vec!["exn".into()],
            arg: Some(Box::new(ast::Type::Named("Int".into()))),
        }],
        return_type: Some(ast::Type::Named("Int".into())),
        where_clause: vec![],
        body: ast::Block { stmts: vec![], ret: Some(Box::new(throw_int_99())) },
    };
    let mut checker = Checker::new();
    checker.check_fn_decl(&fn_decl);

    assert!(
        checker.errors.is_empty(),
        "declared <exn<Int>> must accept throw 99; got {:?}",
        checker.errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
}

#[test]
fn verify_required_effects_do_not_leak_across_fn_checks() {
    // After a function that produces effects is checked, the next pure function
    // must not inherit those effects.

    // fn raise() -> <exn<Int>> Int { throw 99 }
    let raising = ast::FnDecl {
        attrs: vec![],
        is_pub: false,
        name: "raise".into(),
        generics: vec![],
        params: vec![],
        effects: vec![ast::EffectItem {
            name: vec!["exn".into()],
            arg: Some(Box::new(ast::Type::Named("Int".into()))),
        }],
        return_type: Some(ast::Type::Named("Int".into())),
        where_clause: vec![],
        body: ast::Block { stmts: vec![], ret: Some(Box::new(throw_int_99())) },
    };

    // fn pure() -> Int { 0 }
    let pure = ast::FnDecl {
        attrs: vec![],
        is_pub: false,
        name: "pure".into(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(ast::Type::Named("Int".into())),
        where_clause: vec![],
        body: ast::Block {
            stmts: vec![],
            ret: Some(Box::new(sp(ast::Expr::Literal(ast::Literal::Int(0))))),
        },
    };

    let mut checker = Checker::new();
    checker.check_fn_decl(&raising);
    checker.check_fn_decl(&pure);

    assert!(
        checker.errors.iter().all(|e| !e.message.contains("pure")),
        "the pure fn should not pick up effects from previously-checked raise; got {:?}",
        checker.errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
    assert!(
        checker.get_fn_required_effects().is_empty(),
        "fn_required_effects must not leak past check_fn_decl; got {:?}",
        checker.get_fn_required_effects()
    );
}

#[test]
fn verify_handle_consumes_exn_effect() {
    // fn safe() -> Int { handle (throw 99) { return v => v, throw e => 0 } }
    let body_expr = sp(ast::Expr::Handle {
        expr: Box::new(throw_int_99()),
        arms: vec![
            ast::HandleArm {
                kind: ast::HandleArmKind::Return,
                pattern: Some(sp(ast::Pattern::Bind("v".into()))),
                body: sp(ast::Expr::Identifier("v".into())),
            },
            ast::HandleArm {
                kind: ast::HandleArmKind::Exn,
                pattern: Some(sp(ast::Pattern::Bind("e".into()))),
                body: sp(ast::Expr::Literal(ast::Literal::Int(0))),
            },
        ],
    });
    let fn_decl = ast::FnDecl {
        attrs: vec![],
        is_pub: false,
        name: "safe".into(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(ast::Type::Named("Int".into())),
        where_clause: vec![],
        body: ast::Block { stmts: vec![], ret: Some(Box::new(body_expr)) },
    };
    let mut checker = Checker::new();
    checker.check_fn_decl(&fn_decl);

    assert!(
        !checker.errors.iter().any(|e| e.message.contains("does not declare")),
        "handle should consume the exn effect; got {:?}",
        checker.errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
}

#[test]
fn verify_resume_outside_handler_arm_errors() {
    let mut checker = Checker::new();
    let expr = sp(ast::Expr::Resume(None));
    checker.infer_expr(&expr);
    assert!(
        checker.errors.iter().any(|e| e.message.contains("'resume'")),
        "expected resume-outside-arm error; got {:?}",
        checker.errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
}

#[test]
fn verify_resume_in_exn_arm_is_ok() {
    let body = sp(ast::Expr::Handle {
        expr: Box::new(sp(ast::Expr::Throw(Box::new(sp(ast::Expr::Literal(ast::Literal::Int(99))))))),
        arms: vec![
            ast::HandleArm {
                kind: ast::HandleArmKind::Return,
                pattern: Some(sp(ast::Pattern::Bind("v".into()))),
                body: sp(ast::Expr::Identifier("v".into())),
            },
            ast::HandleArm {
                kind: ast::HandleArmKind::Exn,
                pattern: Some(sp(ast::Pattern::Bind("e".into()))),
                body: sp(ast::Expr::Resume(Some(Box::new(sp(ast::Expr::Literal(ast::Literal::Int(0))))))),
            },
        ],
    });
    let fn_decl = ast::FnDecl {
        attrs: vec![],
        is_pub: false,
        name: "f".into(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(ast::Type::Named("Int".into())),
        where_clause: vec![],
        body: ast::Block { stmts: vec![], ret: Some(Box::new(body)) },
    };
    let mut checker = Checker::new();
    checker.check_fn_decl(&fn_decl);
    assert!(
        !checker.errors.iter().any(|e| e.message.contains("'resume'")),
        "resume in exn arm must be allowed; got {:?}",
        checker.errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
}

#[test]
fn verify_resume_in_return_arm_errors() {
    let body = sp(ast::Expr::Handle {
        expr: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(1)))),
        arms: vec![
            ast::HandleArm {
                kind: ast::HandleArmKind::Return,
                pattern: Some(sp(ast::Pattern::Bind("v".into()))),
                body: sp(ast::Expr::Resume(None)),
            },
        ],
    });
    let mut checker = Checker::new();
    checker.infer_expr(&body);
    assert!(
        checker.errors.iter().any(|e| e.message.contains("'resume'")),
        "resume in return arm must error; got {:?}",
        checker.errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
}

#[test]
fn verify_resume_can_be_called_twice_in_same_arm() {
    let plus = sp(ast::Expr::Binary {
        op: ast::BinaryOp::Add,
        left: Box::new(sp(ast::Expr::Resume(Some(Box::new(sp(ast::Expr::Literal(ast::Literal::Int(1)))))))),
        right: Box::new(sp(ast::Expr::Resume(Some(Box::new(sp(ast::Expr::Literal(ast::Literal::Int(2)))))))),
    });
    let body = sp(ast::Expr::Handle {
        expr: Box::new(sp(ast::Expr::Throw(Box::new(sp(ast::Expr::Literal(ast::Literal::Int(99))))))),
        arms: vec![
            ast::HandleArm {
                kind: ast::HandleArmKind::Return,
                pattern: Some(sp(ast::Pattern::Bind("v".into()))),
                body: sp(ast::Expr::Identifier("v".into())),
            },
            ast::HandleArm {
                kind: ast::HandleArmKind::Exn,
                pattern: Some(sp(ast::Pattern::Bind("e".into()))),
                body: plus,
            },
        ],
    });
    let fn_decl = ast::FnDecl {
        attrs: vec![],
        is_pub: false,
        name: "multi".into(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(ast::Type::Named("Int".into())),
        where_clause: vec![],
        body: ast::Block { stmts: vec![], ret: Some(Box::new(body)) },
    };
    let mut checker = Checker::new();
    checker.check_fn_decl(&fn_decl);
    assert!(
        !checker.errors.iter().any(|e| e.message.contains("'resume'") || e.message.contains("moved")),
        "two resume calls in one arm must type-check; got {:?}",
        checker.errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
}

#[test]
fn borrow_held_across_effect_op_is_allowed() {
    let mut checker = Checker::new();
    checker.register_effect("logger".into(), vec!["log".into()]);
    let op_ty = abrase::ty::Type::Function {
        params: vec![abrase::ty::Type::Int],
        effects: vec![],
        ret: Box::new(abrase::ty::Type::Unit),
    };
    checker.register_effect_op("logger::log".into(), op_ty);

    let body = sp(ast::Expr::Region {
        label: Some("inner".into()),
        body: ast::Block {
            stmts: vec![
                sp(ast::Stmt::Expr(sp(ast::Expr::Call {
                    callee: Box::new(sp(ast::Expr::FieldAccess {
                        base: Box::new(sp(ast::Expr::Identifier("logger".into()))),
                        field: "log".into(),
                    })),
                    args: vec![sp(ast::Expr::Literal(ast::Literal::Int(0)))],
                }))),
            ],
            ret: Some(Box::new(sp(ast::Expr::Literal(ast::Literal::Int(0))))),
        },
    });
    let fn_decl = ast::FnDecl {
        attrs: vec![],
        is_pub: false,
        name: "leaky".into(),
        generics: vec![],
        params: vec![],
        effects: vec![ast::EffectItem { name: vec!["logger".into()], arg: None }],
        return_type: Some(ast::Type::Named("Int".into())),
        where_clause: vec![],
        body: ast::Block {
            stmts: vec![
                sp(ast::Stmt::Let {
                    pattern: sp(ast::Pattern::Bind("v".into())),
                    is_mut: false,
                    ty: Some(ast::Type::Named("Int".into())),
                    value: sp(ast::Expr::Literal(ast::Literal::Int(100))),
                }),
                sp(ast::Stmt::Let {
                    pattern: sp(ast::Pattern::Bind("r".into())),
                    is_mut: false,
                    ty: Some(ast::Type::Reference {
                        is_mut: false,
                        inner: Box::new(ast::Type::Named("Int".into())),
                        region: None,
                    }),
                    value: sp(ast::Expr::Unary {
                        op: ast::UnaryOp::Ref,
                        right: Box::new(sp(ast::Expr::Identifier("v".into()))),
                    }),
                }),
            ],
            ret: Some(Box::new(body)),
        },
    };
    checker.check_fn_decl(&fn_decl);
    assert!(
        !checker.errors.iter().any(|e| e.message.contains("live across")),
        "a borrow live across an effect op must be allowed (it does not escape); got {:?}",
        checker.errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
}

#[test]
fn verify_borrow_barrier_allows_borrow_inside_arm_region() {
    let mut checker = Checker::new();
    checker.register_effect("logger".into(), vec!["log".into()]);
    let op_ty = abrase::ty::Type::Function {
        params: vec![abrase::ty::Type::Int],
        effects: vec![],
        ret: Box::new(abrase::ty::Type::Unit),
    };
    checker.register_effect_op("logger::log".into(), op_ty);

    let arm_body = sp(ast::Expr::Block(ast::Block {
        stmts: vec![
            sp(ast::Stmt::Let {
                pattern: sp(ast::Pattern::Bind("v".into())),
                is_mut: false,
                ty: Some(ast::Type::Named("Int".into())),
                value: sp(ast::Expr::Literal(ast::Literal::Int(1))),
            }),
            sp(ast::Stmt::Let {
                pattern: sp(ast::Pattern::Bind("r".into())),
                is_mut: false,
                ty: Some(ast::Type::Reference {
                    is_mut: false,
                    inner: Box::new(ast::Type::Named("Int".into())),
                    region: None,
                }),
                value: sp(ast::Expr::Unary {
                    op: ast::UnaryOp::Ref,
                    right: Box::new(sp(ast::Expr::Identifier("v".into()))),
                }),
            }),
            sp(ast::Stmt::Expr(sp(ast::Expr::Call {
                callee: Box::new(sp(ast::Expr::FieldAccess {
                    base: Box::new(sp(ast::Expr::Identifier("logger".into()))),
                    field: "log".into(),
                })),
                args: vec![sp(ast::Expr::Literal(ast::Literal::Int(0)))],
            }))),
        ],
        ret: Some(Box::new(sp(ast::Expr::Literal(ast::Literal::Int(0))))),
    }));

    let body = sp(ast::Expr::Handle {
        expr: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(0)))),
        arms: vec![
            ast::HandleArm {
                kind: ast::HandleArmKind::Return,
                pattern: Some(sp(ast::Pattern::Bind("v".into()))),
                body: sp(ast::Expr::Identifier("v".into())),
            },
            ast::HandleArm {
                kind: ast::HandleArmKind::Effect(vec!["logger".into(), "log".into()]),
                pattern: Some(sp(ast::Pattern::Bind("msg".into()))),
                body: arm_body,
            },
        ],
    });
    let fn_decl = ast::FnDecl {
        attrs: vec![],
        is_pub: false,
        name: "safe_inner".into(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(ast::Type::Named("Int".into())),
        where_clause: vec![],
        body: ast::Block { stmts: vec![], ret: Some(Box::new(body)) },
    };
    checker.check_fn_decl(&fn_decl);
    assert!(
        !checker.errors.iter().any(|e| e.message.contains("live across effect operation")),
        "borrow bound inside arm region must not trip the barrier; got {:?}",
        checker.errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
}

#[test]
fn verify_effect_op_call_requires_user_effect() {
    let mut checker = Checker::new();
    checker.register_effect("logger".into(), vec!["log".into()]);
    let op_ty = abrase::ty::Type::Function {
        params: vec![abrase::ty::Type::Int],
        effects: vec![],
        ret: Box::new(abrase::ty::Type::Unit),
    };
    checker.register_effect_op("logger::log".into(), op_ty);

    let body = sp(ast::Expr::Call {
        callee: Box::new(sp(ast::Expr::FieldAccess {
            base: Box::new(sp(ast::Expr::Identifier("logger".into()))),
            field: "log".into(),
        })),
        args: vec![sp(ast::Expr::Literal(ast::Literal::Int(7)))],
    });
    let fn_decl = ast::FnDecl {
        attrs: vec![],
        is_pub: false,
        name: "needs_eff".into(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(ast::Type::Named("Unit".into())),
        where_clause: vec![],
        body: ast::Block { stmts: vec![], ret: Some(Box::new(body)) },
    };
    checker.check_fn_decl(&fn_decl);
    assert!(
        checker.errors.iter().any(|e| e.message.contains("does not declare")),
        "calling logger.log without declaring <logger> must leak; got {:?}",
        checker.errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
}

#[test]
fn verify_effect_op_arg_type_mismatch_errors() {
    let mut checker = Checker::new();
    checker.register_effect("logger".into(), vec!["log".into()]);
    let op_ty = abrase::ty::Type::Function {
        params: vec![abrase::ty::Type::Int],
        effects: vec![],
        ret: Box::new(abrase::ty::Type::Unit),
    };
    checker.register_effect_op("logger::log".into(), op_ty);

    let call = sp(ast::Expr::Call {
        callee: Box::new(sp(ast::Expr::FieldAccess {
            base: Box::new(sp(ast::Expr::Identifier("logger".into()))),
            field: "log".into(),
        })),
        args: vec![sp(ast::Expr::Literal(ast::Literal::String("nope".into())))],
    });
    let fn_decl = ast::FnDecl {
        attrs: vec![],
        is_pub: false,
        name: "bad_arg".into(),
        generics: vec![],
        params: vec![],
        effects: vec![ast::EffectItem { name: vec!["logger".into()], arg: None }],
        return_type: Some(ast::Type::Named("Unit".into())),
        where_clause: vec![],
        body: ast::Block { stmts: vec![], ret: Some(Box::new(call)) },
    };
    checker.check_fn_decl(&fn_decl);
    assert!(
        checker.errors.iter().any(|e| e.message.contains("type mismatch")),
        "expected arg type mismatch for logger.log(\"nope\"); got {:?}",
        checker.errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
}

#[test]
fn verify_effect_op_call_declared_no_leak() {
    let mut checker = Checker::new();
    checker.register_effect("logger".into(), vec!["log".into()]);
    let op_ty = abrase::ty::Type::Function {
        params: vec![abrase::ty::Type::Int],
        effects: vec![],
        ret: Box::new(abrase::ty::Type::Unit),
    };
    checker.register_effect_op("logger::log".into(), op_ty);

    let body = sp(ast::Expr::Call {
        callee: Box::new(sp(ast::Expr::FieldAccess {
            base: Box::new(sp(ast::Expr::Identifier("logger".into()))),
            field: "log".into(),
        })),
        args: vec![sp(ast::Expr::Literal(ast::Literal::Int(0)))],
    });
    let fn_decl = ast::FnDecl {
        attrs: vec![],
        is_pub: false,
        name: "declares_logger".into(),
        generics: vec![],
        params: vec![],
        effects: vec![ast::EffectItem { name: vec!["logger".into()], arg: None }],
        return_type: Some(ast::Type::Named("Unit".into())),
        where_clause: vec![],
        body: ast::Block { stmts: vec![], ret: Some(Box::new(body)) },
    };
    checker.check_fn_decl(&fn_decl);
    assert!(
        !checker.errors.iter().any(|e| e.message.contains("does not declare")),
        "declared <logger> must accept logger.log call; got {:?}",
        checker.errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
}

#[test]
fn verify_handle_consumes_user_effect() {
    let mut checker = Checker::new();
    checker.register_effect("logger".into(), vec!["log".into()]);
    let op_ty = abrase::ty::Type::Function {
        params: vec![abrase::ty::Type::Int],
        effects: vec![],
        ret: Box::new(abrase::ty::Type::Unit),
    };
    checker.register_effect_op("logger::log".into(), op_ty);

    let inner_call = sp(ast::Expr::Call {
        callee: Box::new(sp(ast::Expr::FieldAccess {
            base: Box::new(sp(ast::Expr::Identifier("logger".into()))),
            field: "log".into(),
        })),
        args: vec![sp(ast::Expr::Literal(ast::Literal::Int(0)))],
    });
    let handle = sp(ast::Expr::Handle {
        expr: Box::new(inner_call),
        arms: vec![
            ast::HandleArm {
                kind: ast::HandleArmKind::Return,
                pattern: Some(sp(ast::Pattern::Bind("v".into()))),
                body: sp(ast::Expr::Literal(ast::Literal::Int(0))),
            },
            ast::HandleArm {
                kind: ast::HandleArmKind::Effect(vec!["logger".into(), "log".into()]),
                pattern: Some(sp(ast::Pattern::Bind("msg".into()))),
                body: sp(ast::Expr::Literal(ast::Literal::Int(1))),
            },
        ],
    });
    let fn_decl = ast::FnDecl {
        attrs: vec![],
        is_pub: false,
        name: "handles_logger".into(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(ast::Type::Named("Int".into())),
        where_clause: vec![],
        body: ast::Block { stmts: vec![], ret: Some(Box::new(handle)) },
    };
    checker.check_fn_decl(&fn_decl);
    assert!(
        !checker.errors.iter().any(|e| e.message.contains("does not declare")),
        "handle must consume user effect; got {:?}",
        checker.errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
}

#[test]
fn verify_handle_arm_types_unify_never_with_int() {
    let mut checker = Checker::new();
    let handle = sp(ast::Expr::Handle {
        expr: Box::new(sp(ast::Expr::Throw(Box::new(sp(ast::Expr::Literal(ast::Literal::Int(99))))))),
        arms: vec![
            ast::HandleArm {
                kind: ast::HandleArmKind::Return,
                pattern: Some(sp(ast::Pattern::Bind("v".into()))),
                body: sp(ast::Expr::Literal(ast::Literal::Int(7))),
            },
            ast::HandleArm {
                kind: ast::HandleArmKind::Exn,
                pattern: Some(sp(ast::Pattern::Bind("e".into()))),
                body: sp(ast::Expr::Resume(Some(Box::new(sp(ast::Expr::Literal(ast::Literal::Int(0))))))),
            },
        ],
    });
    let ty = checker.infer_expr(&handle);
    assert_eq!(ty, Type::Int, "handle return type must unify Never-arm with Int-arm; got {:?}", ty);
    assert!(
        !checker.errors.iter().any(|e| e.message.contains("arm types do not match")),
        "Never must not conflict with Int in handle arms; got {:?}",
        checker.errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
}

#[test]
fn verify_nested_handle_inner_consumes_outer_propagates() {
    let mut checker = Checker::new();
    checker.register_effect("a".into(), vec!["op".into()]);
    checker.register_effect("b".into(), vec!["op".into()]);
    let op_ty = abrase::ty::Type::Function {
        params: vec![],
        effects: vec![],
        ret: Box::new(abrase::ty::Type::Unit),
    };
    checker.register_effect_op("a::op".into(), op_ty.clone());
    checker.register_effect_op("b::op".into(), op_ty);

    let outer_inner = sp(ast::Expr::Block(ast::Block {
        stmts: vec![
            sp(ast::Stmt::Expr(sp(ast::Expr::Call {
                callee: Box::new(sp(ast::Expr::FieldAccess {
                    base: Box::new(sp(ast::Expr::Identifier("a".into()))),
                    field: "op".into(),
                })),
                args: vec![],
            }))),
            sp(ast::Stmt::Expr(sp(ast::Expr::Call {
                callee: Box::new(sp(ast::Expr::FieldAccess {
                    base: Box::new(sp(ast::Expr::Identifier("b".into()))),
                    field: "op".into(),
                })),
                args: vec![],
            }))),
        ],
        ret: Some(Box::new(sp(ast::Expr::Literal(ast::Literal::Int(0))))),
    }));
    let inner_handle = sp(ast::Expr::Handle {
        expr: Box::new(outer_inner),
        arms: vec![
            ast::HandleArm {
                kind: ast::HandleArmKind::Return,
                pattern: Some(sp(ast::Pattern::Bind("v".into()))),
                body: sp(ast::Expr::Identifier("v".into())),
            },
            ast::HandleArm {
                kind: ast::HandleArmKind::Effect(vec!["a".into(), "op".into()]),
                pattern: None,
                body: sp(ast::Expr::Literal(ast::Literal::Int(0))),
            },
        ],
    });
    let fn_decl = ast::FnDecl {
        attrs: vec![],
        is_pub: false,
        name: "only_handles_a".into(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(ast::Type::Named("Int".into())),
        where_clause: vec![],
        body: ast::Block { stmts: vec![], ret: Some(Box::new(inner_handle)) },
    };
    checker.check_fn_decl(&fn_decl);
    assert!(
        checker.errors.iter().any(|e| e.message.contains("does not declare")),
        "handling only 'a' must leave 'b' as a leaked effect; got {:?}",
        checker.errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
}

#[test]
fn verify_borrow_barrier_silent_for_pure_call() {
    let mut checker = Checker::new();
    let fn_ty = abrase::ty::Type::Function {
        params: vec![],
        effects: vec![],
        ret: Box::new(abrase::ty::Type::Int),
    };
    checker.insert_var("pure_call".into(), fn_ty, false, d_span());

    let body = ast::Block {
        stmts: vec![
            sp(ast::Stmt::Let {
                pattern: sp(ast::Pattern::Bind("v".into())),
                is_mut: false,
                ty: Some(ast::Type::Named("Int".into())),
                value: sp(ast::Expr::Literal(ast::Literal::Int(1))),
            }),
            sp(ast::Stmt::Let {
                pattern: sp(ast::Pattern::Bind("r".into())),
                is_mut: false,
                ty: Some(ast::Type::Reference {
                    is_mut: false,
                    inner: Box::new(ast::Type::Named("Int".into())),
                    region: None,
                }),
                value: sp(ast::Expr::Unary {
                    op: ast::UnaryOp::Ref,
                    right: Box::new(sp(ast::Expr::Identifier("v".into()))),
                }),
            }),
        ],
        ret: Some(Box::new(sp(ast::Expr::Call {
            callee: Box::new(sp(ast::Expr::Identifier("pure_call".into()))),
            args: vec![],
        }))),
    };
    let fn_decl = ast::FnDecl {
        attrs: vec![],
        is_pub: false,
        name: "pure_caller".into(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(ast::Type::Named("Int".into())),
        where_clause: vec![],
        body,
    };
    checker.check_fn_decl(&fn_decl);
    assert!(
        !checker.errors.iter().any(|e| e.message.contains("live across effect operation")),
        "pure call must not trip the borrow barrier; got {:?}",
        checker.errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
}

#[test]
fn mut_borrow_held_across_effect_op_is_allowed() {
    let mut checker = Checker::new();
    checker.register_effect("logger".into(), vec!["log".into()]);
    let op_ty = abrase::ty::Type::Function {
        params: vec![abrase::ty::Type::Int],
        effects: vec![],
        ret: Box::new(abrase::ty::Type::Unit),
    };
    checker.register_effect_op("logger::log".into(), op_ty);

    let body = sp(ast::Expr::Region {
        label: Some("inner".into()),
        body: ast::Block {
            stmts: vec![
                sp(ast::Stmt::Expr(sp(ast::Expr::Call {
                    callee: Box::new(sp(ast::Expr::FieldAccess {
                        base: Box::new(sp(ast::Expr::Identifier("logger".into()))),
                        field: "log".into(),
                    })),
                    args: vec![sp(ast::Expr::Literal(ast::Literal::Int(0)))],
                }))),
            ],
            ret: Some(Box::new(sp(ast::Expr::Literal(ast::Literal::Int(0))))),
        },
    });
    let fn_decl = ast::FnDecl {
        attrs: vec![],
        is_pub: false,
        name: "leaky_mut".into(),
        generics: vec![],
        params: vec![],
        effects: vec![ast::EffectItem { name: vec!["logger".into()], arg: None }],
        return_type: Some(ast::Type::Named("Int".into())),
        where_clause: vec![],
        body: ast::Block {
            stmts: vec![
                sp(ast::Stmt::Let {
                    pattern: sp(ast::Pattern::Bind("v".into())),
                    is_mut: true,
                    ty: Some(ast::Type::Named("Int".into())),
                    value: sp(ast::Expr::Literal(ast::Literal::Int(100))),
                }),
                sp(ast::Stmt::Let {
                    pattern: sp(ast::Pattern::Bind("r".into())),
                    is_mut: false,
                    ty: Some(ast::Type::Reference {
                        is_mut: true,
                        inner: Box::new(ast::Type::Named("Int".into())),
                        region: None,
                    }),
                    value: sp(ast::Expr::Unary {
                        op: ast::UnaryOp::RefMut,
                        right: Box::new(sp(ast::Expr::Identifier("v".into()))),
                    }),
                }),
            ],
            ret: Some(Box::new(body)),
        },
    };
    checker.check_fn_decl(&fn_decl);
    assert!(
        !checker.errors.iter().any(|e| e.message.contains("live across")),
        "a `&mut` live across an effect op must be allowed (it does not escape); got {:?}",
        checker.errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
}

#[test]
fn verify_handle_arm_type_mismatch_errors() {
    let mut checker = Checker::new();
    checker.register_effect("logger".into(), vec!["log".into()]);
    let op_ty = abrase::ty::Type::Function {
        params: vec![abrase::ty::Type::Int],
        effects: vec![],
        ret: Box::new(abrase::ty::Type::Unit),
    };
    checker.register_effect_op("logger::log".into(), op_ty);

    let inner = sp(ast::Expr::Call {
        callee: Box::new(sp(ast::Expr::FieldAccess {
            base: Box::new(sp(ast::Expr::Identifier("logger".into()))),
            field: "log".into(),
        })),
        args: vec![sp(ast::Expr::Literal(ast::Literal::Int(0)))],
    });

    let body = sp(ast::Expr::Handle {
        expr: Box::new(inner),
        arms: vec![
            ast::HandleArm {
                kind: ast::HandleArmKind::Return,
                pattern: Some(sp(ast::Pattern::Bind("v".into()))),
                body: sp(ast::Expr::Literal(ast::Literal::Int(0))),
            },
            ast::HandleArm {
                kind: ast::HandleArmKind::Effect(vec!["logger".into(), "log".into()]),
                pattern: Some(sp(ast::Pattern::Bind("msg".into()))),
                body: sp(ast::Expr::Literal(ast::Literal::String("oops".into()))),
            },
        ],
    });
    let fn_decl = ast::FnDecl {
        attrs: vec![],
        is_pub: false,
        name: "mixed_arms".into(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(ast::Type::Named("Int".into())),
        where_clause: vec![],
        body: ast::Block { stmts: vec![], ret: Some(Box::new(body)) },
    };
    checker.check_fn_decl(&fn_decl);
    assert!(
        checker.errors.iter().any(|e| e.message.contains("Handle arm types do not match")),
        "expected arm-type-mismatch error; got {:?}",
        checker.errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
}

#[test]
fn verify_effect_op_wrong_arg_count_errors() {
    // logger.log expects one Int arg; the call site passes none.
    let mut checker = Checker::new();
    checker.register_effect("logger".into(), vec!["log".into()]);
    let op_ty = abrase::ty::Type::Function {
        params: vec![abrase::ty::Type::Int],
        effects: vec![],
        ret: Box::new(abrase::ty::Type::Unit),
    };
    checker.register_effect_op("logger::log".into(), op_ty);

    let call = sp(ast::Expr::Call {
        callee: Box::new(sp(ast::Expr::FieldAccess {
            base: Box::new(sp(ast::Expr::Identifier("logger".into()))),
            field: "log".into(),
        })),
        args: vec![],
    });
    let fn_decl = ast::FnDecl {
        attrs: vec![],
        is_pub: false,
        name: "bad_arity".into(),
        generics: vec![],
        params: vec![],
        effects: vec![ast::EffectItem { name: vec!["logger".into()], arg: None }],
        return_type: Some(ast::Type::Named("Unit".into())),
        where_clause: vec![],
        body: ast::Block { stmts: vec![], ret: Some(Box::new(call)) },
    };
    checker.check_fn_decl(&fn_decl);
    assert!(
        checker.errors.iter().any(|e|
            e.message.contains("expects 1 argument")
        ),
        "expected arg-count mismatch for logger.log() with no args; got {:?}",
        checker.errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
}

#[test]
fn verify_unhandled_effect_op_in_handler_body_propagates() {
    let mut checker = Checker::new();
    checker.register_effect("a".into(), vec!["one".into()]);
    checker.register_effect("b".into(), vec!["two".into()]);
    let a_one_ty = abrase::ty::Type::Function {
        params: vec![],
        effects: vec![],
        ret: Box::new(abrase::ty::Type::Int),
    };
    let b_two_ty = abrase::ty::Type::Function {
        params: vec![],
        effects: vec![],
        ret: Box::new(abrase::ty::Type::Int),
    };
    checker.register_effect_op("a::one".into(), a_one_ty);
    checker.register_effect_op("b::two".into(), b_two_ty);

    // Body inside `handle` performs both <a> and <b> ops.
    let inner = sp(ast::Expr::Binary {
        op: ast::BinaryOp::Add,
        left: Box::new(sp(ast::Expr::Call {
            callee: Box::new(sp(ast::Expr::FieldAccess {
                base: Box::new(sp(ast::Expr::Identifier("a".into()))),
                field: "one".into(),
            })),
            args: vec![],
        })),
        right: Box::new(sp(ast::Expr::Call {
            callee: Box::new(sp(ast::Expr::FieldAccess {
                base: Box::new(sp(ast::Expr::Identifier("b".into()))),
                field: "two".into(),
            })),
            args: vec![],
        })),
    });
    let handle = sp(ast::Expr::Handle {
        expr: Box::new(inner),
        arms: vec![
            ast::HandleArm {
                kind: ast::HandleArmKind::Return,
                pattern: Some(sp(ast::Pattern::Bind("v".into()))),
                body: sp(ast::Expr::Identifier("v".into())),
            },
            // Only handles a.one; b.two is intentionally NOT handled.
            ast::HandleArm {
                kind: ast::HandleArmKind::Effect(vec!["a".into(), "one".into()]),
                pattern: None,
                body: sp(ast::Expr::Literal(ast::Literal::Int(0))),
            },
        ],
    });
    // Caller declares only <a>, leaving <b> as an undeclared leak.
    let fn_decl = ast::FnDecl {
        attrs: vec![],
        is_pub: false,
        name: "partial_handler".into(),
        generics: vec![],
        params: vec![],
        effects: vec![ast::EffectItem { name: vec!["a".into()], arg: None }],
        return_type: Some(ast::Type::Named("Int".into())),
        where_clause: vec![],
        body: ast::Block { stmts: vec![], ret: Some(Box::new(handle)) },
    };
    checker.check_fn_decl(&fn_decl);
    assert!(
        checker.errors.iter().any(|e| e.message.contains("does not declare")),
        "expected leak diagnostic for unhandled <b> escaping the handle; got {:?}",
        checker.errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
}
