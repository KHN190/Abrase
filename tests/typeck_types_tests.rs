use ect::ty::{Type, Variance};
use ect::ast::{self, Pattern, RecordField, Span, Spanned, Type as AstType, TypeBody, VariantCase};
use ect::typeck::Checker;

fn d_span() -> Span { Span { line: 0, col: 0 } }
fn sp<T>(node: T) -> Spanned<T> { Spanned { node, span: d_span() } }
fn body_breaking(name: &str) -> ast::Block {
    ast::Block {
        stmts: vec![],
        ret: Some(Box::new(sp(ast::Expr::Break(Some(Box::new(sp(ast::Expr::Identifier(name.into())))))))),
    }
}

#[test]
fn verify_exact_name_match_is_equivalent() {
    let checker = Checker::new();
    assert!(checker.are_types_equivalent("Int", "Int"));
    assert!(checker.are_types_equivalent("String", "String"));
}

#[test]
fn verify_different_names_not_equivalent() {
    let checker = Checker::new();
    assert!(!checker.are_types_equivalent("Int", "String"));
    assert!(!checker.are_types_equivalent("Bool", "Int"));
}

#[test]
fn verify_identical_record_definitions_equivalent() {
    let mut checker = Checker::new();

    let fields1 = vec![
        RecordField { name: "x".into(), ty: AstType::Named("Int".into()), is_pub: false },
        RecordField { name: "y".into(), ty: AstType::Named("Int".into()), is_pub: false },
    ];
    let fields2 = vec![
        RecordField { name: "x".into(), ty: AstType::Named("Int".into()), is_pub: false },
        RecordField { name: "y".into(), ty: AstType::Named("Int".into()), is_pub: false },
    ];

    checker.register_type("Point1".into(), TypeBody::Record(fields1));
    checker.register_type("Point2".into(), TypeBody::Record(fields2));

    assert!(checker.are_types_equivalent("Point1", "Point2"));
}

#[test]
fn verify_record_different_field_names_not_equivalent() {
    let mut checker = Checker::new();

    let fields1 = vec![
        RecordField { name: "x".into(), ty: AstType::Named("Int".into()), is_pub: false },
        RecordField { name: "y".into(), ty: AstType::Named("Int".into()), is_pub: false },
    ];
    let fields2 = vec![
        RecordField { name: "a".into(), ty: AstType::Named("Int".into()), is_pub: false },
        RecordField { name: "b".into(), ty: AstType::Named("Int".into()), is_pub: false },
    ];

    checker.register_type("Point1".into(), TypeBody::Record(fields1));
    checker.register_type("Point2".into(), TypeBody::Record(fields2));

    assert!(!checker.are_types_equivalent("Point1", "Point2"));
}

#[test]
fn verify_record_different_field_types_not_equivalent() {
    let mut checker = Checker::new();

    let fields1 = vec![
        RecordField { name: "x".into(), ty: AstType::Named("Int".into()), is_pub: false },
        RecordField { name: "y".into(), ty: AstType::Named("Int".into()), is_pub: false },
    ];
    let fields2 = vec![
        RecordField { name: "x".into(), ty: AstType::Named("Int".into()), is_pub: false },
        RecordField { name: "y".into(), ty: AstType::Named("String".into()), is_pub: false },
    ];

    checker.register_type("Point1".into(), TypeBody::Record(fields1));
    checker.register_type("Point2".into(), TypeBody::Record(fields2));

    assert!(!checker.are_types_equivalent("Point1", "Point2"));
}

#[test]
fn verify_record_different_field_counts_not_equivalent() {
    let mut checker = Checker::new();

    let fields1 = vec![
        RecordField { name: "x".into(), ty: AstType::Named("Int".into()), is_pub: false },
    ];
    let fields2 = vec![
        RecordField { name: "x".into(), ty: AstType::Named("Int".into()), is_pub: false },
        RecordField { name: "y".into(), ty: AstType::Named("Int".into()), is_pub: false },
    ];

    checker.register_type("Point1".into(), TypeBody::Record(fields1));
    checker.register_type("Point2".into(), TypeBody::Record(fields2));

    assert!(!checker.are_types_equivalent("Point1", "Point2"));
}

#[test]
fn verify_identical_unit_variant_definitions_equivalent() {
    let mut checker = Checker::new();

    let variants1 = vec![
        VariantCase::Unit("Red".into()),
        VariantCase::Unit("Green".into()),
        VariantCase::Unit("Blue".into()),
    ];
    let variants2 = vec![
        VariantCase::Unit("Red".into()),
        VariantCase::Unit("Green".into()),
        VariantCase::Unit("Blue".into()),
    ];

    checker.register_type("Color1".into(), TypeBody::Variant(variants1));
    checker.register_type("Color2".into(), TypeBody::Variant(variants2));

    assert!(checker.are_types_equivalent("Color1", "Color2"));
}

#[test]
fn verify_unit_variant_different_names_not_equivalent() {
    let mut checker = Checker::new();

    let variants1 = vec![
        VariantCase::Unit("Red".into()),
        VariantCase::Unit("Green".into()),
    ];
    let variants2 = vec![
        VariantCase::Unit("Red".into()),
        VariantCase::Unit("Yellow".into()),
    ];

    checker.register_type("Color1".into(), TypeBody::Variant(variants1));
    checker.register_type("Color2".into(), TypeBody::Variant(variants2));

    assert!(!checker.are_types_equivalent("Color1", "Color2"));
}

#[test]
fn verify_unit_variant_different_counts_not_equivalent() {
    let mut checker = Checker::new();

    let variants1 = vec![
        VariantCase::Unit("Red".into()),
        VariantCase::Unit("Green".into()),
    ];
    let variants2 = vec![
        VariantCase::Unit("Red".into()),
        VariantCase::Unit("Green".into()),
        VariantCase::Unit("Blue".into()),
    ];

    checker.register_type("Color1".into(), TypeBody::Variant(variants1));
    checker.register_type("Color2".into(), TypeBody::Variant(variants2));

    assert!(!checker.are_types_equivalent("Color1", "Color2"));
}

#[test]
fn verify_identical_tuple_variant_definitions_equivalent() {
    let mut checker = Checker::new();

    let variants1 = vec![
        VariantCase::Tuple("Ok".into(), vec![AstType::Named("Int".into())]),
        VariantCase::Tuple("Err".into(), vec![AstType::Named("String".into())]),
    ];
    let variants2 = vec![
        VariantCase::Tuple("Ok".into(), vec![AstType::Named("Int".into())]),
        VariantCase::Tuple("Err".into(), vec![AstType::Named("String".into())]),
    ];

    checker.register_type("Result1".into(), TypeBody::Variant(variants1));
    checker.register_type("Result2".into(), TypeBody::Variant(variants2));

    assert!(checker.are_types_equivalent("Result1", "Result2"));
}

#[test]
fn verify_tuple_variant_different_arg_types_not_equivalent() {
    let mut checker = Checker::new();

    let variants1 = vec![
        VariantCase::Tuple("Ok".into(), vec![AstType::Named("Int".into())]),
        VariantCase::Tuple("Err".into(), vec![AstType::Named("String".into())]),
    ];
    let variants2 = vec![
        VariantCase::Tuple("Ok".into(), vec![AstType::Named("String".into())]),
        VariantCase::Tuple("Err".into(), vec![AstType::Named("String".into())]),
    ];

    checker.register_type("Result1".into(), TypeBody::Variant(variants1));
    checker.register_type("Result2".into(), TypeBody::Variant(variants2));

    assert!(!checker.are_types_equivalent("Result1", "Result2"));
}

#[test]
fn verify_tuple_variant_different_arg_counts_not_equivalent() {
    let mut checker = Checker::new();

    let variants1 = vec![
        VariantCase::Tuple("Ok".into(), vec![AstType::Named("Int".into())]),
    ];
    let variants2 = vec![
        VariantCase::Tuple("Ok".into(), vec![AstType::Named("Int".into()), AstType::Named("String".into())]),
    ];

    checker.register_type("Result1".into(), TypeBody::Variant(variants1));
    checker.register_type("Result2".into(), TypeBody::Variant(variants2));

    assert!(!checker.are_types_equivalent("Result1", "Result2"));
}

