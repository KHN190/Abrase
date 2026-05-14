use ect::ty::{Type, Variance};
use ect::typeck::Checker;

fn d_span() -> ect::ast::Span {
    ect::ast::Span {
        line: 0,
        col: 0,
    }
}

// Generic Type Variance Tests

// Registration tests
#[test]
fn verify_register_type_variance() {
    let mut checker = Checker::new();
    checker.register_type_variance("MyList".into(), vec![Variance::Covariant]);
    let result = checker.get_type_variance("MyList");
    assert!(result.is_some());
    assert_eq!(result.unwrap()[0], Variance::Covariant);
}

#[test]
fn verify_get_type_variance_builtin_list() {
    let checker = Checker::new();
    let result = checker.get_type_variance("List");
    assert!(result.is_some());
    assert_eq!(result.unwrap()[0], Variance::Covariant);
}

#[test]
fn verify_get_type_variance_builtin_fn() {
    let checker = Checker::new();
    let result = checker.get_type_variance("Fn");
    assert!(result.is_some());
    assert_eq!(result.unwrap().len(), 2);
    assert_eq!(result.unwrap()[0], Variance::Contravariant);
    assert_eq!(result.unwrap()[1], Variance::Covariant);
}

// Named subtype registration tests
#[test]
fn verify_register_named_subtype() {
    let mut checker = Checker::new();
    checker.register_named_subtype("String".into(), "Any".into());
    assert!(checker.is_subtype(&Type::Named("String".into()), &Type::Named("Any".into())));
}

#[test]
fn verify_is_named_subtype_transitive() {
    let mut checker = Checker::new();
    checker.register_named_subtype("Int".into(), "Number".into());
    checker.register_named_subtype("Number".into(), "Any".into());
    assert!(checker.is_subtype(&Type::Named("Int".into()), &Type::Named("Any".into())));
}

// Covariant tests
#[test]
fn verify_list_string_subtype_of_list_any() {
    let mut checker = Checker::new();
    checker.register_named_subtype("String".into(), "Any".into());
    assert!(checker.is_subtype(&Type::Named("List<String>".into()), &Type::Named("List<Any>".into())));
}

#[test]
fn verify_option_int_subtype_of_option_number() {
    let mut checker = Checker::new();
    checker.register_named_subtype("Int".into(), "Number".into());
    assert!(checker.is_subtype(&Type::Named("Option<Int>".into()), &Type::Named("Option<Number>".into())));
}

#[test]
fn verify_result_both_args_covariant() {
    let mut checker = Checker::new();
    checker.register_named_subtype("String".into(), "Any".into());
    checker.register_named_subtype("IOError".into(), "Error".into());
    assert!(checker.is_subtype(&Type::Named("Result<String, IOError>".into()), &Type::Named("Result<Any, Error>".into())));
}

#[test]
fn verify_covariant_different_inners_fail() {
    let checker = Checker::new();
    // Without Int being a subtype of String, List<Int> should not be subtype of List<String>
    assert!(!checker.is_subtype(&Type::Named("List<Int>".into()), &Type::Named("List<String>".into())));
}

#[test]
fn verify_covariant_same_arg_trivially_subtype() {
    let checker = Checker::new();
    // Same argument makes it a trivial subtype via equality
    assert!(checker.is_subtype(&Type::Named("List<Int>".into()), &Type::Named("List<Int>".into())));
}

// Contravariant tests
#[test]
fn verify_fn_input_contravariant() {
    let mut checker = Checker::new();
    checker.register_named_subtype("String".into(), "Any".into());
    // Fn(Any, R) <: Fn(String, R) because input is contravariant
    // We need to construct actual Function types, not Named
    let fn_any_int = Type::Function {
        params: vec![Type::Named("Any".into())],
        effects: vec![],
        ret: Box::new(Type::Int),
    };
    let fn_string_int = Type::Function {
        params: vec![Type::Named("String".into())],
        effects: vec![],
        ret: Box::new(Type::Int),
    };
    assert!(checker.is_subtype(&fn_any_int, &fn_string_int));
}

