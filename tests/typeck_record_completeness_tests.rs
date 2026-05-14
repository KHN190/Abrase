use ect::ast::{self, RecordField, Span, Spanned, TypeBody};
use ect::ty::Type;
use ect::typeck::Checker;

fn d_span() -> Span {
    Span { line: 0, col: 0 }
}

fn sp<T>(node: T) -> Spanned<T> {
    Spanned { node, span: d_span() }
}

// Record Completeness Validation Tests

// Basic completeness tests with check_record_initialization
#[test]
fn verify_record_all_fields_present() {
    let mut checker = Checker::new();

    let field_types = vec![
        ("x".into(), Type::Int),
        ("y".into(), Type::Int),
    ];
    let provided_fields = vec!["x".into(), "y".into()];
    let provided_values = vec![
        ("x".into(), Type::Int),
        ("y".into(), Type::Int),
    ];

    let result = checker.check_record_initialization(
        "Point",
        &field_types,
        &provided_fields,
        &provided_values,
        d_span(),
    );

    assert!(result);
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_record_missing_required_field() {
    let mut checker = Checker::new();

    let field_types = vec![
        ("x".into(), Type::Int),
        ("y".into(), Type::Int),
    ];
    let provided_fields = vec!["x".into()];
    let provided_values = vec![
        ("x".into(), Type::Int),
    ];

    let result = checker.check_record_initialization(
        "Point",
        &field_types,
        &provided_fields,
        &provided_values,
        d_span(),
    );

    assert!(!result);
    assert!(checker.errors.len() > 0);
    assert!(checker.errors[0].message.contains("missing required field"));
}

#[test]
fn verify_record_missing_multiple_fields() {
    let mut checker = Checker::new();

    let field_types = vec![
        ("x".into(), Type::Int),
        ("y".into(), Type::Int),
        ("z".into(), Type::Int),
    ];
    let provided_fields = vec!["x".into()];
    let provided_values = vec![
        ("x".into(), Type::Int),
    ];

    let result = checker.check_record_initialization(
        "Point3D",
        &field_types,
        &provided_fields,
        &provided_values,
        d_span(),
    );

    assert!(!result);
    assert!(checker.errors.len() > 1);
    // Should have errors for missing y and z
}

#[test]
fn verify_record_all_fields_required() {
    let mut checker = Checker::new();

    // Even if only one field is defined, it MUST be provided
    let field_types = vec![
        ("id".into(), Type::Int),
    ];
    let provided_fields = vec![];
    let provided_values = vec![];

    let result = checker.check_record_initialization(
        "User",
        &field_types,
        &provided_fields,
        &provided_values,
        d_span(),
    );

    assert!(!result);
    assert!(checker.errors.len() > 0);
}

// Type mismatch tests
#[test]
fn verify_record_field_type_mismatch() {
    let mut checker = Checker::new();

    let field_types = vec![
        ("x".into(), Type::Int),
        ("y".into(), Type::Int),
    ];
    let provided_fields = vec!["x".into(), "y".into()];
    let provided_values = vec![
        ("x".into(), Type::Int),
        ("y".into(), Type::String),  // Type mismatch!
    ];

    let result = checker.check_record_initialization(
        "Point",
        &field_types,
        &provided_fields,
        &provided_values,
        d_span(),
    );

    assert!(!result);
    assert!(checker.errors.len() > 0);
    assert!(checker.errors[0].message.contains("type mismatch"));
}

#[test]
fn verify_record_multiple_type_mismatches() {
    let mut checker = Checker::new();

    let field_types = vec![
        ("x".into(), Type::Int),
        ("y".into(), Type::Int),
        ("name".into(), Type::String),
    ];
    let provided_fields = vec!["x".into(), "y".into(), "name".into()];
    let provided_values = vec![
        ("x".into(), Type::String),  // Wrong!
        ("y".into(), Type::Bool),     // Wrong!
        ("name".into(), Type::String),
    ];

    let result = checker.check_record_initialization(
        "Point",
        &field_types,
        &provided_fields,
        &provided_values,
        d_span(),
    );

    assert!(!result);
    assert!(checker.errors.len() > 1);
}

// Combined tests: missing fields AND type mismatches
#[test]
fn verify_record_missing_field_and_type_error() {
    let mut checker = Checker::new();

    let field_types = vec![
        ("x".into(), Type::Int),
        ("y".into(), Type::Int),
    ];
    let provided_fields = vec!["x".into()];
    let provided_values = vec![
        ("x".into(), Type::String),  // Type mismatch
    ];

    let result = checker.check_record_initialization(
        "Point",
        &field_types,
        &provided_fields,
        &provided_values,
        d_span(),
    );

    assert!(!result);
    // Should have errors for both missing y and wrong type for x
    assert!(checker.errors.len() > 1);
}

#[test]
fn verify_record_with_reference_fields() {
    let mut checker = Checker::new();

    let field_types = vec![
        ("data".into(), Type::Reference { is_mut: false, inner: Box::new(Type::Int) }),
        ("count".into(), Type::Int),
    ];
    let provided_fields = vec!["data".into(), "count".into()];
    let provided_values = vec![
        ("data".into(), Type::Reference { is_mut: false, inner: Box::new(Type::Int) }),
        ("count".into(), Type::Int),
    ];

    let result = checker.check_record_initialization(
        "Container",
        &field_types,
        &provided_fields,
        &provided_values,
        d_span(),
    );

    assert!(result);
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_record_reference_type_mismatch() {
    let mut checker = Checker::new();

    let field_types = vec![
        ("data".into(), Type::Reference { is_mut: false, inner: Box::new(Type::Int) }),
    ];
    let provided_fields = vec!["data".into()];
    let provided_values = vec![
        ("data".into(), Type::Reference { is_mut: true, inner: Box::new(Type::Int) }),  // Mutability mismatch
    ];

    let result = checker.check_record_initialization(
        "Container",
        &field_types,
        &provided_fields,
        &provided_values,
        d_span(),
    );

    assert!(!result);
    assert!(checker.errors.len() > 0);
}

// Tuple field types
#[test]
fn verify_record_with_tuple_fields() {
    let mut checker = Checker::new();

    let field_types = vec![
        ("pair".into(), Type::Tuple(vec![Type::Int, Type::String])),
    ];
    let provided_fields = vec!["pair".into()];
    let provided_values = vec![
        ("pair".into(), Type::Tuple(vec![Type::Int, Type::String])),
    ];

    let result = checker.check_record_initialization(
        "Pair",
        &field_types,
        &provided_fields,
        &provided_values,
        d_span(),
    );

    assert!(result);
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_record_tuple_field_mismatch() {
    let mut checker = Checker::new();

    let field_types = vec![
        ("pair".into(), Type::Tuple(vec![Type::Int, Type::String])),
    ];
    let provided_fields = vec!["pair".into()];
    let provided_values = vec![
        ("pair".into(), Type::Tuple(vec![Type::String, Type::String])),  // Wrong!
    ];

    let result = checker.check_record_initialization(
        "Pair",
        &field_types,
        &provided_fields,
        &provided_values,
        d_span(),
    );

    assert!(!result);
    assert!(checker.errors.len() > 0);
}

// Named type fields
#[test]
fn verify_record_with_named_type_fields() {
    let mut checker = Checker::new();

    let field_types = vec![
        ("user".into(), Type::Named("User".into())),
        ("id".into(), Type::Int),
    ];
    let provided_fields = vec!["user".into(), "id".into()];
    let provided_values = vec![
        ("user".into(), Type::Named("User".into())),
        ("id".into(), Type::Int),
    ];

    let result = checker.check_record_initialization(
        "Account",
        &field_types,
        &provided_fields,
        &provided_values,
        d_span(),
    );

    assert!(result);
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_record_named_type_field_missing() {
    let mut checker = Checker::new();

    let field_types = vec![
        ("user".into(), Type::Named("User".into())),
        ("id".into(), Type::Int),
    ];
    let provided_fields = vec!["id".into()];
    let provided_values = vec![
        ("id".into(), Type::Int),
    ];

    let result = checker.check_record_initialization(
        "Account",
        &field_types,
        &provided_fields,
        &provided_values,
        d_span(),
    );

    assert!(!result);
    assert!(checker.errors.len() > 0);
    assert!(checker.errors[0].message.contains("missing required field 'user'"));
}

// Empty record
#[test]
fn verify_empty_record_definition() {
    let mut checker = Checker::new();

    let field_types: Vec<(String, Type)> = vec![];
    let provided_fields: Vec<String> = vec![];
    let provided_values: Vec<(String, Type)> = vec![];

    let result = checker.check_record_initialization(
        "Empty",
        &field_types,
        &provided_fields,
        &provided_values,
        d_span(),
    );

    assert!(result);
    assert_eq!(checker.errors.len(), 0);
}

// Large record
#[test]
fn verify_large_record_all_fields_present() {
    let mut checker = Checker::new();

    let mut field_types = Vec::new();
    let mut provided_fields = Vec::new();
    let mut provided_values = Vec::new();

    for i in 0..10 {
        let name = format!("field_{}", i);
        field_types.push((name.clone(), Type::Int));
        provided_fields.push(name.clone());
        provided_values.push((name, Type::Int));
    }

    let result = checker.check_record_initialization(
        "Large",
        &field_types,
        &provided_fields,
        &provided_values,
        d_span(),
    );

    assert!(result);
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_large_record_missing_one_field() {
    let mut checker = Checker::new();

    let mut field_types = Vec::new();
    let mut provided_fields = Vec::new();
    let mut provided_values = Vec::new();

    for i in 0..10 {
        let name = format!("field_{}", i);
        field_types.push((name.clone(), Type::Int));

        // Skip field_5
        if i != 5 {
            provided_fields.push(name.clone());
            provided_values.push((name, Type::Int));
        }
    }

    let result = checker.check_record_initialization(
        "Large",
        &field_types,
        &provided_fields,
        &provided_values,
        d_span(),
    );

    assert!(!result);
    assert!(checker.errors.len() > 0);
    assert!(checker.errors[0].message.contains("field_5"));
}
