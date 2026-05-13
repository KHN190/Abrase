use ect::ast::{self, Pattern, Span, Spanned};
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
    let checker = Checker::new();
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