#[test]
fn verify_fn_output_covariant() {
    let mut checker = Checker::new();
    checker.register_named_subtype("String".into(), "Any".into());
    // Fn(X, String) <: Fn(X, Any) because output is covariant
    let fn_int_string = Type::Function {
        params: vec![Type::Int],
        effects: vec![],
        ret: Box::new(Type::Named("String".into())),
    };
    let fn_int_any = Type::Function {
        params: vec![Type::Int],
        effects: vec![],
        ret: Box::new(Type::Named("Any".into())),
    };
    assert!(checker.is_subtype(&fn_int_string, &fn_int_any));
}

#[test]
fn verify_fn_combined_variance() {
    let mut checker = Checker::new();
    checker.register_named_subtype("String".into(), "Any".into());
    // Fn(Any, String) <: Fn(String, Any) - contra input, co output
    let fn_any_string = Type::Function {
        params: vec![Type::Named("Any".into())],
        effects: vec![],
        ret: Box::new(Type::Named("String".into())),
    };
    let fn_string_any = Type::Function {
        params: vec![Type::Named("String".into())],
        effects: vec![],
        ret: Box::new(Type::Named("Any".into())),
    };
    assert!(checker.is_subtype(&fn_any_string, &fn_string_any));
}

#[test]
fn verify_fn_wrong_direction_fails() {
    let mut checker = Checker::new();
    checker.register_named_subtype("String".into(), "Any".into());
    // Fn(String, R) should NOT be subtype of Fn(Any, R) - wrong direction
    let fn_string_int = Type::Function {
        params: vec![Type::Named("String".into())],
        effects: vec![],
        ret: Box::new(Type::Int),
    };
    let fn_any_int = Type::Function {
        params: vec![Type::Named("Any".into())],
        effects: vec![],
        ret: Box::new(Type::Int),
    };
    assert!(!checker.is_subtype(&fn_string_int, &fn_any_int));
}

// Invariant tests
#[test]
fn verify_cell_string_not_subtype_of_cell_any() {
    let mut checker = Checker::new();
    checker.register_named_subtype("String".into(), "Any".into());
    // Even with String <: Any, Cell<String> should NOT be subtype of Cell<Any> (invariant)
    assert!(!checker.is_subtype(&Type::Named("Cell<String>".into()), &Type::Named("Cell<Any>".into())));
}

#[test]
fn verify_cell_any_not_subtype_of_cell_string() {
    let mut checker = Checker::new();
    checker.register_named_subtype("String".into(), "Any".into());
    // Cell<Any> should NOT be subtype of Cell<String> (invariant)
    assert!(!checker.is_subtype(&Type::Named("Cell<Any>".into()), &Type::Named("Cell<String>".into())));
}

#[test]
fn verify_cell_exact_is_subtype_of_itself() {
    let checker = Checker::new();
    // Cell<String> IS subtype of Cell<String> via equality
    assert!(checker.is_subtype(&Type::Named("Cell<String>".into()), &Type::Named("Cell<String>".into())));
}

// Boundary and negative cases
#[test]
fn verify_different_outer_types_not_subtype() {
    let checker = Checker::new();
    // List<String> should NOT be subtype of Option<String> - different containers
    assert!(!checker.is_subtype(&Type::Named("List<String>".into()), &Type::Named("Option<String>".into())));
}

#[test]
fn verify_arg_count_mismatch_not_subtype() {
    let checker = Checker::new();
    // Result<Int, Err> and Option<Int> have different arg counts - not subtypes
    assert!(!checker.is_subtype(&Type::Named("Result<Int, Err>".into()), &Type::Named("Option<Int>".into())));
}

#[test]
fn verify_unknown_subtype_of_anything() {
    let checker = Checker::new();
    assert!(checker.is_subtype(&Type::Unknown, &Type::Named("List<String>".into())));
    assert!(checker.is_subtype(&Type::Unknown, &Type::Int));
}