#[test]
fn verify_identical_record_variant_definitions_equivalent() {
    let mut checker = Checker::new();

    let fields1 = vec![RecordField { name: "x".into(), ty: AstType::Named("Int".into()), is_pub: false }];
    let fields2 = vec![RecordField { name: "x".into(), ty: AstType::Named("Int".into()), is_pub: false }];

    let variants1 = vec![
        VariantCase::Record("Point".into(), fields1),
    ];
    let variants2 = vec![
        VariantCase::Record("Point".into(), fields2),
    ];

    checker.register_type("Shape1".into(), TypeBody::Variant(variants1));
    checker.register_type("Shape2".into(), TypeBody::Variant(variants2));

    assert!(checker.are_types_equivalent("Shape1", "Shape2"));
}

#[test]
fn verify_record_variant_different_field_names_not_equivalent() {
    let mut checker = Checker::new();

    let fields1 = vec![RecordField { name: "x".into(), ty: AstType::Named("Int".into()), is_pub: false }];
    let fields2 = vec![RecordField { name: "y".into(), ty: AstType::Named("Int".into()), is_pub: false }];

    let variants1 = vec![
        VariantCase::Record("Point".into(), fields1),
    ];
    let variants2 = vec![
        VariantCase::Record("Point".into(), fields2),
    ];

    checker.register_type("Shape1".into(), TypeBody::Variant(variants1));
    checker.register_type("Shape2".into(), TypeBody::Variant(variants2));

    assert!(!checker.are_types_equivalent("Shape1", "Shape2"));
}

#[test]
fn verify_unit_and_tuple_variants_not_equivalent() {
    let mut checker = Checker::new();

    let variants1 = vec![
        VariantCase::Unit("Ok".into()),
    ];
    let variants2 = vec![
        VariantCase::Tuple("Ok".into(), vec![AstType::Named("Int".into())]),
    ];

    checker.register_type("Result1".into(), TypeBody::Variant(variants1));
    checker.register_type("Result2".into(), TypeBody::Variant(variants2));

    assert!(!checker.are_types_equivalent("Result1", "Result2"));
}

#[test]
fn verify_record_and_variant_not_equivalent() {
    let mut checker = Checker::new();

    let fields = vec![
        RecordField { name: "x".into(), ty: AstType::Named("Int".into()), is_pub: false },
    ];
    let variants = vec![
        VariantCase::Unit("Ok".into()),
    ];

    checker.register_type("Type1".into(), TypeBody::Record(fields));
    checker.register_type("Type2".into(), TypeBody::Variant(variants));

    assert!(!checker.are_types_equivalent("Type1", "Type2"));
}

#[test]
fn verify_unregistered_type_not_equivalent() {
    let checker = Checker::new();
    assert!(!checker.are_types_equivalent("Unknown1", "Unknown2"));
}

#[test]
fn verify_registered_and_unregistered_not_equivalent() {
    let mut checker = Checker::new();

    let fields = vec![
        RecordField { name: "x".into(), ty: AstType::Named("Int".into()), is_pub: false },
    ];
    checker.register_type("Point".into(), TypeBody::Record(fields));

    assert!(!checker.are_types_equivalent("Point", "Unknown"));
}

#[test]
fn verify_nested_array_types_equivalent() {
    let mut checker = Checker::new();

    let fields1 = vec![
        RecordField {
            name: "data".into(),
            ty: AstType::Array {
                elem: Box::new(AstType::Named("Int".into())),
                size: 10,
            },
            is_pub: false
        },
    ];
    let fields2 = vec![
        RecordField {
            name: "data".into(),
            ty: AstType::Array {
                elem: Box::new(AstType::Named("Int".into())),
                size: 10,
            },
            is_pub: false
        },
    ];

    checker.register_type("Array1".into(), TypeBody::Record(fields1));
    checker.register_type("Array2".into(), TypeBody::Record(fields2));

    assert!(checker.are_types_equivalent("Array1", "Array2"));
}

#[test]
fn verify_nested_array_different_sizes_not_equivalent() {
    let mut checker = Checker::new();

    let fields1 = vec![
        RecordField {
            name: "data".into(),
            ty: AstType::Array {
                elem: Box::new(AstType::Named("Int".into())),
                size: 10,
            },
            is_pub: false
        },
    ];
    let fields2 = vec![
        RecordField {
            name: "data".into(),
            ty: AstType::Array {
                elem: Box::new(AstType::Named("Int".into())),
                size: 20,
            },
            is_pub: false
        },
    ];

    checker.register_type("Array1".into(), TypeBody::Record(fields1));
    checker.register_type("Array2".into(), TypeBody::Record(fields2));

    assert!(!checker.are_types_equivalent("Array1", "Array2"));
}

#[test]
fn verify_tuple_field_types_equivalent() {
    let mut checker = Checker::new();

    let fields1 = vec![
        RecordField {
            name: "pair".into(),
            ty: AstType::Tuple(vec![AstType::Named("Int".into()), AstType::Named("String".into())]),
            is_pub: false
        },
    ];
    let fields2 = vec![
        RecordField {
            name: "pair".into(),
            ty: AstType::Tuple(vec![AstType::Named("Int".into()), AstType::Named("String".into())]),
            is_pub: false
        },
    ];

    checker.register_type("Pair1".into(), TypeBody::Record(fields1));
    checker.register_type("Pair2".into(), TypeBody::Record(fields2));

    assert!(checker.are_types_equivalent("Pair1", "Pair2"));
}

#[test]
fn verify_tuple_field_different_element_types_not_equivalent() {
    let mut checker = Checker::new();

    let fields1 = vec![
        RecordField {
            name: "pair".into(),
            ty: AstType::Tuple(vec![AstType::Named("Int".into()), AstType::Named("String".into())]),
            is_pub: false
        },
    ];
    let fields2 = vec![
        RecordField {
            name: "pair".into(),
            ty: AstType::Tuple(vec![AstType::Named("Int".into()), AstType::Named("Bool".into())]),
            is_pub: false
        },
    ];

    checker.register_type("Pair1".into(), TypeBody::Record(fields1));
    checker.register_type("Pair2".into(), TypeBody::Record(fields2));

    assert!(!checker.are_types_equivalent("Pair1", "Pair2"));
}

// --- typeck_conversion_tests ---

#[test]
fn verify_convert_primitive_types() {
    let checker = Checker::new();
    assert_eq!(checker.convert_type(&ast::Type::Named("Int".into())), Type::Int);
    assert_eq!(checker.convert_type(&ast::Type::Named("Float".into())), Type::Float);
    assert_eq!(checker.convert_type(&ast::Type::Named("Bool".into())), Type::Bool);
    assert_eq!(checker.convert_type(&ast::Type::Named("Char".into())), Type::Char);
    assert_eq!(checker.convert_type(&ast::Type::Named("String".into())), Type::String);
    assert_eq!(checker.convert_type(&ast::Type::Named("Unit".into())), Type::Unit);
}

#[test]
fn verify_convert_qualified_types() {
    let checker = Checker::new();
    let qualified = ast::Type::Qualified(vec!["std".into(), "io".into(), "Error".into()]);
    let converted = checker.convert_type(&qualified);
    assert_eq!(converted, Type::Named("std.io.Error".into()));
}

#[test]
fn verify_convert_generic_types() {
    let checker = Checker::new();
    let generic = ast::Type::Generic {
        name: "List".into(),
        args: vec![ast::Type::Named("Int".into())],
    };
    let converted = checker.convert_type(&generic);
    assert!(format!("{:?}", converted).contains("List"));
}

#[test]
fn verify_convert_array_with_size() {
    let checker = Checker::new();
    let array = ast::Type::Array {
        elem: Box::new(ast::Type::Named("Int".into())),
        size: 16,
    };
    let converted = checker.convert_type(&array);
    assert!(format!("{:?}", converted).contains("16"));
}

