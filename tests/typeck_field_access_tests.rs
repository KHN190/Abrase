use ect::ty::Type;
use ect::typeck::Checker;
use ect::ast::Span;

fn d_span() -> Span {
    Span { line: 0, col: 0 }
}

fn sp<T>(node: T) -> ect::ast::Spanned<T> {
    ect::ast::Spanned { node, span: d_span() }
}

// Basic Field Access Tests

#[test]
fn verify_field_access_simple_record() {
    let mut checker = Checker::new();

    // Register a simple Point record
    let point_type = ect::ast::TypeBody::Record(vec![
        ect::ast::RecordField {
            is_pub: true,
            name: "x".into(),
            ty: ect::ast::Type::Named("Int".into()),
        },
        ect::ast::RecordField {
            is_pub: true,
            name: "y".into(),
            ty: ect::ast::Type::Named("Int".into()),
        },
    ]);
    checker.register_type("Point".into(), point_type);

    checker.insert_var("p".into(), Type::Named("Point".into()), false, d_span());

    let expr = sp(ect::ast::Expr::FieldAccess {
        base: Box::new(sp(ect::ast::Expr::Identifier("p".into()))),
        field: "x".into(),
    });

    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Int);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_field_access_returns_correct_type() {
    // Register a User record with mixed field types
    let user_type = ect::ast::TypeBody::Record(vec![
        ect::ast::RecordField {
            is_pub: true,
            name: "name".into(),
            ty: ect::ast::Type::Named("String".into()),
        },
        ect::ast::RecordField {
            is_pub: true,
            name: "age".into(),
            ty: ect::ast::Type::Named("Int".into()),
        },
        ect::ast::RecordField {
            is_pub: true,
            name: "active".into(),
            ty: ect::ast::Type::Named("Bool".into()),
        },
    ]);

    // Access name field (String)
    let mut checker = Checker::new();
    checker.register_type("User".into(), user_type.clone());
    checker.insert_var("user".into(), Type::Named("User".into()), false, d_span());
    let name_expr = sp(ect::ast::Expr::FieldAccess {
        base: Box::new(sp(ect::ast::Expr::Identifier("user".into()))),
        field: "name".into(),
    });
    assert_eq!(checker.infer_expr(&name_expr), Type::String);

    // Access age field (Int)
    let mut checker = Checker::new();
    checker.register_type("User".into(), user_type.clone());
    checker.insert_var("user".into(), Type::Named("User".into()), false, d_span());
    let age_expr = sp(ect::ast::Expr::FieldAccess {
        base: Box::new(sp(ect::ast::Expr::Identifier("user".into()))),
        field: "age".into(),
    });
    assert_eq!(checker.infer_expr(&age_expr), Type::Int);

    // Access active field (Bool)
    let mut checker = Checker::new();
    checker.register_type("User".into(), user_type);
    checker.insert_var("user".into(), Type::Named("User".into()), false, d_span());
    let active_expr = sp(ect::ast::Expr::FieldAccess {
        base: Box::new(sp(ect::ast::Expr::Identifier("user".into()))),
        field: "active".into(),
    });
    assert_eq!(checker.infer_expr(&active_expr), Type::Bool);
}

// Error Cases

#[test]
fn verify_field_access_unknown_field_reports_error() {
    let mut checker = Checker::new();

    let point_type = ect::ast::TypeBody::Record(vec![
        ect::ast::RecordField {
            is_pub: true,
            name: "x".into(),
            ty: ect::ast::Type::Named("Int".into()),
        },
    ]);
    checker.register_type("Point".into(), point_type);

    checker.insert_var("p".into(), Type::Named("Point".into()), false, d_span());

    let expr = sp(ect::ast::Expr::FieldAccess {
        base: Box::new(sp(ect::ast::Expr::Identifier("p".into()))),
        field: "z".into(),
    });

    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Unknown);
    assert!(!checker.errors.is_empty());
    assert!(checker.errors[0].message.contains("not found"));
}

#[test]
fn verify_field_access_unregistered_type_reports_error() {
    let mut checker = Checker::new();

    checker.insert_var("obj".into(), Type::Named("UnknownType".into()), false, d_span());

    let expr = sp(ect::ast::Expr::FieldAccess {
        base: Box::new(sp(ect::ast::Expr::Identifier("obj".into()))),
        field: "field".into(),
    });

    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Unknown);
    assert!(!checker.errors.is_empty());
    assert!(checker.errors[0].message.contains("not found"));
}

#[test]
fn verify_field_access_on_variant_reports_error() {
    let mut checker = Checker::new();

    // Register an Option variant
    let option_type = ect::ast::TypeBody::Variant(vec![
        ect::ast::VariantCase::Unit("None".into()),
        ect::ast::VariantCase::Tuple("Some".into(), vec![ect::ast::Type::Named("T".into())]),
    ]);
    checker.register_type("Option".into(), option_type);

    checker.insert_var("opt".into(), Type::Named("Option".into()), false, d_span());

    let expr = sp(ect::ast::Expr::FieldAccess {
        base: Box::new(sp(ect::ast::Expr::Identifier("opt".into()))),
        field: "value".into(),
    });

    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Unknown);
    assert!(!checker.errors.is_empty());
    assert!(checker.errors[0].message.contains("Cannot access field"));
}

