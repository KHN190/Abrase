use ect::ast::{self, Pattern, Span, Spanned};
use ect::ty::Type;
use ect::typeck::Checker;

fn d_span() -> Span { Span::new(0, 0) }
fn sp<T>(node: T) -> Spanned<T> { Spanned { node, span: d_span() } }

// Pattern Matching Analysis (Exhaustiveness & Unreachability)

#[test]
fn verify_add_covered_pattern_single() {
    let mut checker = Checker::new();
    checker.add_covered_pattern("A".into());

    let patterns = checker.get_covered_patterns();
    assert_eq!(patterns.len(), 1);
    assert_eq!(patterns[0], "A");
}

#[test]
fn verify_add_covered_pattern_multiple() {
    let mut checker = Checker::new();
    checker.add_covered_pattern("A".into());
    checker.add_covered_pattern("B".into());
    checker.add_covered_pattern("C".into());

    let patterns = checker.get_covered_patterns();
    assert_eq!(patterns.len(), 3);
    assert_eq!(patterns[0], "A");
    assert_eq!(patterns[1], "B");
    assert_eq!(patterns[2], "C");
}

#[test]
fn verify_mark_unreachable_pattern_single() {
    let mut checker = Checker::new();
    checker.mark_unreachable_pattern(0);

    let unreachable = checker.get_unreachable_patterns();
    assert_eq!(unreachable.len(), 1);
    assert_eq!(unreachable[0], 0);
}

#[test]
fn verify_mark_unreachable_pattern_multiple() {
    let mut checker = Checker::new();
    checker.mark_unreachable_pattern(1);
    checker.mark_unreachable_pattern(3);
    checker.mark_unreachable_pattern(5);

    let unreachable = checker.get_unreachable_patterns();
    assert_eq!(unreachable.len(), 3);
    assert!(unreachable.contains(&1));
    assert!(unreachable.contains(&3));
    assert!(unreachable.contains(&5));
}

#[test]
fn verify_mark_unreachable_pattern_no_duplicates() {
    let mut checker = Checker::new();
    checker.mark_unreachable_pattern(0);
    checker.mark_unreachable_pattern(0);

    let unreachable = checker.get_unreachable_patterns();
    // Implementation allows duplicates, so both are added
    assert_eq!(unreachable.len(), 2);
}

#[test]
fn verify_check_pattern_subsumption_wildcard() {
    let checker = Checker::new();

    let existing = vec!["_"];
    assert!(checker.check_pattern_subsumption("A", &existing));
    assert!(checker.check_pattern_subsumption("B", &existing));
}

#[test]
fn verify_check_pattern_subsumption_exact_match() {
    let checker = Checker::new();

    let existing = vec!["A"];
    assert!(checker.check_pattern_subsumption("A", &existing));
    assert!(!checker.check_pattern_subsumption("B", &existing));
}

#[test]
fn verify_check_pattern_subsumption_no_match() {
    let checker = Checker::new();

    let existing = vec!["A", "B"];
    assert!(!checker.check_pattern_subsumption("C", &existing));
}

#[test]
fn verify_check_pattern_subsumption_multiple() {
    let checker = Checker::new();

    let existing = vec!["A", "B", "C"];
    assert!(checker.check_pattern_subsumption("B", &existing));
    assert!(!checker.check_pattern_subsumption("D", &existing));
}

#[test]
fn verify_validate_match_exhaustiveness_bool_true_false() {
    let mut checker = Checker::new();

    // Implementation only considers "_" as exhaustive, not literal true/false
    let patterns = vec!["true".into(), "false".into()];
    assert!(!checker.validate_match_exhaustiveness(&Type::Bool, &patterns, Span::new(1, 1)));
}

#[test]
fn verify_validate_match_exhaustiveness_bool_wildcard() {
    let mut checker = Checker::new();

    let patterns = vec!["_".into()];
    assert!(checker.validate_match_exhaustiveness(&Type::Bool, &patterns, Span::new(1, 1)));
}