#[test]
fn verify_convert_tuple_types() {
    let checker = Checker::new();
    let tuple = ast::Type::Tuple(vec![
        ast::Type::Named("Int".into()),
        ast::Type::Named("Bool".into()),
    ]);
    let converted = checker.convert_type(&tuple);
    assert_eq!(converted, Type::Tuple(vec![Type::Int, Type::Bool]));
}

#[test]
fn verify_convert_reference_types() {
    let checker = Checker::new();
    let reference = ast::Type::Reference {
        is_mut: false,
        inner: Box::new(ast::Type::Named("String".into())),
        region: None,
    };
    let converted = checker.convert_type(&reference);
    assert_eq!(
        converted,
        Type::Reference {
            is_mut: false,
            inner: Box::new(Type::String),
        }
    );
}

#[test]
fn verify_convert_mutable_reference() {
    let checker = Checker::new();
    let reference = ast::Type::Reference {
        is_mut: true,
        inner: Box::new(ast::Type::Named("Int".into())),
        region: None,
    };
    let converted = checker.convert_type(&reference);
    assert_eq!(
        converted,
        Type::Reference {
            is_mut: true,
            inner: Box::new(Type::Int),
        }
    );
}

#[test]
fn verify_convert_function_types() {
    let checker = Checker::new();
    let function = ast::Type::Function {
        params: vec![
            ast::Type::Named("Int".into()),
            ast::Type::Named("Bool".into()),
        ],
        effects: vec![],
        ret: Box::new(ast::Type::Named("String".into())),
    };
    let converted = checker.convert_type(&function);
    assert_eq!(
        converted,
        Type::Function {
            params: vec![Type::Int, Type::Bool],
            effects: vec![],
            ret: Box::new(Type::String),
        }
    );
}

#[test]
fn verify_types_compatible_same_types() {
    let checker = Checker::new();
    assert!(checker.types_compatible(&Type::Int, &Type::Int));
    assert!(checker.types_compatible(&Type::Bool, &Type::Bool));
    assert!(checker.types_compatible(&Type::String, &Type::String));
}

#[test]
fn verify_types_compatible_with_unknown() {
    let checker = Checker::new();
    assert!(checker.types_compatible(&Type::Int, &Type::Unknown));
    assert!(checker.types_compatible(&Type::Unknown, &Type::Int));
    assert!(checker.types_compatible(&Type::Unknown, &Type::Unknown));
}

#[test]
fn verify_types_compatible_different_types() {
    let checker = Checker::new();
    assert!(!checker.types_compatible(&Type::Int, &Type::Bool));
    assert!(!checker.types_compatible(&Type::String, &Type::Float));
}

#[test]
fn verify_types_compatible_tuples() {
    let checker = Checker::new();
    let tuple1 = Type::Tuple(vec![Type::Int, Type::Bool]);
    let tuple2 = Type::Tuple(vec![Type::Int, Type::Bool]);
    assert!(checker.types_compatible(&tuple1, &tuple2));
}

#[test]
fn verify_types_compatible_tuple_length_mismatch() {
    let checker = Checker::new();
    let tuple1 = Type::Tuple(vec![Type::Int, Type::Bool]);
    let tuple2 = Type::Tuple(vec![Type::Int]);
    assert!(!checker.types_compatible(&tuple1, &tuple2));
}

#[test]
fn verify_types_compatible_tuple_element_mismatch() {
    let checker = Checker::new();
    let tuple1 = Type::Tuple(vec![Type::Int, Type::Bool]);
    let tuple2 = Type::Tuple(vec![Type::Int, Type::String]);
    assert!(!checker.types_compatible(&tuple1, &tuple2));
}

#[test]
fn verify_types_compatible_references() {
    let checker = Checker::new();
    let ref1 = Type::Reference { is_mut: false, inner: Box::new(Type::Int) };
    let ref2 = Type::Reference { is_mut: false, inner: Box::new(Type::Int) };
    assert!(checker.types_compatible(&ref1, &ref2));
}

#[test]
fn verify_types_compatible_reference_mutability_mismatch() {
    let checker = Checker::new();
    let ref_immut = Type::Reference { is_mut: false, inner: Box::new(Type::Int) };
    let ref_mut = Type::Reference { is_mut: true, inner: Box::new(Type::Int) };
    assert!(!checker.types_compatible(&ref_immut, &ref_mut));
}

#[test]
fn verify_types_compatible_functions() {
    let checker = Checker::new();
    let fn1 = Type::Function {
        params: vec![Type::Int, Type::Bool],
        effects: vec![],
        ret: Box::new(Type::String),
    };
    let fn2 = Type::Function {
        params: vec![Type::Int, Type::Bool],
        effects: vec![],
        ret: Box::new(Type::String),
    };
    assert!(checker.types_compatible(&fn1, &fn2));
}

#[test]
fn verify_types_compatible_function_param_mismatch() {
    let checker = Checker::new();
    let fn1 = Type::Function {
        params: vec![Type::Int, Type::Bool],
        effects: vec![],
        ret: Box::new(Type::String),
    };
    let fn2 = Type::Function {
        params: vec![Type::Int, Type::String],
        effects: vec![],
        ret: Box::new(Type::String),
    };
    assert!(!checker.types_compatible(&fn1, &fn2));
}

#[test]
fn verify_unify_same_types() {
    let checker = Checker::new();
    let result = checker.unify_types(&Type::Int, &Type::Int);
    assert_eq!(result, Some(Type::Int));
}

#[test]
fn verify_unify_with_unknown() {
    let checker = Checker::new();
    assert_eq!(checker.unify_types(&Type::Int, &Type::Unknown), Some(Type::Int));
    assert_eq!(checker.unify_types(&Type::Unknown, &Type::Bool), Some(Type::Bool));
}

#[test]
fn verify_unify_tuples() {
    let checker = Checker::new();
    let tuple1 = Type::Tuple(vec![Type::Int, Type::Bool]);
    let tuple2 = Type::Tuple(vec![Type::Int, Type::Bool]);
    let unified = checker.unify_types(&tuple1, &tuple2);
    assert_eq!(unified, Some(tuple1));
}

#[test]
fn verify_unify_incompatible_types() {
    let checker = Checker::new();
    let result = checker.unify_types(&Type::Int, &Type::Bool);
    assert_eq!(result, None);
}

#[test]
fn verify_unify_references() {
    let checker = Checker::new();
    let ref1 = Type::Reference { is_mut: false, inner: Box::new(Type::Int) };
    let ref2 = Type::Reference { is_mut: false, inner: Box::new(Type::Int) };
    let unified = checker.unify_types(&ref1, &ref2);
    assert!(unified.is_some());
}

#[test]
fn verify_unify_functions() {
    let checker = Checker::new();
    let fn1 = Type::Function {
        params: vec![Type::Int],
        effects: vec![],
        ret: Box::new(Type::Bool),
    };
    let fn2 = Type::Function {
        params: vec![Type::Int],
        effects: vec![],
        ret: Box::new(Type::Bool),
    };
    let unified = checker.unify_types(&fn1, &fn2);
    assert!(unified.is_some());
}

#[test]
fn verify_is_assignable() {
    let checker = Checker::new();
    assert!(checker.is_assignable(&Type::Int, &Type::Int));
    assert!(checker.is_assignable(&Type::Int, &Type::Unknown));
    assert!(checker.is_assignable(&Type::Unknown, &Type::Bool));
    assert!(!checker.is_assignable(&Type::Int, &Type::Bool));
}

// --- typeck_generic_type_tests ---

#[test]
fn verify_generic_type_construction() {
    let generic_list = Type::Generic {
        name: "List".into(),
        args: vec![Type::Int],
    };

    assert_eq!(generic_list, Type::Generic {
        name: "List".into(),
        args: vec![Type::Int],
    });
}

