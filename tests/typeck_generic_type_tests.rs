use ect::ty::Type;
use ect::typeck::Checker;

// Generic Type Construction & Conversion

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
    let mut checker = Checker::new();

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
    let mut checker = Checker::new();

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
    let mut checker = Checker::new();

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
    let mut checker = Checker::new();

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
    let mut checker = Checker::new();

    let list_int = Type::Generic {
        name: "List".into(),
        args: vec![Type::Int],
    };

    assert!(checker.types_compatible(&list_int, &list_int));
}

#[test]
fn verify_different_generic_names_not_compatible() {
    let mut checker = Checker::new();

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
    let mut checker = Checker::new();

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
    let mut checker = Checker::new();

    let list_int = Type::Generic {
        name: "List".into(),
        args: vec![Type::Int],
    };

    assert!(checker.types_compatible(&list_int, &Type::Unknown));
    assert!(checker.types_compatible(&Type::Unknown, &list_int));
}

#[test]
fn verify_generic_with_different_arg_count_not_compatible() {
    let mut checker = Checker::new();

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
    let mut checker = Checker::new();

    let list_int = Type::Generic {
        name: "List".into(),
        args: vec![Type::Int],
    };

    assert!(checker.is_subtype(&list_int, &list_int));
}

#[test]
fn verify_generic_not_subtype_different_name() {
    let mut checker = Checker::new();

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
    let mut checker = Checker::new();

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
    let mut checker = Checker::new();

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
    let mut checker = Checker::new();

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
    let mut checker = Checker::new();

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
    let mut checker = Checker::new();

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
    let mut checker = Checker::new();

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
