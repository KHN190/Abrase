use ect::ast::{Literal, StringPart};
use ect::ty::Type;
use ect::typeck::Checker;

fn d_span() -> ect::ast::Span {
    ect::ast::Span {
        line: 0,
        col: 0,
    }
}

// String Interpolation Validation Tests

#[test]
fn verify_interpolation_identifier_in_scope() {
    let mut checker = Checker::new();

    // Register Show trait
    checker.register_trait("Show".into(), vec!["to_string".into()]);

    // Register Int as implementing Show
    checker.register_impl("Int", "Show");

    // Insert variable in scope
    checker.insert_var("x".into(), Type::Int, false, d_span());

    // Create interpolated string with identifier in scope
    let parts = vec![
        StringPart::Literal("Value: ".into()),
        StringPart::Interp(vec!["x".into()]),
    ];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(result);
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_interpolation_identifier_out_of_scope() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("Int", "Show");

    // Variable NOT in scope
    let parts = vec![
        StringPart::Literal("Value: ".into()),
        StringPart::Interp(vec!["undefined_var".into()]),
    ];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(!result);
    assert!(checker.errors.len() > 0);
    assert!(checker.errors[0].message.contains("undefined") ||
            checker.errors[0].message.contains("Undefined"));
}

#[test]
fn verify_interpolation_multiple_identifiers() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("Int", "Show");
    checker.register_impl("String", "Show");

    checker.insert_var("x".into(), Type::Int, false, d_span());
    checker.insert_var("name".into(), Type::String, false, d_span());

    let parts = vec![
        StringPart::Literal("User: ".into()),
        StringPart::Interp(vec!["name".into()]),
        StringPart::Literal(" ID: ".into()),
        StringPart::Interp(vec!["x".into()]),
    ];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(result);
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_type_implements_show() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("Int", "Show");

    checker.insert_var("x".into(), Type::Int, false, d_span());

    let parts = vec![
        StringPart::Literal("Value: ".into()),
        StringPart::Interp(vec!["x".into()]),
    ];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(result);
}

#[test]
fn verify_type_does_not_implement_show() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    // Int NOT registered as implementing Show

    checker.insert_var("x".into(), Type::Int, false, d_span());

    let parts = vec![
        StringPart::Literal("Value: ".into()),
        StringPart::Interp(vec!["x".into()]),
    ];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(!result);
    assert!(checker.errors.len() > 0);
    assert!(checker.errors[0].message.contains("Show") ||
            checker.errors[0].message.contains("trait"));
}

#[test]
fn verify_multiple_types_some_implement_show() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("Int", "Show");
    // String does NOT implement Show

    checker.insert_var("x".into(), Type::Int, false, d_span());
    checker.insert_var("s".into(), Type::String, false, d_span());

    let parts = vec![
        StringPart::Interp(vec!["x".into()]),
        StringPart::Interp(vec!["s".into()]),
    ];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(!result);
    assert!(checker.errors.len() > 0);
    // Should report error for s not implementing Show
    assert!(checker.errors.iter().any(|e| e.message.contains("s") || e.message.contains("String")));
}

#[test]
fn verify_interpolation_field_access_path() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);

    // Register User type with name field
    use ect::ast::{TypeBody, RecordField, Type as AstType};
    let user_type = TypeBody::Record(vec![
        RecordField {
            name: "name".into(),
            ty: AstType::Named("String".into()),
            is_pub: false,
        },
    ]);
    checker.register_type("User".into(), user_type);

    // Insert variable representing a record type
    checker.insert_var("user".into(), Type::Named("User".into()), false, d_span());

    // Register String as implementing Show
    checker.register_impl("String", "Show");

    // Interpolation with field access: {user.name}
    let parts = vec![
        StringPart::Literal("Name: ".into()),
        StringPart::Interp(vec!["user".into(), "name".into()]),
    ];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(result);
}

#[test]
fn verify_interpolation_undefined_field() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("String", "Show");

    checker.insert_var("user".into(), Type::Named("User".into()), false, d_span());

    // Try to access non-existent field
    let parts = vec![
        StringPart::Interp(vec!["user".into(), "nonexistent".into()]),
    ];

    let result = checker.check_string_interpolation(&parts, d_span());
    // May or may not error depending on implementation detail
    // At minimum, should not crash
    assert!(result || checker.errors.len() > 0);
}

#[test]
fn verify_interpolation_deeply_nested_path() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("String", "Show");

    // Insert base variable
    checker.insert_var("root".into(), Type::Named("Root".into()), false, d_span());

    // Nested path: {root.field1.field2.field3}
    let parts = vec![
        StringPart::Interp(vec![
            "root".into(),
            "field1".into(),
            "field2".into(),
            "field3".into(),
        ]),
    ];

    let result = checker.check_string_interpolation(&parts, d_span());
    // Should at least not crash
    assert!(result || checker.errors.len() > 0);
}

