use ect::ast::{RecordField, Type as AstType, TypeBody, VariantCase};
use ect::ty::Type;
use ect::typeck::Checker;

fn d_span() -> ect::ast::Span {
    ect::ast::Span {
        line: 0,
        col: 0,
    }
}

// Type Equivalence Tests

// Basic equivalence tests
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

// Record type equivalence tests
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

// Variant type equivalence tests
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

// Tuple variant equivalence tests
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

// Record variant equivalence tests
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

// Mixed variant types equivalence tests
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

// Unregistered types
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

// Complex nested types
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
