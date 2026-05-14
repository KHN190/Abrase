use ect::ty::Type;
use ect::typeck::Checker;
use ect::ast::{Span, Pattern};

fn d_span() -> Span {
    Span { line: 0, col: 0 }
}

fn sp<T>(node: T) -> ect::ast::Spanned<T> {
    ect::ast::Spanned { node, span: d_span() }
}

/// Build a block whose return expression is the given identifier.
/// `infer_block` will return that identifier's type, letting us assert
/// the exact type the loop body sees for the bound variable.
fn body_returning(var: &str) -> ect::ast::Block {
    ect::ast::Block {
        stmts: vec![],
        ret: Some(Box::new(sp(ect::ast::Expr::Identifier(var.into())))),
    }
}

// For Loop Generic Type Propagation Tests

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
        body: body_returning("n"),
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
        body: body_returning("s"),
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
        body: body_returning("f"),
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
        body: body_returning("n"),
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
        body: body_returning("val"),
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
        body: body_returning("c"),
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
        body: body_returning("v"),
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
        body: body_returning("x"),
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
        body: body_returning("x"),
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
        body: body_returning("maybe_n"),
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
        body: body_returning("row"),
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
        body: body_returning("maybe_list"),
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
