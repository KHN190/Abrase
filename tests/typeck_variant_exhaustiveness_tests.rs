use ect::ast::{VariantCase, TypeBody};
use ect::ty::Type;
use ect::typeck::Checker;

fn d_span() -> ect::ast::Span {
    ect::ast::Span {
        line: 0,
        col: 0,
    }
}

// Registry tests
#[test]
fn verify_register_variant_cases() {
    let mut checker = Checker::new();
    let cases = vec!["Some".into(), "None".into()];
    checker.register_variant_cases("Option".into(), cases);

    let result = checker.get_variant_cases("Option");
    assert!(result.is_some());
    assert_eq!(result.unwrap().len(), 2);
}

#[test]
fn verify_get_variant_cases() {
    let mut checker = Checker::new();
    checker.register_variant_cases("Result".into(), vec!["Ok".into(), "Err".into()]);

    let cases = checker.get_variant_cases("Result");
    assert!(cases.is_some());
    assert!(cases.unwrap().contains(&"Ok".into()));
    assert!(cases.unwrap().contains(&"Err".into()));
}

#[test]
fn verify_register_type_auto_populates_variant_registry() {
    let mut checker = Checker::new();

    let cases = vec![
        VariantCase::Unit("Some".into()),
        VariantCase::Unit("None".into()),
    ];
    checker.register_type("Option".into(), TypeBody::Variant(cases));

    let variant_cases = checker.get_variant_cases("Option");
    assert!(variant_cases.is_some());
    assert_eq!(variant_cases.unwrap().len(), 2);
}