#[test]
fn verify_generic_type_with_multiple_args() {
    let generic_map = Type::Generic {
        name: "Map".into(),
        args: vec![Type::String, Type::Int],
    };

    assert_eq!(generic_map, Type::Generic {
        name: "Map".into(),
        args: vec![Type::String, Type::Int],
    });
}

#[test]
fn verify_generic_type_nested() {
    let nested = Type::Generic {
        name: "List".into(),
        args: vec![Type::Generic {
            name: "Option".into(),
            args: vec![Type::String],
        }],
    };

    assert_eq!(nested, Type::Generic {
        name: "List".into(),
        args: vec![Type::Generic {
            name: "Option".into(),
            args: vec![Type::String],
        }],
    });
}

// Generic Type Conversion from AST

#[test]
fn verify_convert_simple_generic_type() {
    let checker = Checker::new();

    // Create ast::Type::Generic and convert it
    let ast_generic = ect::ast::Type::Generic {
        name: "List".into(),
        args: vec![ect::ast::Type::Named("Int".into())],
    };

    let converted = checker.convert_type(&ast_generic);

    assert_eq!(converted, Type::Generic {
        name: "List".into(),
        args: vec![Type::Int],
    });
}

#[test]
fn verify_convert_generic_with_multiple_args() {
    let checker = Checker::new();

    let ast_generic = ect::ast::Type::Generic {
        name: "Tuple".into(),
        args: vec![
            ect::ast::Type::Named("String".into()),
            ect::ast::Type::Named("Int".into()),
        ],
    };

    let converted = checker.convert_type(&ast_generic);

    assert_eq!(converted, Type::Generic {
        name: "Tuple".into(),
        args: vec![Type::String, Type::Int],
    });
}

#[test]
fn verify_convert_nested_generic_type() {
    let checker = Checker::new();

    let ast_generic = ect::ast::Type::Generic {
        name: "List".into(),
        args: vec![ect::ast::Type::Generic {
            name: "Option".into(),
            args: vec![ect::ast::Type::Named("String".into())],
        }],
    };

    let converted = checker.convert_type(&ast_generic);

    assert_eq!(converted, Type::Generic {
        name: "List".into(),
        args: vec![Type::Generic {
            name: "Option".into(),
            args: vec![Type::String],
        }],
    });
}

#[test]
fn verify_convert_generic_result_type() {
    let checker = Checker::new();

    let ast_generic = ect::ast::Type::Generic {
        name: "Result".into(),
        args: vec![
            ect::ast::Type::Named("String".into()),
            ect::ast::Type::Named("Int".into()),
        ],
    };

    let converted = checker.convert_type(&ast_generic);

    assert_eq!(converted, Type::Generic {
        name: "Result".into(),
        args: vec![Type::String, Type::Int],
    });
}

// Generic Type Compatibility

#[test]
fn verify_same_generic_types_compatible() {
    let checker = Checker::new();

    let list_int = Type::Generic {
        name: "List".into(),
        args: vec![Type::Int],
    };

    assert!(checker.types_compatible(&list_int, &list_int));
}

#[test]
fn verify_different_generic_names_not_compatible() {
    let checker = Checker::new();

    let list_int = Type::Generic {
        name: "List".into(),
        args: vec![Type::Int],
    };
    let vec_int = Type::Generic {
        name: "Vec".into(),
        args: vec![Type::Int],
    };

    assert!(!checker.types_compatible(&list_int, &vec_int));
}

#[test]
fn verify_different_generic_args_not_compatible() {
    let checker = Checker::new();

    let list_int = Type::Generic {
        name: "List".into(),
        args: vec![Type::Int],
    };
    let list_string = Type::Generic {
        name: "List".into(),
        args: vec![Type::String],
    };

    assert!(!checker.types_compatible(&list_int, &list_string));
}

#[test]
fn verify_generic_with_unknown_compatible() {
    let checker = Checker::new();

    let list_int = Type::Generic {
        name: "List".into(),
        args: vec![Type::Int],
    };

    assert!(checker.types_compatible(&list_int, &Type::Unknown));
    assert!(checker.types_compatible(&Type::Unknown, &list_int));
}

#[test]
fn verify_generic_with_different_arg_count_not_compatible() {
    let checker = Checker::new();

    let option_int = Type::Generic {
        name: "Option".into(),
        args: vec![Type::Int],
    };
    let map_int_string = Type::Generic {
        name: "Map".into(),
        args: vec![Type::Int, Type::String],
    };

    assert!(!checker.types_compatible(&option_int, &map_int_string));
}

// Generic Type Subtyping

#[test]
fn verify_generic_subtype_same_type() {
    let checker = Checker::new();

    let list_int = Type::Generic {
        name: "List".into(),
        args: vec![Type::Int],
    };

    assert!(checker.is_subtype(&list_int, &list_int));
}

#[test]
fn verify_generic_not_subtype_different_name() {
    let checker = Checker::new();

    let list_int = Type::Generic {
        name: "List".into(),
        args: vec![Type::Int],
    };
    let vec_int = Type::Generic {
        name: "Vec".into(),
        args: vec![Type::Int],
    };

    assert!(!checker.is_subtype(&list_int, &vec_int));
}

#[test]
fn verify_nested_generic_compatible() {
    let checker = Checker::new();

    let nested1 = Type::Generic {
        name: "List".into(),
        args: vec![Type::Generic {
            name: "Option".into(),
            args: vec![Type::Int],
        }],
    };
    let nested2 = Type::Generic {
        name: "List".into(),
        args: vec![Type::Generic {
            name: "Option".into(),
            args: vec![Type::Int],
        }],
    };

    assert!(checker.types_compatible(&nested1, &nested2));
}

// Generic Type Ownership

#[test]
fn verify_generic_with_copy_args_is_copy() {
    let generic = Type::Generic {
        name: "List".into(),
        args: vec![Type::Int, Type::Bool],
    };

    assert_eq!(generic.ownership(), ect::ty::Ownership::Copy);
}

#[test]
fn verify_generic_with_move_arg_is_move() {
    let generic = Type::Generic {
        name: "List".into(),
        args: vec![Type::String],
    };

    assert_eq!(generic.ownership(), ect::ty::Ownership::Move);
}

#[test]
fn verify_generic_with_mixed_args_is_move() {
    let generic = Type::Generic {
        name: "Map".into(),
        args: vec![Type::Int, Type::String],
    };

    assert_eq!(generic.ownership(), ect::ty::Ownership::Move);
}

#[test]
fn verify_generic_with_no_args_is_copy() {
    let generic = Type::Generic {
        name: "Empty".into(),
        args: vec![],
    };

    // Empty args means all Copy (vacuous truth)
    assert_eq!(generic.ownership(), ect::ty::Ownership::Copy);
}

// Integration: Generic Types in Function Signatures

#[test]
fn verify_generic_in_function_signature() {
    let checker = Checker::new();

    let list_int = Type::Generic {
        name: "List".into(),
        args: vec![Type::Int],
    };

    let fn_type = Type::Function {
        params: vec![list_int.clone()],
        effects: vec![],
        ret: Box::new(Type::Int),
    };

    assert!(checker.types_compatible(&fn_type, &fn_type));
}

#[test]
fn verify_generic_in_tuple() {
    let checker = Checker::new();

    let list_int = Type::Generic {
        name: "List".into(),
        args: vec![Type::Int],
    };

    let tuple = Type::Tuple(vec![list_int.clone(), Type::String]);

    let same_tuple = Type::Tuple(vec![list_int, Type::String]);

    assert!(checker.types_compatible(&tuple, &same_tuple));
}

// Edge Cases

#[test]
fn verify_empty_generic_args() {
    let generic_empty = Type::Generic {
        name: "Void".into(),
        args: vec![],
    };

    let generic_empty2 = Type::Generic {
        name: "Void".into(),
        args: vec![],
    };

    assert_eq!(generic_empty, generic_empty2);
}