// Nested Field Access Tests

#[test]
fn verify_nested_field_access() {
    // Register Point type
    let point_type = ect::ast::TypeBody::Record(vec![
        ect::ast::RecordField {
            is_pub: true,
            name: "x".into(),
            ty: ect::ast::Type::Named("Int".into()),
        },
    ]);

    // Register Shape type containing a Point
    let shape_type = ect::ast::TypeBody::Record(vec![
        ect::ast::RecordField {
            is_pub: true,
            name: "origin".into(),
            ty: ect::ast::Type::Named("Point".into()),
        },
    ]);

    let mut checker = Checker::new();
    checker.register_type("Point".into(), point_type.clone());
    checker.register_type("Shape".into(), shape_type.clone());
    checker.insert_var("shape".into(), Type::Named("Shape".into()), false, d_span());

    // Access shape.origin (returns Point)
    let origin_expr = sp(ect::ast::Expr::FieldAccess {
        base: Box::new(sp(ect::ast::Expr::Identifier("shape".into()))),
        field: "origin".into(),
    });
    let origin_ty = checker.infer_expr(&origin_expr);
    assert_eq!(origin_ty, Type::Named("Point".into()));

    // Access shape.origin.x (returns Int) - fresh checker for nested access
    let mut checker2 = Checker::new();
    checker2.register_type("Point".into(), point_type);
    checker2.register_type("Shape".into(), shape_type);
    checker2.insert_var("shape".into(), Type::Named("Shape".into()), false, d_span());

    // Build the nested expression directly
    let x_expr = sp(ect::ast::Expr::FieldAccess {
        base: Box::new(sp(ect::ast::Expr::FieldAccess {
            base: Box::new(sp(ect::ast::Expr::Identifier("shape".into()))),
            field: "origin".into(),
        })),
        field: "x".into(),
    });
    let x_ty = checker2.infer_expr(&x_expr);
    assert_eq!(x_ty, Type::Int);
}

// Field Access with Different Types

#[test]
fn verify_field_access_with_reference_type() {
    let mut checker = Checker::new();

    let point_type = ect::ast::TypeBody::Record(vec![
        ect::ast::RecordField {
            is_pub: true,
            name: "x".into(),
            ty: ect::ast::Type::Named("Int".into()),
        },
    ]);
    checker.register_type("Point".into(), point_type);

    // Variable holds a reference to Point
    let ref_ty = Type::Reference {
        is_mut: false,
        inner: Box::new(Type::Named("Point".into())),
    };
    checker.insert_var("p_ref".into(), ref_ty, false, d_span());

    let expr = sp(ect::ast::Expr::FieldAccess {
        base: Box::new(sp(ect::ast::Expr::Identifier("p_ref".into()))),
        field: "x".into(),
    });

    // Should still report error since reference types can't directly access fields (need deref)
    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Unknown);
}

#[test]
fn verify_field_access_with_tuple_type_fails() {
    let mut checker = Checker::new();

    let tuple_type = Type::Tuple(vec![Type::Int, Type::String]);
    checker.insert_var("t".into(), tuple_type, false, d_span());

    let expr = sp(ect::ast::Expr::FieldAccess {
        base: Box::new(sp(ect::ast::Expr::Identifier("t".into()))),
        field: "x".into(),
    });

    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Unknown);
    assert!(!checker.errors.is_empty());
}

// Generic Record Field Access

#[test]
fn verify_field_access_generic_record() {
    let mut checker = Checker::new();

    // Register a generic Pair<T> record
    let pair_type = ect::ast::TypeBody::Record(vec![
        ect::ast::RecordField {
            is_pub: true,
            name: "first".into(),
            ty: ect::ast::Type::Named("T".into()),
        },
        ect::ast::RecordField {
            is_pub: true,
            name: "second".into(),
            ty: ect::ast::Type::Named("T".into()),
        },
    ]);
    checker.register_type("Pair".into(), pair_type);

    // Variable has type Pair (generic instance)
    let pair_ty = Type::Generic {
        name: "Pair".into(),
        args: vec![Type::Int],
    };
    checker.insert_var("pair".into(), pair_ty, false, d_span());

    let expr = sp(ect::ast::Expr::FieldAccess {
        base: Box::new(sp(ect::ast::Expr::Identifier("pair".into()))),
        field: "first".into(),
    });

    // Should return the field type (T, which is Int in this instantiation)
    // Currently returns T (the declared type), but ideally would substitute generics
    let _ty = checker.infer_expr(&expr);
    assert!(checker.errors.is_empty());
}

// Multiple Fields in Sequence