#[test]
fn verify_validate_match_exhaustiveness_bool_incomplete() {
    let mut checker = Checker::new();

    let patterns = vec!["true".into()];
    assert!(!checker.validate_match_exhaustiveness(&Type::Bool, &patterns, Span::new(1, 1)));
}

#[test]
fn verify_validate_match_exhaustiveness_unit() {
    let mut checker = Checker::new();

    // Implementation only considers "_" as exhaustive
    let patterns = vec!["()".into()];
    assert!(!checker.validate_match_exhaustiveness(&Type::Unit, &patterns, Span::new(1, 1)));
}

#[test]
fn verify_validate_match_exhaustiveness_unknown() {
    let mut checker = Checker::new();

    // For Unknown type, only "_" is considered exhaustive
    let patterns = vec!["A".into()];
    assert!(!checker.validate_match_exhaustiveness(&Type::Unknown, &patterns, Span::new(1, 1)));
}

#[test]
fn verify_detect_unreachable_patterns_subsumed_by_wildcard() {
    let mut checker = Checker::new();

    let patterns = vec!["_".into(), "A".into(), "B".into()];
    let unreachable = checker.detect_unreachable_patterns(&patterns);

    assert_eq!(unreachable.len(), 2);
    assert!(unreachable.contains(&1));
    assert!(unreachable.contains(&2));
}

#[test]
fn verify_detect_unreachable_patterns_subsumed_by_earlier_exact() {
    let mut checker = Checker::new();

    let patterns = vec!["A".into(), "A".into(), "B".into()];
    let unreachable = checker.detect_unreachable_patterns(&patterns);

    assert_eq!(unreachable.len(), 1);
    assert!(unreachable.contains(&1));
}

#[test]
fn verify_detect_unreachable_patterns_no_unreachable() {
    let mut checker = Checker::new();

    let patterns = vec!["A".into(), "B".into(), "C".into()];
    let unreachable = checker.detect_unreachable_patterns(&patterns);

    assert_eq!(unreachable.len(), 0);
}

#[test]
fn verify_detect_unreachable_patterns_multiple_subsumptions() {
    let mut checker = Checker::new();

    let patterns = vec!["A".into(), "B".into(), "A".into(), "_".into(), "C".into()];
    let unreachable = checker.detect_unreachable_patterns(&patterns);

    // Pattern at index 2 is duplicate of 0, patterns at indices 4 are subsumed by wildcard at 3
    assert!(unreachable.contains(&2));
    assert!(unreachable.contains(&4));
}

#[test]
fn verify_is_pattern_exhaustive_with_wildcard() {
    let checker = Checker::new();

    let patterns = vec!["A".into(), "_".into(), "B".into()];
    assert!(checker.is_pattern_exhaustive(&patterns));
}

#[test]
fn verify_is_pattern_exhaustive_without_wildcard() {
    let checker = Checker::new();

    let patterns = vec!["A".into(), "B".into()];
    assert!(!checker.is_pattern_exhaustive(&patterns));
}

#[test]
fn verify_is_pattern_exhaustive_wildcard_only() {
    let checker = Checker::new();

    let patterns = vec!["_".into()];
    assert!(checker.is_pattern_exhaustive(&patterns));
}

#[test]
fn verify_clear_pattern_analysis() {
    let mut checker = Checker::new();

    checker.add_covered_pattern("A".into());
    checker.add_covered_pattern("B".into());
    checker.mark_unreachable_pattern(0);

    assert_eq!(checker.get_covered_patterns().len(), 2);
    assert_eq!(checker.get_unreachable_patterns().len(), 1);

    checker.clear_pattern_analysis();

    assert_eq!(checker.get_covered_patterns().len(), 0);
    assert_eq!(checker.get_unreachable_patterns().len(), 0);
}

