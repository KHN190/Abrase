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

// Field Type Resolution from TypeBody (Record Fields)

#[test]
fn verify_field_type_from_record_body() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("String", "Show");
    checker.register_impl("Int", "Show");

    // Register User type with record body
    let user_type = TypeBody::Record(vec![
        RecordField {
            name: "name".into(),
            ty: AstType::Named("String".into()),
            is_pub: false,
        },
        RecordField {
            name: "age".into(),
            ty: AstType::Named("Int".into()),
            is_pub: false,
        },
    ]);
    checker.register_type("User".into(), user_type);

    // Insert variable of User type
    checker.insert_var("user".into(), Type::Named("User".into()), false, d_span());

    // Access field {user.name} - should resolve to String
    let parts = vec![StringPart::Interp(vec!["user".into(), "name".into()])];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(result, "Field type resolution should validate that String implements Show");
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_field_type_mismatch_no_show() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    // Only register String as implementing Show, NOT Int

    let person_type = TypeBody::Record(vec![
        RecordField {
            name: "id".into(),
            ty: AstType::Named("Int".into()),
            is_pub: false,
        },
    ]);
    checker.register_type("Person".into(), person_type);

    checker.insert_var("person".into(), Type::Named("Person".into()), false, d_span());

    // Access field {person.id} - resolves to Int which doesn't implement Show
    let parts = vec![StringPart::Interp(vec!["person".into(), "id".into()])];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(!result, "Should fail because Int doesn't implement Show");
    assert!(checker.errors.len() > 0);
    assert!(checker.errors[0].message.contains("Show") ||
            checker.errors[0].message.contains("Int"));
}

#[test]
fn verify_nested_field_access() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("String", "Show");

    // Address record with street field
    let address_type = TypeBody::Record(vec![
        RecordField {
            name: "street".into(),
            ty: AstType::Named("String".into()),
            is_pub: false,
        },
    ]);
    checker.register_type("Address".into(), address_type);

    // Person record with address field
    let person_type = TypeBody::Record(vec![
        RecordField {
            name: "address".into(),
            ty: AstType::Named("Address".into()),
            is_pub: false,
        },
    ]);
    checker.register_type("Person".into(), person_type);

    checker.insert_var("person".into(), Type::Named("Person".into()), false, d_span());

    // Access nested: {person.address.street}
    let parts = vec![StringPart::Interp(vec![
        "person".into(),
        "address".into(),
        "street".into(),
    ])];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(result);
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_field_not_found_in_record() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);

    let user_type = TypeBody::Record(vec![
        RecordField {
            name: "name".into(),
            ty: AstType::Named("String".into()),
            is_pub: false,
        },
    ]);
    checker.register_type("User".into(), user_type);

    checker.insert_var("user".into(), Type::Named("User".into()), false, d_span());

    // Try to access non-existent field
    let parts = vec![StringPart::Interp(vec!["user".into(), "email".into()])];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(!result);
    assert!(checker.errors.len() > 0);
    assert!(checker.errors[0].message.contains("email") ||
            checker.errors[0].message.contains("field"));
}

// Scope Depth Lookup (Nested Scopes)

#[test]
fn verify_variable_in_parent_scope() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("Int", "Show");

    // Insert in outer scope
    checker.insert_var("x".into(), Type::Int, false, d_span());

    // Enter nested scope
    checker.enter_scope();

    // Variable x should still be accessible in nested scope
    let parts = vec![StringPart::Interp(vec!["x".into()])];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(result, "Should find variable in parent scope");
    assert_eq!(checker.errors.len(), 0);

    checker.exit_scope();
}

#[test]
fn verify_variable_shadowing_in_scope() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("Int", "Show");
    checker.register_impl("String", "Show");

    // Outer scope: x is Int
    checker.insert_var("x".into(), Type::Int, false, d_span());

    checker.enter_scope();

    // Inner scope: x is shadowed with String
    checker.insert_var("x".into(), Type::String, false, d_span());

    let parts = vec![StringPart::Interp(vec!["x".into()])];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(result);
    // Should use String type from inner scope
    assert_eq!(checker.errors.len(), 0);

    checker.exit_scope();
}

#[test]
fn verify_deeply_nested_scope_lookup() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("Int", "Show");

    // Root scope
    checker.insert_var("root_var".into(), Type::Int, false, d_span());

    // Nested 3 levels deep
    checker.enter_scope();
    checker.enter_scope();
    checker.enter_scope();

    let parts = vec![StringPart::Interp(vec!["root_var".into()])];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(result, "Should find variable in deeply nested parent scope");

    checker.exit_scope();
    checker.exit_scope();
    checker.exit_scope();
}

// Qualified Name Resolution

#[test]
fn verify_qualified_name_basic() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("String", "Show");

    // Insert variable with qualified type name (simple case for now)
    checker.insert_var("msg".into(), Type::String, false, d_span());

    let parts = vec![StringPart::Interp(vec!["msg".into()])];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(result);
}

#[test]
fn verify_qualified_module_path() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);

    // Register type from module: std.io.Error
    checker.register_type(
        "Error".into(),
        TypeBody::Record(vec![
            RecordField {
                name: "message".into(),
                ty: AstType::Named("String".into()),
                is_pub: false,
            },
        ]),
    );
    checker.register_impl("String", "Show");

    // Insert module-qualified variable
    checker.insert_var("err".into(), Type::Named("Error".into()), false, d_span());

    let parts = vec![StringPart::Interp(vec!["err".into(), "message".into()])];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(result);
}