#[test]
fn verify_anything_subtype_of_unknown() {
    let checker = Checker::new();
    assert!(checker.is_subtype(&Type::Named("List<String>".into()), &Type::Unknown));
    assert!(checker.is_subtype(&Type::Int, &Type::Unknown));
}

// Integration with is_assignable
#[test]
fn verify_is_assignable_uses_covariance() {
    let mut checker = Checker::new();
    checker.register_named_subtype("String".into(), "Any".into());
    let list_string = Type::Named("List<String>".into());
    let list_any = Type::Named("List<Any>".into());
    // Should be assignable due to covariance
    assert!(checker.is_assignable(&list_any, &list_string));
}

#[test]
fn verify_is_assignable_rejects_invariant_mismatch() {
    let mut checker = Checker::new();
    checker.register_named_subtype("String".into(), "Any".into());
    let cell_string = Type::Named("Cell<String>".into());
    let cell_any = Type::Named("Cell<Any>".into());
    // Should NOT be assignable - invariant blocks it
    assert!(!checker.is_assignable(&cell_any, &cell_string));
}

#[test]
fn verify_types_compatible_delegates_to_variance() {
    let mut checker = Checker::new();
    checker.register_named_subtype("String".into(), "Any".into());
    let list_string = Type::Named("List<String>".into());
    let list_any = Type::Named("List<Any>".into());
    // types_compatible should work via variance
    assert!(checker.types_compatible(&list_any, &list_string));
}

#[test]
fn verify_user_registered_type_variance() {
    let mut checker = Checker::new();
    // Register a custom type with specific variance
    checker.register_type_variance("Box".into(), vec![Variance::Covariant]);
    checker.register_named_subtype("String".into(), "Any".into());
    // Box<String> should be subtype of Box<Any> via registered covariance
    assert!(checker.is_subtype(&Type::Named("Box<String>".into()), &Type::Named("Box<Any>".into())));
}

// Nested generics tests
#[test]
fn verify_nested_covariant_list_of_option() {
    let checker = Checker::new();
    // List<Option<Int>> should be subtype of List<Option<Int>> via equality
    let nested1 = Type::Named("List<Option<Int>>".into());
    let nested2 = Type::Named("List<Option<Int>>".into());
    assert!(checker.is_subtype(&nested1, &nested2));
}

#[test]
fn verify_nested_invariant_stops_propagation() {
    let mut checker = Checker::new();
    checker.register_named_subtype("String".into(), "Any".into());
    // List<Cell<String>> should NOT be subtype of List<Cell<Any>>
    // because Cell is invariant and stops the covariance propagation
    let list_cell_string = Type::Named("List<Cell<String>>".into());
    let list_cell_any = Type::Named("List<Cell<Any>>".into());
    assert!(!checker.is_subtype(&list_cell_string, &list_cell_any));
}

// Tuple covariance
#[test]
fn verify_tuple_covariant_elements() {
    let mut checker = Checker::new();
    checker.register_named_subtype("String".into(), "Any".into());
    let tuple1 = Type::Tuple(vec![Type::Named("String".into()), Type::Int]);
    let tuple2 = Type::Tuple(vec![Type::Named("Any".into()), Type::Int]);
    assert!(checker.is_subtype(&tuple1, &tuple2));
}

#[test]
fn verify_tuple_length_mismatch_not_subtype() {
    let checker = Checker::new();
    let tuple1 = Type::Tuple(vec![Type::Int]);
    let tuple2 = Type::Tuple(vec![Type::Int, Type::Int]);
    assert!(!checker.is_subtype(&tuple1, &tuple2));
}

// Array covariance
#[test]
fn verify_array_covariant() {
    let mut checker = Checker::new();
    checker.register_named_subtype("String".into(), "Any".into());
    let array_string = Type::Named("Array<String>".into());
    let array_any = Type::Named("Array<Any>".into());
    assert!(checker.is_subtype(&array_string, &array_any));
}
