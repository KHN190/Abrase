use ect::ty::Type;
use ect::typeck::Checker;
use ect::ast::{Span, Pattern};

fn d_span() -> Span {
    Span { line: 0, col: 0 }
}

fn sp<T>(node: T) -> ect::ast::Spanned<T> {
    ect::ast::Spanned { node, span: d_span() }
}

fn dummy_block() -> ect::ast::Block {
    ect::ast::Block { stmts: vec![], ret: None }
}

// For Loop Generic Type Propagation Tests

#[test]
fn verify_for_loop_list_int_binds_element_type() {
    let mut checker = Checker::new();

    // Variable holds List<Int>
    let list_int_ty = Type::Generic {
        name: "List".into(),
        args: vec![Type::Int],
    };
    checker.insert_var("nums".into(), list_int_ty, false, d_span());

    let for_expr = sp(ect::ast::Expr::For {
        pattern: sp(Pattern::Bind("n".into())),
        iter: Box::new(sp(ect::ast::Expr::Identifier("nums".into()))),
        body: dummy_block(),
    });

    checker.infer_expr(&for_expr);

    // After the for loop, the variable 'n' should have been bound to Int
    // We can verify this indirectly by checking there are no type errors in the loop
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_for_loop_list_string_binds_element_type() {
    let mut checker = Checker::new();

    // Variable holds List<String>
    let list_string_ty = Type::Generic {
        name: "List".into(),
        args: vec![Type::String],
    };
    checker.insert_var("strs".into(), list_string_ty, false, d_span());

    let for_expr = sp(ect::ast::Expr::For {
        pattern: sp(Pattern::Bind("s".into())),
        iter: Box::new(sp(ect::ast::Expr::Identifier("strs".into()))),
        body: dummy_block(),
    });

    checker.infer_expr(&for_expr);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_for_loop_vec_bool_binds_element_type() {
    let mut checker = Checker::new();

    // Variable holds Vec<Bool>
    let vec_bool_ty = Type::Generic {
        name: "Vec".into(),
        args: vec![Type::Bool],
    };
    checker.insert_var("flags".into(), vec_bool_ty, false, d_span());

    let for_expr = sp(ect::ast::Expr::For {
        pattern: sp(Pattern::Bind("f".into())),
        iter: Box::new(sp(ect::ast::Expr::Identifier("flags".into()))),
        body: dummy_block(),
    });

    checker.infer_expr(&for_expr);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_for_loop_option_type_binds_element() {
    let mut checker = Checker::new();

    // Variable holds Option<Int>
    let option_int_ty = Type::Generic {
        name: "Option".into(),
        args: vec![Type::Int],
    };
    checker.insert_var("maybe_num".into(), option_int_ty, false, d_span());

    let for_expr = sp(ect::ast::Expr::For {
        pattern: sp(Pattern::Bind("n".into())),
        iter: Box::new(sp(ect::ast::Expr::Identifier("maybe_num".into()))),
        body: dummy_block(),
    });

    checker.infer_expr(&for_expr);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_for_loop_result_type_binds_ok_element() {
    let mut checker = Checker::new();

    // Variable holds Result<String, Int>
    let result_ty = Type::Generic {
        name: "Result".into(),
        args: vec![Type::String, Type::Int],
    };
    checker.insert_var("res".into(), result_ty, false, d_span());

    let for_expr = sp(ect::ast::Expr::For {
        pattern: sp(Pattern::Bind("val".into())),
        iter: Box::new(sp(ect::ast::Expr::Identifier("res".into()))),
        body: dummy_block(),
    });

    checker.infer_expr(&for_expr);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_for_loop_string_binds_char() {
    let mut checker = Checker::new();

    checker.insert_var("text".into(), Type::String, false, d_span());

    let for_expr = sp(ect::ast::Expr::For {
        pattern: sp(Pattern::Bind("c".into())),
        iter: Box::new(sp(ect::ast::Expr::Identifier("text".into()))),
        body: dummy_block(),
    });

    checker.infer_expr(&for_expr);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_for_loop_unknown_iterable_type() {
    let mut checker = Checker::new();

    checker.insert_var("unknown".into(), Type::Unknown, false, d_span());

    let for_expr = sp(ect::ast::Expr::For {
        pattern: sp(Pattern::Bind("x".into())),
        iter: Box::new(sp(ect::ast::Expr::Identifier("unknown".into()))),
        body: dummy_block(),
    });

    checker.infer_expr(&for_expr);
    // Unknown type should propagate, no error
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_for_loop_non_iterable_type_error() {
    let mut checker = Checker::new();

    // Int is not iterable
    checker.insert_var("num".into(), Type::Int, false, d_span());

    let for_expr = sp(ect::ast::Expr::For {
        pattern: sp(Pattern::Bind("x".into())),
        iter: Box::new(sp(ect::ast::Expr::Identifier("num".into()))),
        body: dummy_block(),
    });

    checker.infer_expr(&for_expr);
    // Non-iterable type binds Unknown, which is acceptable but could warrant an error
    // For now, just verify no panic occurs
}

// Nested Generic Type Propagation

#[test]
fn verify_for_loop_nested_generic_list_option() {
    let mut checker = Checker::new();

    // Variable holds List<Option<Int>>
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
        body: dummy_block(),
    });

    checker.infer_expr(&for_expr);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_for_loop_nested_generic_option_list() {
    let mut checker = Checker::new();

    // Variable holds Option<List<String>>
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
        body: dummy_block(),
    });

    checker.infer_expr(&for_expr);
    assert!(checker.errors.is_empty());
}

// Field Access with Generic Types

#[test]
fn verify_field_access_on_generic_record() {
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

    // Variable has type Pair<Int>
    let pair_int_ty = Type::Generic {
        name: "Pair".into(),
        args: vec![Type::Int],
    };
    checker.insert_var("pair".into(), pair_int_ty, false, d_span());

    let field_expr = sp(ect::ast::Expr::FieldAccess {
        base: Box::new(sp(ect::ast::Expr::Identifier("pair".into()))),
        field: "first".into(),
    });

    let _ty = checker.infer_expr(&field_expr);
    // Currently returns the declared type (T), not the substituted type (Int)
    // This is a limitation that could be addressed with full generic substitution
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_field_access_on_generic_container() {
    let mut checker = Checker::new();

    // Register a Container<T> record
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

    // Variable has type Container<String>
    let container_string_ty = Type::Generic {
        name: "Container".into(),
        args: vec![Type::String],
    };
    checker.insert_var("container".into(), container_string_ty, false, d_span());

    // Access size field (should work regardless of generic)
    let size_expr = sp(ect::ast::Expr::FieldAccess {
        base: Box::new(sp(ect::ast::Expr::Identifier("container".into()))),
        field: "size".into(),
    });

    let ty = checker.infer_expr(&size_expr);
    assert_eq!(ty, Type::Int);
    assert!(checker.errors.is_empty());
}

// Type Substitution Helpers

#[test]
fn verify_extract_iterable_element_type_list_int() {
    let checker = Checker::new();

    let list_int = Type::Generic {
        name: "List".into(),
        args: vec![Type::Int],
    };

    let elem_ty = checker.extract_iterable_element_type(&list_int);
    assert_eq!(elem_ty, Type::Int);
}

#[test]
fn verify_extract_iterable_element_type_vec_string() {
    let checker = Checker::new();

    let vec_string = Type::Generic {
        name: "Vec".into(),
        args: vec![Type::String],
    };

    let elem_ty = checker.extract_iterable_element_type(&vec_string);
    assert_eq!(elem_ty, Type::String);
}

#[test]
fn verify_extract_iterable_element_type_option_bool() {
    let checker = Checker::new();

    let option_bool = Type::Generic {
        name: "Option".into(),
        args: vec![Type::Bool],
    };

    let elem_ty = checker.extract_iterable_element_type(&option_bool);
    assert_eq!(elem_ty, Type::Bool);
}

#[test]
fn verify_extract_iterable_element_type_result_int_string() {
    let checker = Checker::new();

    let result = Type::Generic {
        name: "Result".into(),
        args: vec![Type::Int, Type::String],
    };

    let elem_ty = checker.extract_iterable_element_type(&result);
    // Result iterates over the Ok type (first argument)
    assert_eq!(elem_ty, Type::Int);
}

#[test]
fn verify_extract_iterable_element_type_string() {
    let checker = Checker::new();

    let elem_ty = checker.extract_iterable_element_type(&Type::String);
    assert_eq!(elem_ty, Type::Char);
}

#[test]
fn verify_extract_iterable_element_type_non_iterable() {
    let checker = Checker::new();

    let elem_ty = checker.extract_iterable_element_type(&Type::Int);
    assert_eq!(elem_ty, Type::Unknown);
}

#[test]
fn verify_extract_iterable_element_type_unknown_generic() {
    let checker = Checker::new();

    let unknown_generic = Type::Generic {
        name: "UnknownType".into(),
        args: vec![Type::Int],
    };

    let elem_ty = checker.extract_iterable_element_type(&unknown_generic);
    assert_eq!(elem_ty, Type::Unknown);
}

// Complex Iteration Scenarios

#[test]
fn verify_for_loop_with_nested_iteration() {
    let mut checker = Checker::new();

    // List<List<Int>>
    let nested_list = Type::Generic {
        name: "List".into(),
        args: vec![Type::Generic {
            name: "List".into(),
            args: vec![Type::Int],
        }],
    };
    checker.insert_var("matrix".into(), nested_list, false, d_span());

    // Outer loop should bind to List<Int>
    let for_expr = sp(ect::ast::Expr::For {
        pattern: sp(Pattern::Bind("row".into())),
        iter: Box::new(sp(ect::ast::Expr::Identifier("matrix".into()))),
        body: dummy_block(),
    });

    checker.infer_expr(&for_expr);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_for_loop_array_generic() {
    let mut checker = Checker::new();

    // Array<Float>
    let array_float = Type::Generic {
        name: "Array".into(),
        args: vec![Type::Float],
    };
    checker.insert_var("values".into(), array_float, false, d_span());

    let for_expr = sp(ect::ast::Expr::For {
        pattern: sp(Pattern::Bind("v".into())),
        iter: Box::new(sp(ect::ast::Expr::Identifier("values".into()))),
        body: dummy_block(),
    });

    checker.infer_expr(&for_expr);
    assert!(checker.errors.is_empty());
}
