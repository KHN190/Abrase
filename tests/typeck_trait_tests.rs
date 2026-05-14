use ect::ty::Type;
use ect::typeck::Checker;

// Generics & Trait Constraints Tests

#[test]
fn verify_register_trait() {
    let mut checker = Checker::new();

    let methods = vec!["display".into(), "debug".into()];
    checker.register_trait("Show".into(), methods.clone());

    let trait_methods = checker.get_trait("Show");
    assert!(trait_methods.is_some());
    assert_eq!(trait_methods.unwrap(), methods);
}

#[test]
fn verify_register_impl() {
    let mut checker = Checker::new();

    checker.register_impl("Int".into(), "Show".into());
    assert!(checker.has_impl("Int", "Show"));
}

#[test]
fn verify_impl_not_registered() {
    let checker = Checker::new();

    assert!(!checker.has_impl("String", "Iterator"));
}

#[test]
fn verify_register_generic_params() {
    let mut checker = Checker::new();

    let params = vec!["T".into(), "U".into()];
    checker.register_generic_params("map".into(), params.clone());

    let registered = checker.get_generic_params("map");
    assert!(registered.is_some());
    assert_eq!(registered.unwrap(), params);
}

#[test]
fn verify_register_trait_bound() {
    let mut checker = Checker::new();

    checker.register_trait_bound("T".into(), "Show".into());
    checker.register_trait_bound("T".into(), "Debug".into());

    let bounds = checker.get_trait_bounds("T");
    assert!(bounds.is_some());
    assert_eq!(bounds.unwrap().len(), 2);
}

#[test]
fn verify_validate_where_clause_satisfied() {
    let mut checker = Checker::new();

    checker.register_trait_bound("T".into(), "Show".into());
    checker.register_impl("Int".into(), "Show".into());

    assert!(checker.validate_where_clause("T", "Int"));
}

#[test]
fn verify_validate_where_clause_unsatisfied() {
    let mut checker = Checker::new();

    checker.register_trait_bound("T".into(), "Show".into());

    assert!(!checker.validate_where_clause("T", "String"));
}

#[test]
fn verify_validate_where_clause_no_bounds() {
    let checker = Checker::new();

    assert!(checker.validate_where_clause("T", "Int"));
}

#[test]
fn verify_validate_generic_instance_correct_params() {
    let mut checker = Checker::new();

    checker.register_generic_params("Vec".into(), vec!["T".into()]);
    assert!(checker.validate_generic_instance("Vec", &[Type::Int]));
}

#[test]
fn verify_validate_generic_instance_wrong_param_count() {
    let mut checker = Checker::new();

    checker.register_generic_params("Vec".into(), vec!["T".into()]);
    assert!(!checker.validate_generic_instance("Vec", &[Type::Int, Type::Bool]));
}

#[test]
fn verify_validate_generic_instance_no_params() {
    let checker = Checker::new();

    assert!(checker.validate_generic_instance("Int", &[]));
}

#[test]
fn verify_check_all_trait_bounds_satisfied() {
    let mut checker = Checker::new();

    checker.register_generic_params("apply".into(), vec!["T".into()]);
    checker.register_trait_bound("T".into(), "Show".into());
    checker.register_impl("Int".into(), "Show".into());

    let type_args = vec![("T".into(), Type::Int)];
    assert!(checker.check_all_trait_bounds("apply", &type_args));
}

#[test]
fn verify_check_all_trait_bounds_unsatisfied() {
    let mut checker = Checker::new();

    checker.register_generic_params("apply".into(), vec!["T".into()]);
    checker.register_trait_bound("T".into(), "Show".into());

    let type_args = vec![("T".into(), Type::String)];
    assert!(!checker.check_all_trait_bounds("apply", &type_args));
}

#[test]
fn verify_check_all_trait_bounds_multiple_constraints() {
    let mut checker = Checker::new();

    checker.register_generic_params("apply".into(), vec!["T".into()]);
    checker.register_trait_bound("T".into(), "Show".into());
    checker.register_trait_bound("T".into(), "Debug".into());
    checker.register_impl("Int".into(), "Show".into());
    checker.register_impl("Int".into(), "Debug".into());

    let type_args = vec![("T".into(), Type::Int)];
    assert!(checker.check_all_trait_bounds("apply", &type_args));
}

#[test]
fn verify_multiple_generic_params() {
    let mut checker = Checker::new();

    checker.register_generic_params("map".into(), vec!["T".into(), "U".into()]);
    checker.register_trait_bound("T".into(), "Show".into());
    checker.register_trait_bound("U".into(), "Debug".into());
    checker.register_impl("Int".into(), "Show".into());
    checker.register_impl("Bool".into(), "Debug".into());

    let type_args = vec![
        ("T".into(), Type::Int),
        ("U".into(), Type::Bool),
    ];
    assert!(checker.check_all_trait_bounds("map", &type_args));
}

#[test]
fn verify_trait_not_registered() {
    let checker = Checker::new();

    assert!(checker.get_trait("UnknownTrait").is_none());
}

#[test]
fn verify_generic_params_not_registered() {
    let checker = Checker::new();

    assert!(checker.get_generic_params("unknown_fn").is_none());
}

#[test]
fn verify_trait_bounds_multiple_traits() {
    let mut checker = Checker::new();

    checker.register_trait_bound("T".into(), "Show".into());
    checker.register_trait_bound("T".into(), "Eq".into());
    checker.register_trait_bound("T".into(), "Ord".into());

    let bounds = checker.get_trait_bounds("T");
    assert_eq!(bounds.unwrap().len(), 3);
}

#[test]
fn verify_impl_registry_multiple_impls() {
    let mut checker = Checker::new();

    checker.register_impl("Int".into(), "Show".into());
    checker.register_impl("Int".into(), "Debug".into());
    checker.register_impl("String".into(), "Show".into());

    assert!(checker.has_impl("Int", "Show"));
    assert!(checker.has_impl("Int", "Debug"));
    assert!(checker.has_impl("String", "Show"));
    assert!(!checker.has_impl("String", "Debug"));
}

#[test]
fn verify_validate_where_clause_multiple_bounds_all_satisfied() {
    let mut checker = Checker::new();

    checker.register_trait_bound("T".into(), "Show".into());
    checker.register_trait_bound("T".into(), "Debug".into());
    checker.register_impl("Bool".into(), "Show".into());
    checker.register_impl("Bool".into(), "Debug".into());

    assert!(checker.validate_where_clause("T", "Bool"));
}

#[test]
fn verify_validate_where_clause_multiple_bounds_partially_satisfied() {
    let mut checker = Checker::new();

    checker.register_trait_bound("T".into(), "Show".into());
    checker.register_trait_bound("T".into(), "Debug".into());
    checker.register_impl("Bool".into(), "Show".into());

    assert!(!checker.validate_where_clause("T", "Bool"));
}