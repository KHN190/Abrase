use ect::ast::{self, Pattern, Span, Spanned};
use ect::ty::Type;
use ect::typeck::Checker;

fn d_span() -> Span { Span::new(0, 0) }
fn sp<T>(node: T) -> Spanned<T> { Spanned { node, span: d_span() } }

// Ownership & Borrowing Tests

#[test]
fn verify_ownership_primitives_are_copy() {
    let _checker = Checker::new();
    use ect::ty::Ownership;
    assert_eq!(Type::Int.ownership(), Ownership::Copy);
    assert_eq!(Type::Bool.ownership(), Ownership::Copy);
    assert_eq!(Type::Float.ownership(), Ownership::Copy);
    assert_eq!(Type::Char.ownership(), Ownership::Copy);
    assert_eq!(Type::Unit.ownership(), Ownership::Copy);
}

#[test]
fn verify_ownership_string_is_move() {
    use ect::ty::Ownership;
    assert_eq!(Type::String.ownership(), Ownership::Move);
}

#[test]
fn verify_ownership_reference_is_copy() {
    use ect::ty::Ownership;
    let ref_ty = Type::Reference { is_mut: false, inner: Box::new(Type::String) };
    assert_eq!(ref_ty.ownership(), Ownership::Copy);
}

#[test]
fn verify_ownership_tuple_copy_all() {
    use ect::ty::Ownership;
    let tuple = Type::Tuple(vec![Type::Int, Type::Bool, Type::Float]);
    assert_eq!(tuple.ownership(), Ownership::Copy);
}

#[test]
fn verify_ownership_tuple_move_with_string() {
    use ect::ty::Ownership;
    let tuple = Type::Tuple(vec![Type::Int, Type::String]);
    assert_eq!(tuple.ownership(), Ownership::Move);
}

#[test]
fn verify_immutable_borrow_allowed() {
    let mut checker = Checker::new();
    checker.insert_var("x".into(), Type::Int, false, d_span());
    assert!(checker.try_immut_borrow("x", d_span()).is_ok());
}

#[test]
fn verify_mutable_borrow_not_allowed_on_immut_var() {
    let mut checker = Checker::new();
    checker.insert_var("x".into(), Type::Int, false, d_span());
    let result = checker.try_mut_borrow("x", d_span());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Cannot mutably borrow immutable variable"));
}

#[test]
fn verify_mutable_borrow_allowed_on_mut_var() {
    let mut checker = Checker::new();
    checker.insert_var("x".into(), Type::Int, true, d_span());
    assert!(checker.try_mut_borrow("x", d_span()).is_ok());
}

#[test]
fn verify_borrow_double_immutable_allowed() {
    let mut checker = Checker::new();
    checker.insert_var("x".into(), Type::Int, false, d_span());
    assert!(checker.try_immut_borrow("x", d_span()).is_ok());
    assert!(checker.try_immut_borrow("x", d_span()).is_ok());
}

#[test]
fn verify_borrow_immutable_then_mutable_error() {
    let mut checker = Checker::new();
    checker.insert_var("x".into(), Type::Int, true, d_span());
    assert!(checker.try_immut_borrow("x", d_span()).is_ok());
    let result = checker.try_mut_borrow("x", d_span());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Cannot mutably borrow"));
}

#[test]
fn verify_borrow_mutable_then_immutable_error() {
    let mut checker = Checker::new();
    checker.insert_var("x".into(), Type::Int, true, d_span());
    assert!(checker.try_mut_borrow("x", d_span()).is_ok());
    let result = checker.try_immut_borrow("x", d_span());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Cannot immutably borrow"));
}

#[test]
fn verify_borrow_mutable_twice_error() {
    let mut checker = Checker::new();
    checker.insert_var("x".into(), Type::Int, true, d_span());
    assert!(checker.try_mut_borrow("x", d_span()).is_ok());
    let result = checker.try_mut_borrow("x", d_span());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("mutable borrow already active"));
}

#[test]
fn verify_move_copy_type_when_using_identifier() {
    let mut checker = Checker::new();
    checker.insert_var("x".into(), Type::Int, false, d_span());
    let expr = sp(ast::Expr::Identifier("x".into()));
    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Int);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_move_move_type_when_using_identifier() {
    let mut checker = Checker::new();
    checker.insert_var("x".into(), Type::String, false, d_span());
    let expr = sp(ast::Expr::Identifier("x".into()));
    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::String);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_use_after_move_error() {
    let mut checker = Checker::new();
    checker.insert_var("x".into(), Type::String, false, d_span());

    let expr1 = sp(ast::Expr::Identifier("x".into()));
    let _ty1 = checker.infer_expr(&expr1);

    let expr2 = sp(ast::Expr::Identifier("x".into()));
    let _ty2 = checker.infer_expr(&expr2);

    assert!(!checker.errors.is_empty());
    assert!(checker.errors[0].message.contains("Use of moved value"));
}