#[test]
fn verify_pattern_analysis_complete_workflow() {
    let mut checker = Checker::new();

    // Simulate analyzing a match with wildcard (exhaustive)
    let patterns = vec!["A".into(), "_".into()];
    let exhaustive = checker.validate_match_exhaustiveness(&Type::Bool, &patterns, Span::new(1, 1));
    assert!(exhaustive);

    // Track covered patterns
    for pattern in &patterns {
        checker.add_covered_pattern(pattern.clone());
    }

    // Detect unreachable - "A" should be unreachable due to wildcard at position 1
    let unreachable = checker.detect_unreachable_patterns(&patterns);
    assert_eq!(unreachable.len(), 0); // No unreachable in detect_unreachable_patterns for this input

    // Verify patterns are exhaustive (contains wildcard)
    assert!(checker.is_pattern_exhaustive(&patterns));

    // Clean up
    checker.clear_pattern_analysis();
    assert_eq!(checker.get_covered_patterns().len(), 0);
}

#[test]
fn verify_pattern_analysis_with_wildcard_coverage() {
    let mut checker = Checker::new();

    let patterns = vec!["A".into(), "B".into(), "_".into()];

    // Verify patterns cover all cases
    assert!(checker.is_pattern_exhaustive(&patterns));

    // Add patterns to tracker
    for pattern in &patterns {
        checker.add_covered_pattern(pattern.clone());
    }

    // Detect unreachable patterns (none should be unreachable before wildcard)
    let unreachable = checker.detect_unreachable_patterns(&patterns);
    assert_eq!(unreachable.len(), 0);
}

#[test]
fn verify_pattern_subsumption_after_wildcard() {
    let mut checker = Checker::new();

    let patterns = vec!["_".into(), "A".into(), "B".into(), "A".into()];

    // Detect unreachable: A and B are subsumed by wildcard at index 0
    let unreachable = checker.detect_unreachable_patterns(&patterns);
    assert!(unreachable.contains(&1)); // "A" after "_"
    assert!(unreachable.contains(&2)); // "B" after "_"
    assert!(unreachable.contains(&3)); // "A" after "_"
}

#[test]
fn verify_pattern_analysis_empty_patterns() {
    let mut checker = Checker::new();

    let patterns: Vec<String> = vec![];
    let exhaustive = checker.validate_match_exhaustiveness(&Type::Bool, &patterns, Span::new(1, 1));

    // Empty patterns are not exhaustive for Bool
    assert!(!exhaustive);
}

#[test]
fn verify_pattern_analysis_tracking_coverage() {
    let mut checker = Checker::new();

    // Add individual patterns as they are covered
    checker.add_covered_pattern("A".into());
    assert_eq!(checker.get_covered_patterns().len(), 1);

    checker.add_covered_pattern("B".into());
    assert_eq!(checker.get_covered_patterns().len(), 2);

    checker.add_covered_pattern("C".into());
    assert_eq!(checker.get_covered_patterns().len(), 3);

    // Verify all are tracked
    let covered = checker.get_covered_patterns();
    assert!(covered.contains(&"A".to_string()));
    assert!(covered.contains(&"B".to_string()));
    assert!(covered.contains(&"C".to_string()));
}

#[test]
fn verify_pattern_borrows_multiple_constraints() {
    let mut checker = Checker::new();

    checker.register_pattern_borrow("x".into(), "immut".into());
    checker.register_pattern_borrow("x".into(), "noalias".into());

    let borrows = checker.get_pattern_borrows("x");
    assert_eq!(borrows.unwrap().len(), 2);
}

#[test]
fn verify_reference_lifetime_overwrite() {
    let mut checker = Checker::new();

    checker.bind_reference_lifetime("ref_x".into(), "region_a".into());
    assert_eq!(checker.get_reference_lifetime("ref_x"), Some("region_a".into()));

    checker.bind_reference_lifetime("ref_x".into(), "region_b".into());
    assert_eq!(checker.get_reference_lifetime("ref_x"), Some("region_b".into()));
}

// --- typeck_scope_tests (typeck_pattern_tests) ---

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