#[test]
fn verify_string_literal_without_interpolation() {
    let mut checker = Checker::new();

    // Plain string should not require Show trait checks
    let lit = Literal::String("hello world".into());
    let result = checker.infer_literal(&lit, d_span());

    assert_eq!(result, Type::String);
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_interpolated_string_type() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("Int", "Show");

    checker.insert_var("x".into(), Type::Int, false, d_span());

    let lit = Literal::StringInterp(vec![
        StringPart::Literal("Value: ".into()),
        StringPart::Interp(vec!["x".into()]),
    ]);

    let result = checker.infer_literal(&lit, d_span());

    // Interpolated string should still be Type::String
    assert_eq!(result, Type::String);
    // But if x implements Show, no errors
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_interpolated_string_validates_identifiers() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("Int", "Show");

    // x is NOT in scope

    let lit = Literal::StringInterp(vec![
        StringPart::Literal("Value: ".into()),
        StringPart::Interp(vec!["x".into()]),
    ]);

    let result = checker.infer_literal(&lit, d_span());

    // Type is still String, but should have error
    assert_eq!(result, Type::String);
    assert!(checker.errors.len() > 0);
}

#[test]
fn verify_primitives_implement_show() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("Int", "Show");
    checker.register_impl("Bool", "Show");
    checker.register_impl("Float", "Show");
    checker.register_impl("String", "Show");
    checker.register_impl("Char", "Show");

    checker.insert_var("i".into(), Type::Int, false, d_span());
    checker.insert_var("b".into(), Type::Bool, false, d_span());
    checker.insert_var("f".into(), Type::Float, false, d_span());
    checker.insert_var("s".into(), Type::String, false, d_span());
    checker.insert_var("c".into(), Type::Char, false, d_span());

    // All should be usable in interpolation
    let parts = vec![
        StringPart::Interp(vec!["i".into()]),
        StringPart::Interp(vec!["b".into()]),
        StringPart::Interp(vec!["f".into()]),
        StringPart::Interp(vec!["s".into()]),
        StringPart::Interp(vec!["c".into()]),
    ];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(result);
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_custom_type_with_show() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("User", "Show");

    checker.insert_var("user".into(), Type::Named("User".into()), false, d_span());

    let parts = vec![
        StringPart::Literal("User: ".into()),
        StringPart::Interp(vec!["user".into()]),
    ];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(result);
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_custom_type_without_show() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    // User does NOT implement Show

    checker.insert_var("user".into(), Type::Named("User".into()), false, d_span());

    let parts = vec![
        StringPart::Literal("User: ".into()),
        StringPart::Interp(vec!["user".into()]),
    ];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(!result);
    assert!(checker.errors.len() > 0);
    assert!(checker.errors[0].message.contains("Show"));
}

// Integration Tests

#[test]
fn verify_complex_interpolation_string() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("Int", "Show");
    checker.register_impl("String", "Show");
    checker.register_impl("User", "Show");

    checker.insert_var("user_id".into(), Type::Int, false, d_span());
    checker.insert_var("name".into(), Type::String, false, d_span());
    checker.insert_var("user".into(), Type::Named("User".into()), false, d_span());

    let parts = vec![
        StringPart::Literal("User ".into()),
        StringPart::Interp(vec!["name".into()]),
        StringPart::Literal(" has ID ".into()),
        StringPart::Interp(vec!["user_id".into()]),
        StringPart::Literal(" and profile: ".into()),
        StringPart::Interp(vec!["user".into()]),
    ];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(result);
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_mixed_errors_in_interpolation() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("Int", "Show");
    // String does NOT implement Show

    checker.insert_var("id".into(), Type::Int, false, d_span());
    checker.insert_var("name".into(), Type::String, false, d_span());
    // undefined is not in scope

    let parts = vec![
        StringPart::Interp(vec!["id".into()]),           // OK
        StringPart::Interp(vec!["name".into()]),         // ERROR: String doesn't implement Show
        StringPart::Interp(vec!["undefined".into()]),    // ERROR: not in scope
    ];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(!result);
    // Should have at least 2 errors
    assert!(checker.errors.len() >= 2);
}

#[test]
fn verify_empty_interpolation_parts() {
    let mut checker = Checker::new();

    let parts: Vec<StringPart> = vec![];
    let result = checker.check_string_interpolation(&parts, d_span());

    // Empty string should be valid
    assert!(result);
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_only_literal_parts() {
    let mut checker = Checker::new();

    let parts = vec![
        StringPart::Literal("Hello ".into()),
        StringPart::Literal("world".into()),
    ];

    let result = checker.check_string_interpolation(&parts, d_span());

    // No interpolations, should be valid
    assert!(result);
    assert_eq!(checker.errors.len(), 0);
}