// Exhaustive match tests
#[test]
fn verify_all_cases_covered_no_error() {
    let mut checker = Checker::new();
    checker.register_variant_cases("Result".into(), vec!["Ok".into(), "Err".into()]);

    let covered = vec!["Ok".into(), "Err".into()];
    let result = checker.check_variant_exhaustiveness("Result", &covered, false, d_span());

    assert!(result);
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_wildcard_covers_all() {
    let mut checker = Checker::new();
    checker.register_variant_cases("Option".into(), vec!["Some".into(), "None".into()]);

    let covered: Vec<String> = vec![];
    let result = checker.check_variant_exhaustiveness("Option", &covered, true, d_span());

    assert!(result);
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_wildcard_with_some_cases_also_ok() {
    let mut checker = Checker::new();
    checker.register_variant_cases("Option".into(), vec!["Some".into(), "None".into()]);

    let covered = vec!["Some".into()];
    let result = checker.check_variant_exhaustiveness("Option", &covered, true, d_span());

    assert!(result);
    assert_eq!(checker.errors.len(), 0);
}

// Non-exhaustive match tests
#[test]
fn verify_missing_one_case_errors() {
    let mut checker = Checker::new();
    checker.register_variant_cases("Result".into(), vec!["Ok".into(), "Err".into()]);

    let covered = vec!["Ok".into()];
    let result = checker.check_variant_exhaustiveness("Result", &covered, false, d_span());

    assert!(!result);
    assert!(checker.errors.len() > 0);
    assert!(checker.errors[0].message.contains("Err"));
}

#[test]
fn verify_missing_all_cases_errors() {
    let mut checker = Checker::new();
    checker.register_variant_cases("Option".into(), vec!["Some".into(), "None".into()]);

    let covered: Vec<String> = vec![];
    let result = checker.check_variant_exhaustiveness("Option", &covered, false, d_span());

    assert!(!result);
    assert!(checker.errors.len() > 1);
}

#[test]
fn verify_missing_none_case_option() {
    let mut checker = Checker::new();
    checker.register_variant_cases("Option".into(), vec!["Some".into(), "None".into()]);

    let covered = vec!["Some".into()];
    let result = checker.check_variant_exhaustiveness("Option", &covered, false, d_span());

    assert!(!result);
    assert!(checker.errors[0].message.contains("None"));
}

#[test]
fn verify_missing_some_case_option() {
    let mut checker = Checker::new();
    checker.register_variant_cases("Option".into(), vec!["Some".into(), "None".into()]);

    let covered = vec!["None".into()];
    let result = checker.check_variant_exhaustiveness("Option", &covered, false, d_span());

    assert!(!result);
    assert!(checker.errors[0].message.contains("Some"));
}

// Custom types
#[test]
fn verify_custom_three_case_enum_exhaustive() {
    let mut checker = Checker::new();
    let cases = vec!["Red".into(), "Green".into(), "Blue".into()];
    checker.register_variant_cases("Color".into(), cases);

    let covered = vec!["Red".into(), "Green".into(), "Blue".into()];
    let result = checker.check_variant_exhaustiveness("Color", &covered, false, d_span());

    assert!(result);
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_custom_three_case_enum_missing_one() {
    let mut checker = Checker::new();
    checker.register_variant_cases("Color".into(),
        vec!["Red".into(), "Green".into(), "Blue".into()]);

    let covered = vec!["Red".into(), "Green".into()];
    let result = checker.check_variant_exhaustiveness("Color", &covered, false, d_span());

    assert!(!result);
    assert!(checker.errors.len() > 0);
    assert!(checker.errors[0].message.contains("Blue"));
}

#[test]
fn verify_unit_variant_exhaustive() {
    let mut checker = Checker::new();
    let cases = vec![
        VariantCase::Unit("Yes".into()),
        VariantCase::Unit("No".into()),
    ];
    checker.register_type("Bool".into(), TypeBody::Variant(cases));

    let covered = vec!["Yes".into(), "No".into()];
    let result = checker.check_variant_exhaustiveness("Bool", &covered, false, d_span());

    assert!(result);
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_tuple_variant_exhaustive() {
    let mut checker = Checker::new();
    let cases = vec![
        VariantCase::Tuple("Ok".into(), vec![]),
        VariantCase::Tuple("Err".into(), vec![]),
    ];
    checker.register_type("Result".into(), TypeBody::Variant(cases));

    let covered = vec!["Ok".into(), "Err".into()];
    let result = checker.check_variant_exhaustiveness("Result", &covered, false, d_span());

    assert!(result);
}

// Duplicate/unreachable
#[test]
fn verify_duplicate_cases_still_exhaustive() {
    let mut checker = Checker::new();
    checker.register_variant_cases("Result".into(), vec!["Ok".into(), "Err".into()]);

    // Duplicate Ok but all cases present
    let covered = vec!["Ok".into(), "Ok".into(), "Err".into()];
    let result = checker.check_variant_exhaustiveness("Result", &covered, false, d_span());

    // Still exhaustive because all required cases present
    assert!(result);
}

// Or-patterns
#[test]
fn verify_or_pattern_covers_multiple_cases() {
    let mut checker = Checker::new();
    checker.register_variant_cases("Color".into(),
        vec!["Red".into(), "Green".into(), "Blue".into()]);

    // Simulating or-pattern which would cover Red | Green
    let covered = vec!["Red".into(), "Green".into(), "Blue".into()];
    let result = checker.check_variant_exhaustiveness("Color", &covered, false, d_span());

    assert!(result);
}

#[test]
fn verify_or_pattern_still_missing_case() {
    let mut checker = Checker::new();
    checker.register_variant_cases("Color".into(),
        vec!["Red".into(), "Green".into(), "Blue".into()]);

    // Simulating or-pattern covering Red | Green but missing Blue
    let covered = vec!["Red".into(), "Green".into()];
    let result = checker.check_variant_exhaustiveness("Color", &covered, false, d_span());

    assert!(!result);
    assert!(checker.errors[0].message.contains("Blue"));
}

// Unknown types
#[test]
fn verify_unknown_type_passes_exhaustiveness() {
    let mut checker = Checker::new();

    // Type not registered - should be lenient
    let covered = vec!["Ok".into()];
    let result = checker.check_variant_exhaustiveness("UnknownType", &covered, false, d_span());

    assert!(result);
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_non_variant_type_passes_exhaustiveness() {
    let mut checker = Checker::new();

    // Record type not variant type - should be lenient
    checker.register_type("Point".into(), TypeBody::Record(vec![]));

    let covered: Vec<String> = vec![];
    let result = checker.check_variant_exhaustiveness("Point", &covered, false, d_span());

    assert!(result);
    assert_eq!(checker.errors.len(), 0);
}

// Single case
#[test]
fn verify_single_case_variant_exhaustive() {
    let mut checker = Checker::new();
    checker.register_variant_cases("Unit".into(), vec!["Unit".into()]);

    let covered = vec!["Unit".into()];
    let result = checker.check_variant_exhaustiveness("Unit", &covered, false, d_span());

    assert!(result);
}

#[test]
fn verify_single_case_variant_missing_errors() {
    let mut checker = Checker::new();
    checker.register_variant_cases("Unit".into(), vec!["Unit".into()]);

    let covered: Vec<String> = vec![];
    let result = checker.check_variant_exhaustiveness("Unit", &covered, false, d_span());

    assert!(!result);
    assert!(checker.errors[0].message.contains("Unit"));
}