// Trait Bound Verification (where clauses)

#[test]
fn verify_generic_with_show_bound() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);

    // Register generic parameter T with Show bound
    checker.register_trait_bound("T".into(), "Show".into());

    // Variable of generic type with Show bound
    checker.insert_var("item".into(), Type::Named("T".into()), false, d_span());

    let parts = vec![StringPart::Interp(vec!["item".into()])];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(result, "Generic T with Show bound should be valid in interpolation");
}

#[test]
fn verify_generic_without_show_bound() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    // T is NOT registered with Show bound

    // Variable of generic type without Show bound
    checker.insert_var("item".into(), Type::Named("T".into()), false, d_span());

    let parts = vec![StringPart::Interp(vec!["item".into()])];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(!result, "Generic T without Show bound should fail");
    assert!(checker.errors.len() > 0);
}

#[test]
fn verify_multiple_trait_bounds() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_trait("Clone".into(), vec!["clone".into()]);

    // Register T with both Show and Clone bounds
    checker.register_trait_bound("T".into(), "Show".into());
    checker.register_trait_bound("T".into(), "Clone".into());

    checker.insert_var("item".into(), Type::Named("T".into()), false, d_span());

    let parts = vec![StringPart::Interp(vec!["item".into()])];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(result, "T with Show and Clone bounds should be valid");
}

// Automatic Dereference for Reference Types

#[test]
fn verify_reference_auto_deref() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("Int", "Show");

    // Variable of reference type &Int
    let ref_int = Type::Reference {
        is_mut: false,
        inner: Box::new(Type::Int),
    };
    checker.insert_var("ref_x".into(), ref_int, false, d_span());

    // Should auto-deref and check that Int implements Show
    let parts = vec![StringPart::Interp(vec!["ref_x".into()])];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(result, "Should auto-deref &Int and find that Int implements Show");
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_mutable_reference_auto_deref() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("String", "Show");

    // Variable of mutable reference type &mut String
    let ref_string = Type::Reference {
        is_mut: true,
        inner: Box::new(Type::String),
    };
    checker.insert_var("ref_s".into(), ref_string, false, d_span());

    let parts = vec![StringPart::Interp(vec!["ref_s".into()])];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(result, "Should auto-deref &mut String");
}

#[test]
fn verify_reference_to_type_without_show() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    // Int does NOT implement Show

    let ref_int = Type::Reference {
        is_mut: false,
        inner: Box::new(Type::Int),
    };
    checker.insert_var("ref_x".into(), ref_int, false, d_span());

    let parts = vec![StringPart::Interp(vec!["ref_x".into()])];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(!result, "Should fail because Int doesn't implement Show");
    assert!(checker.errors.len() > 0);
}

#[test]
fn verify_reference_field_access() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("String", "Show");

    let user_type = TypeBody::Record(vec![
        RecordField {
            name: "name".into(),
            ty: AstType::Named("String".into()),
            is_pub: false,
        },
    ]);
    checker.register_type("User".into(), user_type);

    // Variable of reference type &User
    let ref_user = Type::Reference {
        is_mut: false,
        inner: Box::new(Type::Named("User".into())),
    };
    checker.insert_var("user_ref".into(), ref_user, false, d_span());

    // Access field through reference: {user_ref.name}
    let parts = vec![StringPart::Interp(vec!["user_ref".into(), "name".into()])];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(result, "Should auto-deref and access field");
}

// Integration Tests

#[test]
fn verify_complex_type_hierarchy() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("String", "Show");
    checker.register_impl("Int", "Show");

    // Company has Address which has String fields
    let address_type = TypeBody::Record(vec![
        RecordField {
            name: "city".into(),
            ty: AstType::Named("String".into()),
            is_pub: false,
        },
    ]);
    checker.register_type("Address".into(), address_type);

    let company_type = TypeBody::Record(vec![
        RecordField {
            name: "name".into(),
            ty: AstType::Named("String".into()),
            is_pub: false,
        },
        RecordField {
            name: "address".into(),
            ty: AstType::Named("Address".into()),
            is_pub: false,
        },
        RecordField {
            name: "employee_count".into(),
            ty: AstType::Named("Int".into()),
            is_pub: false,
        },
    ]);
    checker.register_type("Company".into(), company_type);

    checker.insert_var("company".into(), Type::Named("Company".into()), false, d_span());

    // Multiple interpolations accessing different field paths
    let parts = vec![
        StringPart::Interp(vec!["company".into(), "name".into()]),
        StringPart::Interp(vec!["company".into(), "employee_count".into()]),
        StringPart::Interp(vec!["company".into(), "address".into(), "city".into()]),
    ];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(result, "Complex type hierarchy should resolve correctly");
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_scope_and_field_combined() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("String", "Show");

    let person_type = TypeBody::Record(vec![
        RecordField {
            name: "name".into(),
            ty: AstType::Named("String".into()),
            is_pub: false,
        },
    ]);
    checker.register_type("Person".into(), person_type);

    // Outer scope
    checker.insert_var("person".into(), Type::Named("Person".into()), false, d_span());

    // Inner scope with another variable
    checker.enter_scope();
    checker.insert_var("greeting".into(), Type::String, false, d_span());

    let parts = vec![
        StringPart::Interp(vec!["greeting".into()]),
        StringPart::Interp(vec!["person".into(), "name".into()]),
    ];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(result);

    checker.exit_scope();
}