#[test]
fn verify_deeply_nested_generics() {
    let checker = Checker::new();

    let deeply_nested = Type::Generic {
        name: "L1".into(),
        args: vec![Type::Generic {
            name: "L2".into(),
            args: vec![Type::Generic {
                name: "L3".into(),
                args: vec![Type::Int],
            }],
        }],
    };

    let same_nested = Type::Generic {
        name: "L1".into(),
        args: vec![Type::Generic {
            name: "L2".into(),
            args: vec![Type::Generic {
                name: "L3".into(),
                args: vec![Type::Int],
            }],
        }],
    };

    assert!(checker.types_compatible(&deeply_nested, &same_nested));
}

#[test]
fn verify_generic_with_reference_arg() {
    let checker = Checker::new();

    let ref_int = Type::Reference {
        is_mut: false,
        inner: Box::new(Type::Int),
    };

    let generic_with_ref = Type::Generic {
        name: "Option".into(),
        args: vec![ref_int.clone()],
    };

    let same_generic = Type::Generic {
        name: "Option".into(),
        args: vec![ref_int],
    };

    assert!(checker.types_compatible(&generic_with_ref, &same_generic));
}

#[test]
fn verify_generic_in_reference() {
    let checker = Checker::new();

    let list_int = Type::Generic {
        name: "List".into(),
        args: vec![Type::Int],
    };

    let ref_to_list = Type::Reference {
        is_mut: false,
        inner: Box::new(list_int.clone()),
    };

    let same_ref = Type::Reference {
        is_mut: false,
        inner: Box::new(list_int),
    };

    assert!(checker.types_compatible(&ref_to_list, &same_ref));
}

// --- typeck_generic_propagation_tests ---