#[test]
fn verify_reference_operation_immutable() {
    let mut checker = Checker::new();
    checker.insert_var("x".into(), Type::Int, false, d_span());
    let expr = sp(ast::Expr::Unary {
        op: ast::UnaryOp::Ref,
        right: Box::new(sp(ast::Expr::Identifier("x".into()))),
    });
    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Reference { is_mut: false, inner: Box::new(Type::Int) });
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_reference_operation_mutable() {
    let mut checker = Checker::new();
    checker.insert_var("x".into(), Type::Int, true, d_span());
    let expr = sp(ast::Expr::Unary {
        op: ast::UnaryOp::RefMut,
        right: Box::new(sp(ast::Expr::Identifier("x".into()))),
    });
    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Reference { is_mut: true, inner: Box::new(Type::Int) });
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_mutable_reference_on_immutable_error() {
    let mut checker = Checker::new();
    checker.insert_var("x".into(), Type::Int, false, d_span());
    let expr = sp(ast::Expr::Unary {
        op: ast::UnaryOp::RefMut,
        right: Box::new(sp(ast::Expr::Identifier("x".into()))),
    });
    let _ty = checker.infer_expr(&expr);
    assert!(!checker.errors.is_empty());
}

#[test]
fn verify_ownership_in_let_statement_copy() {
    let mut checker = Checker::new();
    let init_expr = sp(ast::Expr::Literal(ast::Literal::Int(42)));
    let stmt = sp(ast::Stmt::Let {
        pattern: sp(Pattern::Bind("x".into())),
        is_mut: false,
        ty: None,
        value: init_expr,
    });
    checker.check_stmt(&stmt);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_ownership_in_let_statement_move() {
    let mut checker = Checker::new();
    let init_expr = sp(ast::Expr::Literal(ast::Literal::String("hello".into())));
    let stmt = sp(ast::Stmt::Let {
        pattern: sp(Pattern::Bind("s".into())),
        is_mut: false,
        ty: None,
        value: init_expr,
    });
    checker.check_stmt(&stmt);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_copy_semantics_multiple_uses() {
    let mut checker = Checker::new();
    checker.insert_var("x".into(), Type::Int, false, d_span());

    let expr1 = sp(ast::Expr::Identifier("x".into()));
    checker.infer_expr(&expr1);

    let expr2 = sp(ast::Expr::Identifier("x".into()));
    checker.infer_expr(&expr2);

    assert!(checker.errors.is_empty());
}

#[test]
fn verify_release_borrow() {
    let mut checker = Checker::new();
    checker.insert_var("x".into(), Type::Int, false, d_span());
    checker.try_immut_borrow("x", d_span()).unwrap();
    checker.release_borrow("x");
    assert!(true);
}

#[test]
fn verify_check_ownership_method() {
    let checker = Checker::new();
    use ect::ty::Ownership;

    assert_eq!(checker.check_ownership(&Type::Int), Ownership::Copy);
    assert_eq!(checker.check_ownership(&Type::String), Ownership::Move);

    let ref_ty = Type::Reference { is_mut: false, inner: Box::new(Type::String) };
    assert_eq!(checker.check_ownership(&ref_ty), Ownership::Copy);
}

// Type Ownership Attributes Tests

#[test]
fn verify_register_type_ownership_copy() {
    let mut checker = Checker::new();
    use ect::ty::Ownership;

    checker.register_ownership("Point".into(), Ownership::Copy);

    let ownership = checker.get_type_ownership("Point");
    assert!(ownership.is_some());
    assert_eq!(ownership.unwrap(), Ownership::Copy);
}

#[test]
fn verify_register_type_ownership_move() {
    let mut checker = Checker::new();
    use ect::ty::Ownership;

    checker.register_ownership("Buffer".into(), Ownership::Move);

    let ownership = checker.get_type_ownership("Buffer");
    assert!(ownership.is_some());
    assert_eq!(ownership.unwrap(), Ownership::Move);
}

#[test]
fn verify_register_type_ownership_share() {
    let mut checker = Checker::new();
    use ect::ty::Ownership;

    checker.register_ownership("Rc".into(), Ownership::Share);

    let ownership = checker.get_type_ownership("Rc");
    assert!(ownership.is_some());
    assert_eq!(ownership.unwrap(), Ownership::Share);
}

#[test]
fn verify_infer_ownership_primitive_int() {
    let checker = Checker::new();
    use ect::ty::Ownership;

    assert_eq!(checker.infer_type_ownership("Int"), Ownership::Copy);
}

#[test]
fn verify_infer_ownership_primitive_float() {
    let checker = Checker::new();
    use ect::ty::Ownership;

    assert_eq!(checker.infer_type_ownership("Float"), Ownership::Copy);
}

#[test]
fn verify_infer_ownership_primitive_bool() {
    let checker = Checker::new();
    use ect::ty::Ownership;

    assert_eq!(checker.infer_type_ownership("Bool"), Ownership::Copy);
}

#[test]
fn verify_infer_ownership_primitive_char() {
    let checker = Checker::new();
    use ect::ty::Ownership;

    assert_eq!(checker.infer_type_ownership("Char"), Ownership::Copy);
}

#[test]
fn verify_infer_ownership_primitive_unit() {
    let checker = Checker::new();
    use ect::ty::Ownership;

    assert_eq!(checker.infer_type_ownership("Unit"), Ownership::Copy);
}

#[test]
fn verify_infer_ownership_string_default() {
    let checker = Checker::new();
    use ect::ty::Ownership;

    assert_eq!(checker.infer_type_ownership("String"), Ownership::Share);
}

#[test]
fn verify_infer_ownership_unknown_default() {
    let checker = Checker::new();
    use ect::ty::Ownership;

    assert_eq!(checker.infer_type_ownership("CustomType"), Ownership::Move);
}

#[test]
fn verify_infer_ownership_registered_type() {
    let mut checker = Checker::new();
    use ect::ty::Ownership;

    checker.register_ownership("MyType".into(), Ownership::Copy);

    assert_eq!(checker.infer_type_ownership("MyType"), Ownership::Copy);
}

#[test]
fn verify_convert_ownership_attr_copy() {
    let checker = Checker::new();
    use ect::ty::Ownership;

    let attr = Some(ast::OwnershipAttr::Copy);
    assert_eq!(checker.convert_ownership_attr(&attr), Ownership::Copy);
}

#[test]
fn verify_convert_ownership_attr_move() {
    let checker = Checker::new();
    use ect::ty::Ownership;

    let attr = Some(ast::OwnershipAttr::Move);
    assert_eq!(checker.convert_ownership_attr(&attr), Ownership::Move);
}

#[test]
fn verify_convert_ownership_attr_share() {
    let checker = Checker::new();
    use ect::ty::Ownership;

    let attr = Some(ast::OwnershipAttr::Share);
    assert_eq!(checker.convert_ownership_attr(&attr), Ownership::Share);
}

#[test]
fn verify_convert_ownership_attr_none_defaults_to_move() {
    let checker = Checker::new();
    use ect::ty::Ownership;

    let attr = None;
    assert_eq!(checker.convert_ownership_attr(&attr), Ownership::Move);
}

#[test]
fn verify_register_type_with_ownership_copy() {
    let mut checker = Checker::new();
    use ect::ty::Ownership;

    let type_body = ast::TypeBody::Record(vec![]);
    checker.register_type_with_ownership("Point".into(), Ownership::Copy, type_body);

    assert_eq!(checker.get_type_ownership("Point").unwrap(), Ownership::Copy);
    assert!(checker.get_type("Point").is_some());
}

#[test]
fn verify_register_type_with_ownership_move() {
    let mut checker = Checker::new();
    use ect::ty::Ownership;

    let type_body = ast::TypeBody::Record(vec![]);
    checker.register_type_with_ownership("Buffer".into(), Ownership::Move, type_body);

    assert_eq!(checker.get_type_ownership("Buffer").unwrap(), Ownership::Move);
    assert!(checker.get_type("Buffer").is_some());
}

#[test]
fn verify_register_type_with_ownership_share() {
    let mut checker = Checker::new();
    use ect::ty::Ownership;

    let type_body = ast::TypeBody::Variant(vec![]);
    checker.register_type_with_ownership("Rc".into(), Ownership::Share, type_body);

    assert_eq!(checker.get_type_ownership("Rc").unwrap(), Ownership::Share);
    assert!(checker.get_type("Rc").is_some());
}

#[test]
fn verify_ownership_override_primitives_not_allowed() {
    let mut checker = Checker::new();
    use ect::ty::Ownership;

    // Attempting to override Int ownership still returns Copy
    checker.register_ownership("Int".into(), Ownership::Move);
    assert_eq!(checker.infer_type_ownership("Int"), Ownership::Copy);
}

#[test]
fn verify_ownership_string_can_be_overridden_to_copy() {
    let mut checker = Checker::new();
    use ect::ty::Ownership;

    checker.register_ownership("String".into(), Ownership::Copy);
    assert_eq!(checker.get_type_ownership("String").unwrap(), Ownership::Copy);
}

#[test]
fn verify_multiple_custom_types_ownership() {
    let mut checker = Checker::new();
    use ect::ty::Ownership;

    checker.register_ownership("Type1".into(), Ownership::Copy);
    checker.register_ownership("Type2".into(), Ownership::Move);
    checker.register_ownership("Type3".into(), Ownership::Share);

    assert_eq!(checker.get_type_ownership("Type1").unwrap(), Ownership::Copy);
    assert_eq!(checker.get_type_ownership("Type2").unwrap(), Ownership::Move);
    assert_eq!(checker.get_type_ownership("Type3").unwrap(), Ownership::Share);
}

#[test]
fn verify_ownership_registry_empty_by_default() {
    let checker = Checker::new();

    assert!(checker.get_type_ownership("NonExistent").is_none());
}

#[test]
fn verify_infer_ownership_uses_registry_before_defaults() {
    let mut checker = Checker::new();
    use ect::ty::Ownership;

    // Register custom ownership for a type
    checker.register_ownership("MyString".into(), Ownership::Copy);

    // Should use registry value, not default inference
    assert_eq!(checker.infer_type_ownership("MyString"), Ownership::Copy);
}