#[test]
fn verify_multiple_field_accesses_same_object() {
    let record_type = ect::ast::TypeBody::Record(vec![
        ect::ast::RecordField {
            is_pub: true,
            name: "a".into(),
            ty: ect::ast::Type::Named("Int".into()),
        },
        ect::ast::RecordField {
            is_pub: true,
            name: "b".into(),
            ty: ect::ast::Type::Named("String".into()),
        },
        ect::ast::RecordField {
            is_pub: true,
            name: "c".into(),
            ty: ect::ast::Type::Named("Bool".into()),
        },
    ]);

    // Access field a
    let mut checker = Checker::new();
    checker.register_type("Record".into(), record_type.clone());
    checker.insert_var("r".into(), Type::Named("Record".into()), false, d_span());
    let a_expr = sp(ect::ast::Expr::FieldAccess {
        base: Box::new(sp(ect::ast::Expr::Identifier("r".into()))),
        field: "a".into(),
    });
    assert_eq!(checker.infer_expr(&a_expr), Type::Int);

    // Access field b
    let mut checker = Checker::new();
    checker.register_type("Record".into(), record_type.clone());
    checker.insert_var("r".into(), Type::Named("Record".into()), false, d_span());
    let b_expr = sp(ect::ast::Expr::FieldAccess {
        base: Box::new(sp(ect::ast::Expr::Identifier("r".into()))),
        field: "b".into(),
    });
    assert_eq!(checker.infer_expr(&b_expr), Type::String);

    // Access field c
    let mut checker = Checker::new();
    checker.register_type("Record".into(), record_type);
    checker.insert_var("r".into(), Type::Named("Record".into()), false, d_span());
    let c_expr = sp(ect::ast::Expr::FieldAccess {
        base: Box::new(sp(ect::ast::Expr::Identifier("r".into()))),
        field: "c".into(),
    });
    assert_eq!(checker.infer_expr(&c_expr), Type::Bool);
}

// Unknown Base Type Propagation

#[test]
fn verify_field_access_on_unknown_type() {
    let mut checker = Checker::new();

    checker.insert_var("unknown".into(), Type::Unknown, false, d_span());

    let expr = sp(ect::ast::Expr::FieldAccess {
        base: Box::new(sp(ect::ast::Expr::Identifier("unknown".into()))),
        field: "field".into(),
    });

    let ty = checker.infer_expr(&expr);
    // Unknown propagates as Unknown
    assert_eq!(ty, Type::Unknown);
}

// Generic Field Type Substitution Tests

#[test]
fn verify_generic_field_type_substitution_with_int() {
    let checker = Checker::new();

    // When accessing a field of type T on Pair<Int>, substitute T with Int
    let pair_int_ty = Type::Generic {
        name: "Pair".into(),
        args: vec![Type::Int],
    };

    let field_ty = Type::Named("T".into());

    let substituted = checker.substitute_generic_field_type(&pair_int_ty, &field_ty);
    assert_eq!(substituted, Type::Int);
}

#[test]
fn verify_generic_field_type_substitution_with_string() {
    let checker = Checker::new();

    // When accessing a field of type T on Pair<String>, substitute T with String
    let pair_string_ty = Type::Generic {
        name: "Pair".into(),
        args: vec![Type::String],
    };

    let field_ty = Type::Named("T".into());

    let substituted = checker.substitute_generic_field_type(&pair_string_ty, &field_ty);
    assert_eq!(substituted, Type::String);
}

#[test]
fn verify_generic_field_type_no_substitution_for_non_t() {
    let checker = Checker::new();

    // Field type that's not T should not be substituted
    let pair_int_ty = Type::Generic {
        name: "Pair".into(),
        args: vec![Type::Int],
    };

    let field_ty = Type::Named("U".into());

    let substituted = checker.substitute_generic_field_type(&pair_int_ty, &field_ty);
    assert_eq!(substituted, Type::Named("U".into()));
}

#[test]
fn verify_generic_field_type_no_substitution_for_named_type() {
    let checker = Checker::new();

    // Named (non-generic) type should not substitute
    let pair_ty = Type::Named("Pair".into());

    let field_ty = Type::Named("T".into());

    let substituted = checker.substitute_generic_field_type(&pair_ty, &field_ty);
    assert_eq!(substituted, Type::Named("T".into()));
}

#[test]
fn verify_field_access_on_generic_pair_returns_substituted_type() {
    let mut checker = Checker::new();

    // Register a Pair<T> record
    let pair_type = ect::ast::TypeBody::Record(vec![
        ect::ast::RecordField {
            is_pub: true,
            name: "first".into(),
            ty: ect::ast::Type::Named("T".into()),
        },
        ect::ast::RecordField {
            is_pub: true,
            name: "second".into(),
            ty: ect::ast::Type::Named("T".into()),
        },
    ]);
    checker.register_type("Pair".into(), pair_type);

    // Variable has type Pair<Int>
    let pair_int_ty = Type::Generic {
        name: "Pair".into(),
        args: vec![Type::Int],
    };
    checker.insert_var("pair".into(), pair_int_ty, false, d_span());

    let expr = sp(ect::ast::Expr::FieldAccess {
        base: Box::new(sp(ect::ast::Expr::Identifier("pair".into()))),
        field: "first".into(),
    });

    let ty = checker.infer_expr(&expr);
    // With substitution, should return Int instead of T
    assert_eq!(ty, Type::Int);
    assert!(checker.errors.is_empty());
}