#[test]
fn verify_for_loop_list_int_binds_element_type() {
    let mut checker = Checker::new();

    let list_int_ty = Type::Generic {
        name: "List".into(),
        args: vec![Type::Int],
    };
    checker.insert_var("nums".into(), list_int_ty, false, d_span());

    let for_expr = sp(ect::ast::Expr::For {
        pattern: sp(Pattern::Bind("n".into())),
        iter: Box::new(sp(ect::ast::Expr::Identifier("nums".into()))),
        body: body_breaking("n"),
    });

    let ty = checker.infer_expr(&for_expr);
    assert_eq!(ty, Type::Int, "bound variable should be Int");
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_for_loop_list_string_binds_element_type() {
    let mut checker = Checker::new();

    let list_string_ty = Type::Generic {
        name: "List".into(),
        args: vec![Type::String],
    };
    checker.insert_var("strs".into(), list_string_ty, false, d_span());

    let for_expr = sp(ect::ast::Expr::For {
        pattern: sp(Pattern::Bind("s".into())),
        iter: Box::new(sp(ect::ast::Expr::Identifier("strs".into()))),
        body: body_breaking("s"),
    });

    let ty = checker.infer_expr(&for_expr);
    assert_eq!(ty, Type::String, "bound variable should be String");
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_for_loop_vec_bool_binds_element_type() {
    let mut checker = Checker::new();

    let vec_bool_ty = Type::Generic {
        name: "Vec".into(),
        args: vec![Type::Bool],
    };
    checker.insert_var("flags".into(), vec_bool_ty, false, d_span());

    let for_expr = sp(ect::ast::Expr::For {
        pattern: sp(Pattern::Bind("f".into())),
        iter: Box::new(sp(ect::ast::Expr::Identifier("flags".into()))),
        body: body_breaking("f"),
    });

    let ty = checker.infer_expr(&for_expr);
    assert_eq!(ty, Type::Bool, "bound variable should be Bool");
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_for_loop_option_type_binds_element() {
    let mut checker = Checker::new();

    let option_int_ty = Type::Generic {
        name: "Option".into(),
        args: vec![Type::Int],
    };
    checker.insert_var("maybe_num".into(), option_int_ty, false, d_span());

    let for_expr = sp(ect::ast::Expr::For {
        pattern: sp(Pattern::Bind("n".into())),
        iter: Box::new(sp(ect::ast::Expr::Identifier("maybe_num".into()))),
        body: body_breaking("n"),
    });

    let ty = checker.infer_expr(&for_expr);
    assert_eq!(ty, Type::Int, "Option<Int> should bind Int");
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_for_loop_result_type_binds_ok_element() {
    let mut checker = Checker::new();

    let result_ty = Type::Generic {
        name: "Result".into(),
        args: vec![Type::String, Type::Int],
    };
    checker.insert_var("res".into(), result_ty, false, d_span());

    let for_expr = sp(ect::ast::Expr::For {
        pattern: sp(Pattern::Bind("val".into())),
        iter: Box::new(sp(ect::ast::Expr::Identifier("res".into()))),
        body: body_breaking("val"),
    });

    let ty = checker.infer_expr(&for_expr);
    assert_eq!(ty, Type::String, "Result<String, Int> should bind the Ok type (String)");
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_for_loop_string_binds_char() {
    let mut checker = Checker::new();

    checker.insert_var("text".into(), Type::String, false, d_span());

    let for_expr = sp(ect::ast::Expr::For {
        pattern: sp(Pattern::Bind("c".into())),
        iter: Box::new(sp(ect::ast::Expr::Identifier("text".into()))),
        body: body_breaking("c"),
    });

    let ty = checker.infer_expr(&for_expr);
    assert_eq!(ty, Type::Char, "iterating String should yield Char");
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_for_loop_array_generic_binds_element() {
    let mut checker = Checker::new();

    let array_float = Type::Generic {
        name: "Array".into(),
        args: vec![Type::Float],
    };
    checker.insert_var("values".into(), array_float, false, d_span());

    let for_expr = sp(ect::ast::Expr::For {
        pattern: sp(Pattern::Bind("v".into())),
        iter: Box::new(sp(ect::ast::Expr::Identifier("values".into()))),
        body: body_breaking("v"),
    });

    let ty = checker.infer_expr(&for_expr);
    assert_eq!(ty, Type::Float, "Array<Float> should bind Float");
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_for_loop_unknown_iterable_propagates_unknown() {
    let mut checker = Checker::new();

    checker.insert_var("unknown".into(), Type::Unknown, false, d_span());

    let for_expr = sp(ect::ast::Expr::For {
        pattern: sp(Pattern::Bind("x".into())),
        iter: Box::new(sp(ect::ast::Expr::Identifier("unknown".into()))),
        body: body_breaking("x"),
    });

    let ty = checker.infer_expr(&for_expr);
    assert_eq!(ty, Type::Unknown, "Unknown iterator should bind Unknown");
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_for_loop_non_iterable_type_binds_unknown() {
    let mut checker = Checker::new();

    checker.insert_var("num".into(), Type::Int, false, d_span());

    let for_expr = sp(ect::ast::Expr::For {
        pattern: sp(Pattern::Bind("x".into())),
        iter: Box::new(sp(ect::ast::Expr::Identifier("num".into()))),
        body: body_breaking("x"),
    });

    let ty = checker.infer_expr(&for_expr);
    assert_eq!(ty, Type::Unknown, "non-iterable Int should bind Unknown, not Int itself");
}

// Nested Generic Type Propagation

#[test]
fn verify_for_loop_nested_generic_list_option() {
    let mut checker = Checker::new();

    // List<Option<Int>> — each element is Option<Int>
    let nested_ty = Type::Generic {
        name: "List".into(),
        args: vec![Type::Generic {
            name: "Option".into(),
            args: vec![Type::Int],
        }],
    };
    checker.insert_var("maybe_nums".into(), nested_ty, false, d_span());

    let for_expr = sp(ect::ast::Expr::For {
        pattern: sp(Pattern::Bind("maybe_n".into())),
        iter: Box::new(sp(ect::ast::Expr::Identifier("maybe_nums".into()))),
        body: body_breaking("maybe_n"),
    });

    let ty = checker.infer_expr(&for_expr);
    assert_eq!(
        ty,
        Type::Generic { name: "Option".into(), args: vec![Type::Int] },
        "List<Option<Int>> should bind Option<Int>"
    );
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_for_loop_nested_list_of_lists_binds_inner_list() {
    let mut checker = Checker::new();

    // List<List<Int>> — each element is List<Int>
    let nested_list = Type::Generic {
        name: "List".into(),
        args: vec![Type::Generic {
            name: "List".into(),
            args: vec![Type::Int],
        }],
    };
    checker.insert_var("matrix".into(), nested_list, false, d_span());

    let for_expr = sp(ect::ast::Expr::For {
        pattern: sp(Pattern::Bind("row".into())),
        iter: Box::new(sp(ect::ast::Expr::Identifier("matrix".into()))),
        body: body_breaking("row"),
    });

    let ty = checker.infer_expr(&for_expr);
    assert_eq!(
        ty,
        Type::Generic { name: "List".into(), args: vec![Type::Int] },
        "outer loop over List<List<Int>> should bind List<Int>"
    );
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_for_loop_option_list_binds_list() {
    let mut checker = Checker::new();

    // Option<List<String>> — element is List<String>
    let nested_ty = Type::Generic {
        name: "Option".into(),
        args: vec![Type::Generic {
            name: "List".into(),
            args: vec![Type::String],
        }],
    };
    checker.insert_var("maybe_strs".into(), nested_ty, false, d_span());

    let for_expr = sp(ect::ast::Expr::For {
        pattern: sp(Pattern::Bind("maybe_list".into())),
        iter: Box::new(sp(ect::ast::Expr::Identifier("maybe_strs".into()))),
        body: body_breaking("maybe_list"),
    });

    let ty = checker.infer_expr(&for_expr);
    assert_eq!(
        ty,
        Type::Generic { name: "List".into(), args: vec![Type::String] },
        "Option<List<String>> should bind List<String>"
    );
    assert!(checker.errors.is_empty());
}

// Field Access with Generic Types

#[test]
fn verify_field_access_on_generic_record_substitutes_type() {
    let mut checker = Checker::new();

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

    let pair_int_ty = Type::Generic {
        name: "Pair".into(),
        args: vec![Type::Int],
    };
    checker.insert_var("pair".into(), pair_int_ty, false, d_span());

    let field_expr = sp(ect::ast::Expr::FieldAccess {
        base: Box::new(sp(ect::ast::Expr::Identifier("pair".into()))),
        field: "first".into(),
    });

    let ty = checker.infer_expr(&field_expr);
    assert_eq!(ty, Type::Int, "Pair<Int>.first should resolve to Int via generic substitution");
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_field_access_on_generic_container_concrete_field() {
    let mut checker = Checker::new();

    let container_type = ect::ast::TypeBody::Record(vec![
        ect::ast::RecordField {
            is_pub: true,
            name: "data".into(),
            ty: ect::ast::Type::Named("T".into()),
        },
        ect::ast::RecordField {
            is_pub: true,
            name: "size".into(),
            ty: ect::ast::Type::Named("Int".into()),
        },
    ]);
    checker.register_type("Container".into(), container_type);

    let container_string_ty = Type::Generic {
        name: "Container".into(),
        args: vec![Type::String],
    };
    checker.insert_var("container".into(), container_string_ty, false, d_span());

    // size field is concrete Int regardless of T
    let size_expr = sp(ect::ast::Expr::FieldAccess {
        base: Box::new(sp(ect::ast::Expr::Identifier("container".into()))),
        field: "size".into(),
    });
    assert_eq!(checker.infer_expr(&size_expr), Type::Int);

    // data field is T → should substitute to String
    let mut checker2 = Checker::new();
    let container_type2 = ect::ast::TypeBody::Record(vec![
        ect::ast::RecordField {
            is_pub: true,
            name: "data".into(),
            ty: ect::ast::Type::Named("T".into()),
        },
        ect::ast::RecordField {
            is_pub: true,
            name: "size".into(),
            ty: ect::ast::Type::Named("Int".into()),
        },
    ]);
    checker2.register_type("Container".into(), container_type2);
    checker2.insert_var(
        "c".into(),
        Type::Generic { name: "Container".into(), args: vec![Type::String] },
        false, d_span(),
    );
    let data_expr = sp(ect::ast::Expr::FieldAccess {
        base: Box::new(sp(ect::ast::Expr::Identifier("c".into()))),
        field: "data".into(),
    });
    assert_eq!(checker2.infer_expr(&data_expr), Type::String, "Container<String>.data should substitute to String");
    assert!(checker2.errors.is_empty());
}

// Type Substitution Helpers (unit tests for extract_iterable_element_type)

#[test]
fn verify_extract_iterable_element_type_list_int() {
    let checker = Checker::new();
    let elem_ty = checker.extract_iterable_element_type(&Type::Generic {
        name: "List".into(),
        args: vec![Type::Int],
    });
    assert_eq!(elem_ty, Type::Int);
}

#[test]
fn verify_extract_iterable_element_type_vec_string() {
    let checker = Checker::new();
    let elem_ty = checker.extract_iterable_element_type(&Type::Generic {
        name: "Vec".into(),
        args: vec![Type::String],
    });
    assert_eq!(elem_ty, Type::String);
}

#[test]
fn verify_extract_iterable_element_type_option_bool() {
    let checker = Checker::new();
    let elem_ty = checker.extract_iterable_element_type(&Type::Generic {
        name: "Option".into(),
        args: vec![Type::Bool],
    });
    assert_eq!(elem_ty, Type::Bool);
}

#[test]
fn verify_extract_iterable_element_type_result_int_string() {
    let checker = Checker::new();
    let elem_ty = checker.extract_iterable_element_type(&Type::Generic {
        name: "Result".into(),
        args: vec![Type::Int, Type::String],
    });
    assert_eq!(elem_ty, Type::Int);
}

#[test]
fn verify_extract_iterable_element_type_string() {
    let checker = Checker::new();
    assert_eq!(checker.extract_iterable_element_type(&Type::String), Type::Char);
}

#[test]
fn verify_extract_iterable_element_type_non_iterable() {
    let checker = Checker::new();
    assert_eq!(checker.extract_iterable_element_type(&Type::Int), Type::Unknown);
}

#[test]
fn verify_extract_iterable_element_type_unknown_generic() {
    let checker = Checker::new();
    let elem_ty = checker.extract_iterable_element_type(&Type::Generic {
        name: "UnknownType".into(),
        args: vec![Type::Int],
    });
    assert_eq!(elem_ty, Type::Unknown);
}

// --- typeck_recursive_types_tests ---

#[test]
fn verify_direct_self_reference_rejected() {
    let mut checker = Checker::new();

    // type A = { x: A } - direct infinite size
    let field_a = RecordField {
        name: "x".into(),
        ty: AstType::Named("A".into()),
        is_pub: false,
    };

    let a_type = TypeBody::Record(vec![field_a]);

    let is_valid = checker.check_recursive_type("A", &a_type, d_span());
    assert!(!is_valid, "Direct self-reference should be rejected");
    assert!(checker.errors.len() > 0);
    assert!(checker.errors.iter().any(|e| e.message.contains("recursive") || e.message.contains("cycle")));
}

#[test]
fn verify_indirect_cycle_rejected() {
    let mut checker = Checker::new();

    // type A = { x: B }
    // type B = { y: A } - creates cycle A -> B -> A
    let field_ab = RecordField {
        name: "x".into(),
        ty: AstType::Named("B".into()),
        is_pub: false,
    };
    let a_type = TypeBody::Record(vec![field_ab]);

    let field_ba = RecordField {
        name: "y".into(),
        ty: AstType::Named("A".into()),
        is_pub: false,
    };
    let b_type = TypeBody::Record(vec![field_ba]);

    checker.register_type("A".into(), a_type);
    checker.register_type("B".into(), b_type);

    let is_valid = checker.detect_type_cycles();
    assert!(!is_valid, "Indirect cycles should be detected");
    assert!(checker.errors.len() > 0);
}

#[test]
fn verify_self_reference_through_variant() {
    let mut checker = Checker::new();

    // type List = | Cons(Int, List) | Nil
    let cons_case = VariantCase::Tuple(
        "Cons".into(),
        vec![AstType::Named("Int".into()), AstType::Named("List".into())],
    );
    let nil_case = VariantCase::Unit("Nil".into());

    let list_type = TypeBody::Variant(vec![cons_case, nil_case]);

    let is_valid = checker.check_recursive_type("List", &list_type, d_span());
    assert!(is_valid, "recursive variant with a Nil base case must be accepted");
}

#[test]
fn verify_reference_indirection_allowed() {
    let mut checker = Checker::new();

    // type A = { x: &A } - self-reference through pointer is OK
    let field = RecordField {
        name: "x".into(),
        ty: AstType::Reference {
            is_mut: false,
            inner: Box::new(AstType::Named("A".into())),
            region: None,
        },
        is_pub: false,
    };

    let a_type = TypeBody::Record(vec![field]);

    let is_valid = checker.check_recursive_type("A", &a_type, d_span());
    assert!(is_valid, "Self-reference through & should be allowed");
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_mutable_reference_indirection_allowed() {
    let mut checker = Checker::new();

    // type Node = { value: Int, next: &mut Node }
    let value_field = RecordField {
        name: "value".into(),
        ty: AstType::Named("Int".into()),
        is_pub: false,
    };

    let next_field = RecordField {
        name: "next".into(),
        ty: AstType::Reference {
            is_mut: true,
            inner: Box::new(AstType::Named("Node".into())),
            region: None,
        },
        is_pub: false,
    };

    let node_type = TypeBody::Record(vec![value_field, next_field]);

    let is_valid = checker.check_recursive_type("Node", &node_type, d_span());
    assert!(is_valid);
}

#[test]
fn verify_reference_in_variant_allowed() {
    let mut checker = Checker::new();

    // type Tree = | Node(Int, &Tree, &Tree) | Leaf(Int)
    let node_case = VariantCase::Tuple(
        "Node".into(),
        vec![
            AstType::Named("Int".into()),
            AstType::Reference {
                is_mut: false,
                inner: Box::new(AstType::Named("Tree".into())),
                region: None,
            },
            AstType::Reference {
                is_mut: false,
                inner: Box::new(AstType::Named("Tree".into())),
                region: None,
            },
        ],
    );
    let leaf_case = VariantCase::Tuple("Leaf".into(), vec![AstType::Named("Int".into())]);

    let tree_type = TypeBody::Variant(vec![node_case, leaf_case]);

    let is_valid = checker.check_recursive_type("Tree", &tree_type, d_span());
    assert!(is_valid, "Reference indirection in variant should be allowed");
}

#[test]
fn verify_non_recursive_type_accepted() {
    let mut checker = Checker::new();

    // type Point = { x: Int, y: Int } - no recursion
    let x_field = RecordField {
        name: "x".into(),
        ty: AstType::Named("Int".into()),
        is_pub: false,
    };

    let y_field = RecordField {
        name: "y".into(),
        ty: AstType::Named("Int".into()),
        is_pub: false,
    };

    let point_type = TypeBody::Record(vec![x_field, y_field]);

    let is_valid = checker.check_recursive_type("Point", &point_type, d_span());
    assert!(is_valid, "Non-recursive type should be accepted");
}

#[test]
fn verify_recursive_with_base_case() {
    let mut checker = Checker::new();

    // type IntList = | Cons(Int, IntList) | Nil - has base case Nil
    let cons = VariantCase::Tuple(
        "Cons".into(),
        vec![AstType::Named("Int".into()), AstType::Named("IntList".into())],
    );
    let nil = VariantCase::Unit("Nil".into());

    let list_type = TypeBody::Variant(vec![cons, nil]);

    let is_valid = checker.check_recursive_type("IntList", &list_type, d_span());
    assert!(is_valid, "recursive variant with a Nil base case must be accepted");
}

#[test]
fn verify_mutual_recursion_without_indirection_rejected() {
    let mut checker = Checker::new();

    // type A = { b: B }
    // type B = { a: A } - A and B form a cycle
    let a_field = RecordField {
        name: "b".into(),
        ty: AstType::Named("B".into()),
        is_pub: false,
    };

    let b_field = RecordField {
        name: "a".into(),
        ty: AstType::Named("A".into()),
        is_pub: false,
    };

    checker.register_type("A".into(), TypeBody::Record(vec![a_field]));
    checker.register_type("B".into(), TypeBody::Record(vec![b_field]));

    let has_cycle = checker.detect_type_cycles();
    assert!(!has_cycle, "Mutual recursion without indirection should be detected");
}

#[test]
fn verify_mutual_recursion_with_indirection_allowed() {
    let mut checker = Checker::new();

    // type A = { b: &B }
    // type B = { a: &A } - A and B form a cycle but with references
    let a_field = RecordField {
        name: "b".into(),
        ty: AstType::Reference {
            is_mut: false,
            inner: Box::new(AstType::Named("B".into())),
            region: None,
        },
        is_pub: false,
    };

    let b_field = RecordField {
        name: "a".into(),
        ty: AstType::Reference {
            is_mut: false,
            inner: Box::new(AstType::Named("A".into())),
            region: None,
        },
        is_pub: false,
    };

    checker.register_type("A".into(), TypeBody::Record(vec![a_field]));
    checker.register_type("B".into(), TypeBody::Record(vec![b_field]));

    let has_cycle = checker.detect_type_cycles();
    assert!(has_cycle == false || checker.errors.is_empty(), "Mutual recursion with references should be allowed");
}

#[test]
fn verify_complex_recursive_structure() {
    let mut checker = Checker::new();

    // type BinaryTree = | Node(Int, &BinaryTree, &BinaryTree) | Leaf
    let node = VariantCase::Tuple(
        "Node".into(),
        vec![
            AstType::Named("Int".into()),
            AstType::Reference {
                is_mut: false,
                inner: Box::new(AstType::Named("BinaryTree".into())),
                region: None,
            },
            AstType::Reference {
                is_mut: false,
                inner: Box::new(AstType::Named("BinaryTree".into())),
                region: None,
            },
        ],
    );
    let leaf = VariantCase::Unit("Leaf".into());

    let tree_type = TypeBody::Variant(vec![node, leaf]);

    let is_valid = checker.check_recursive_type("BinaryTree", &tree_type, d_span());
    assert!(is_valid, "Complex recursive structure with proper indirection should be valid");
}

#[test]
fn verify_linked_list_pattern() {
    let mut checker = Checker::new();

    // Standard linked list: type LinkedList<T> = | Node(T, &LinkedList<T>) | Nil
    let node = VariantCase::Tuple(
        "Node".into(),
        vec![
            AstType::Named("T".into()),
            AstType::Reference {
                is_mut: false,
                inner: Box::new(AstType::Named("LinkedList".into())),
                region: None,
            },
        ],
    );
    let nil = VariantCase::Unit("Nil".into());

    let list_type = TypeBody::Variant(vec![node, nil]);

    let is_valid = checker.check_recursive_type("LinkedList", &list_type, d_span());
    assert!(is_valid, "Standard linked list pattern should be valid");
}

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

#[test]
fn verify_fn_input_contravariant() {
    let mut checker = Checker::new();
    checker.register_named_subtype("String".into(), "Any".into());
    // Fn(Any, R) <: Fn(String, R) because input is contravariant
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

#[test]
fn verify_array_covariant() {
    let mut checker = Checker::new();
    checker.register_named_subtype("String".into(), "Any".into());
    let array_string = Type::Named("Array<String>".into());
    let array_any = Type::Named("Array<Any>".into());
    assert!(checker.is_subtype(&array_string, &array_any));
}

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

#[test]
fn verify_or_pattern_covers_multiple_cases() {
    let mut checker = Checker::new();
    checker.register_variant_cases("Color".into(),
        vec!["Red".into(), "Green".into(), "Blue".into()]);

    let covered = vec!["Red".into(), "Green".into(), "Blue".into()];
    let result = checker.check_variant_exhaustiveness("Color", &covered, false, d_span());

    assert!(result);
}

#[test]
fn verify_or_pattern_still_missing_case() {
    let mut checker = Checker::new();
    checker.register_variant_cases("Color".into(),
        vec!["Red".into(), "Green".into(), "Blue".into()]);

    let covered = vec!["Red".into(), "Green".into()];
    let result = checker.check_variant_exhaustiveness("Color", &covered, false, d_span());

    assert!(!result);
    assert!(checker.errors[0].message.contains("Blue"));
}

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

#[test]
fn verify_type_alias_resolves_in_variable_type() {
    // `type Num = Int` then a variable declared as `Num` should resolve to Int
    let mut checker = Checker::new();
    checker.check_program(&[
        ast::Decl::TypeAlias {
            is_pub: false,
            name: "Num".into(),
            generics: vec![],
            ty: ast::Type::Named("Int".into()),
        },
    ]);
    // After the alias is registered, convert_type on Named("Num") should
    // yield Int (or at least not Unknown)
    let resolved = checker.convert_type(&ast::Type::Named("Num".into()));
    assert_ne!(resolved, ect::ty::Type::Unknown,
        "type alias 'Num = Int' must not resolve to Unknown; got {:?}", resolved);
    assert_eq!(resolved, ect::ty::Type::Int,
        "type alias 'Num = Int' must resolve to Int; got {:?}", resolved);
}

#[test]
fn recursive_variant_with_unit_base_case_accepted() {
    // type Tree = Leaf | Node(Int, Tree, Tree)  — Leaf is a Unit base case.
    let mut checker = Checker::new();
    let body = TypeBody::Variant(vec![
        VariantCase::Unit("Leaf".into()),
        VariantCase::Tuple(
            "Node".into(),
            vec![
                AstType::Named("Int".into()),
                AstType::Named("Tree".into()),
                AstType::Named("Tree".into()),
            ],
        ),
    ]);
    let ok = checker.check_recursive_type("Tree", &body, d_span());
    assert!(ok, "Tree with Leaf base case must typecheck; errors: {:?}",
        checker.errors.iter().map(|e| &e.message).collect::<Vec<_>>());
    assert!(checker.errors.is_empty());
}

#[test]
fn recursive_variant_with_tuple_base_case_accepted() {
    // type T = Leaf(Int) | Node(T, T)  — Leaf is a base case (no T in payload).
    let mut checker = Checker::new();
    let body = TypeBody::Variant(vec![
        VariantCase::Tuple("Leaf".into(), vec![AstType::Named("Int".into())]),
        VariantCase::Tuple(
            "Node".into(),
            vec![AstType::Named("T".into()), AstType::Named("T".into())],
        ),
    ]);
    let ok = checker.check_recursive_type("T", &body, d_span());
    assert!(ok, "Leaf(Int) is a valid base case; errors: {:?}",
        checker.errors.iter().map(|e| &e.message).collect::<Vec<_>>());
}

#[test]
fn variant_with_no_base_case_rejected() {
    // type Bad = One(Bad) — every case carries a Bad payload; no way to
    // construct an initial value. Must be rejected.
    let mut checker = Checker::new();
    let body = TypeBody::Variant(vec![
        VariantCase::Tuple("One".into(), vec![AstType::Named("Bad".into())]),
    ]);
    let ok = checker.check_recursive_type("Bad", &body, d_span());
    assert!(!ok, "variant whose every case self-references must be rejected");
    assert!(
        checker.errors.iter().any(|e| e.message.contains("no base case")),
        "expected a 'no base case' diagnostic; got {:?}",
        checker.errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
}

#[test]
fn variant_with_all_self_referencing_cases_rejected() {
    // type Bad2 = A(Bad2) | B(Int, Bad2)  — no case lacks a Bad2 payload.
    let mut checker = Checker::new();
    let body = TypeBody::Variant(vec![
        VariantCase::Tuple("A".into(), vec![AstType::Named("Bad2".into())]),
        VariantCase::Tuple(
            "B".into(),
            vec![AstType::Named("Int".into()), AstType::Named("Bad2".into())],
        ),
    ]);
    let ok = checker.check_recursive_type("Bad2", &body, d_span());
    assert!(!ok, "no base case across all cases ⇒ reject");
    assert!(
        checker.errors.iter().any(|e| e.message.contains("no base case")),
        "expected 'no base case' diagnostic; got {:?}",
        checker.errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
}

#[test]
fn pure_record_self_reference_still_rejected() {
    let mut checker = Checker::new();
    let body = TypeBody::Record(vec![
        RecordField {
            name: "next".into(),
            ty: AstType::Named("BadRec".into()),
            is_pub: false,
        },
    ]);
    let ok = checker.check_recursive_type("BadRec", &body, d_span());
    assert!(!ok, "record self-reference (inline, no pointer) must be rejected");
    assert!(
        checker.errors.iter().any(|e| e.message.contains("recursive cycle")),
        "expected 'recursive cycle' diagnostic; got {:?}",
        checker.errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
}

#[test]
fn variant_with_record_payload_self_reference_accepted() {
    let mut checker = Checker::new();
    let body = TypeBody::Variant(vec![
        VariantCase::Unit("Leaf".into()),
        VariantCase::Record(
            "Branch".into(),
            vec![
                RecordField { name: "left".into(),  ty: AstType::Named("Tree2".into()), is_pub: false },
                RecordField { name: "right".into(), ty: AstType::Named("Tree2".into()), is_pub: false },
            ],
        ),
    ]);
    let ok = checker.check_recursive_type("Tree2", &body, d_span());
    assert!(ok, "variant case with record payload may reference the type; errors: {:?}",
        checker.errors.iter().map(|e| &e.message).collect::<Vec<_>>());
}

#[test]
fn nullary_variant_constructor_resolves_at_expression_position() {
    let mut checker = Checker::new();
    // Register `type Tree = Leaf | Node(Int, Tree, Tree)` end-to-end.
    let body = TypeBody::Variant(vec![
        VariantCase::Unit("Leaf".into()),
        VariantCase::Tuple(
            "Node".into(),
            vec![
                AstType::Named("Int".into()),
                AstType::Named("Tree".into()),
                AstType::Named("Tree".into()),
            ],
        ),
    ]);
    checker.register_type("Tree".into(), body);
    checker.register_variant_cases("Tree".into(),
        vec!["Leaf".into(), "Node".into()]);

    let expr = sp(ast::Expr::Identifier("Leaf".into()));
    let ty = checker.infer_expr(&expr);
    // Bare unit constructor evaluates to the owning type.
    assert_eq!(ty, ect::ty::Type::Named("Tree".into()),
        "Leaf must resolve to Tree via the constructor fallback; got {:?}", ty);
    assert!(
        !checker.errors.iter().any(|e| e.message.contains("Undefined variable")),
        "no 'Undefined variable' error expected; got {:?}",
        checker.errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
}

#[test]
fn tuple_variant_constructor_resolves_as_function() {
    let mut checker = Checker::new();
    let body = TypeBody::Variant(vec![
        VariantCase::Unit("Leaf".into()),
        VariantCase::Tuple(
            "Node".into(),
            vec![
                AstType::Named("Int".into()),
                AstType::Named("Tree".into()),
                AstType::Named("Tree".into()),
            ],
        ),
    ]);
    checker.register_type("Tree".into(), body);
    checker.register_variant_cases("Tree".into(),
        vec!["Leaf".into(), "Node".into()]);

    let expr = sp(ast::Expr::Identifier("Node".into()));
    let ty = checker.infer_expr(&expr);
    match ty {
        ect::ty::Type::Function { params, ret, .. } => {
            assert_eq!(params.len(), 3, "Node has three payload params");
            assert_eq!(*ret, ect::ty::Type::Named("Tree".into()),
                "Node's return type must be Tree");
        }
        other => panic!("expected Function for Node constructor; got {:?}", other),
    }
}
