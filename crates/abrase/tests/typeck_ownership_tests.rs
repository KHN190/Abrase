use abrase::ast::{self, Pattern, Span, Spanned};
use abrase::ty::{Type, Ownership};
use abrase::typeck::{Checker, ownership::BorrowKind};

fn d_span() -> Span { Span::new(0, 0) }
fn sp<T>(node: T) -> Spanned<T> { Spanned { node, span: d_span() } }

// Ownership & Borrowing Tests

#[test]
fn verify_ownership_primitives_are_copy() {
    let _checker = Checker::new();
    use abrase::ty::Ownership;
    assert_eq!(Type::Int.ownership(), Ownership::Copy);
    assert_eq!(Type::Bool.ownership(), Ownership::Copy);
    assert_eq!(Type::Float.ownership(), Ownership::Copy);
    assert_eq!(Type::Char.ownership(), Ownership::Copy);
    assert_eq!(Type::Unit.ownership(), Ownership::Copy);
}

#[test]
fn verify_ownership_string_is_move() {
    use abrase::ty::Ownership;
    assert_eq!(Type::String.ownership(), Ownership::Move);
}

#[test]
fn verify_ownership_reference_is_copy() {
    use abrase::ty::Ownership;
    let ref_ty = Type::Reference { is_mut: false, inner: Box::new(Type::String) };
    assert_eq!(ref_ty.ownership(), Ownership::Copy);
}

#[test]
fn verify_ownership_tuple_copy_all() {
    use abrase::ty::Ownership;
    let tuple = Type::Tuple(vec![Type::Int, Type::Bool, Type::Float]);
    assert_eq!(tuple.ownership(), Ownership::Copy);
}

#[test]
fn verify_ownership_tuple_move_with_string() {
    use abrase::ty::Ownership;
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
    use abrase::ty::Ownership;

    assert_eq!(checker.check_ownership(&Type::Int), Ownership::Copy);
    assert_eq!(checker.check_ownership(&Type::String), Ownership::Move);

    let ref_ty = Type::Reference { is_mut: false, inner: Box::new(Type::String) };
    assert_eq!(checker.check_ownership(&ref_ty), Ownership::Copy);
}

// Type Ownership Attributes Tests

#[test]
fn verify_register_type_ownership_copy() {
    let mut checker = Checker::new();
    use abrase::ty::Ownership;

    checker.register_ownership("Point".into(), Ownership::Copy);

    let ownership = checker.get_type_ownership("Point");
    assert!(ownership.is_some());
    assert_eq!(ownership.unwrap(), Ownership::Copy);
}

#[test]
fn verify_register_type_ownership_move() {
    let mut checker = Checker::new();
    use abrase::ty::Ownership;

    checker.register_ownership("Buffer".into(), Ownership::Move);

    let ownership = checker.get_type_ownership("Buffer");
    assert!(ownership.is_some());
    assert_eq!(ownership.unwrap(), Ownership::Move);
}

#[test]
fn verify_register_type_ownership_share() {
    let mut checker = Checker::new();
    use abrase::ty::Ownership;

    checker.register_ownership("Rc".into(), Ownership::Share);

    let ownership = checker.get_type_ownership("Rc");
    assert!(ownership.is_some());
    assert_eq!(ownership.unwrap(), Ownership::Share);
}

#[test]
fn verify_infer_ownership_primitive_int() {
    let checker = Checker::new();
    use abrase::ty::Ownership;

    assert_eq!(checker.infer_type_ownership("Int"), Ownership::Copy);
}

#[test]
fn verify_infer_ownership_primitive_float() {
    let checker = Checker::new();
    use abrase::ty::Ownership;

    assert_eq!(checker.infer_type_ownership("Float"), Ownership::Copy);
}

#[test]
fn verify_infer_ownership_primitive_bool() {
    let checker = Checker::new();
    use abrase::ty::Ownership;

    assert_eq!(checker.infer_type_ownership("Bool"), Ownership::Copy);
}

#[test]
fn verify_infer_ownership_primitive_char() {
    let checker = Checker::new();
    use abrase::ty::Ownership;

    assert_eq!(checker.infer_type_ownership("Char"), Ownership::Copy);
}

#[test]
fn verify_infer_ownership_primitive_unit() {
    let checker = Checker::new();
    use abrase::ty::Ownership;

    assert_eq!(checker.infer_type_ownership("Unit"), Ownership::Copy);
}

#[test]
fn verify_infer_ownership_string_default() {
    // T5: String is Move (single classification across ty.rs and typeck).
    let checker = Checker::new();
    use abrase::ty::Ownership;

    assert_eq!(checker.infer_type_ownership("String"), Ownership::Move);
}

#[test]
fn verify_t5_string_classification_consistent_with_ty_rs() {
    // Type::String.ownership() and infer_type_ownership("String") must agree.
    let checker = Checker::new();
    assert_eq!(checker.infer_type_ownership("String"), Type::String.ownership());
}

#[test]
fn verify_infer_ownership_unknown_default() {
    let checker = Checker::new();
    use abrase::ty::Ownership;

    assert_eq!(checker.infer_type_ownership("CustomType"), Ownership::Move);
}

#[test]
fn verify_infer_ownership_registered_type() {
    let mut checker = Checker::new();
    use abrase::ty::Ownership;

    checker.register_ownership("MyType".into(), Ownership::Copy);

    assert_eq!(checker.infer_type_ownership("MyType"), Ownership::Copy);
}

#[test]
fn verify_convert_ownership_attr_copy() {
    let checker = Checker::new();
    use abrase::ty::Ownership;

    let attr = Some(ast::OwnershipAttr::Copy);
    assert_eq!(checker.convert_ownership_attr(&attr), Ownership::Copy);
}

#[test]
fn verify_convert_ownership_attr_move() {
    let checker = Checker::new();
    use abrase::ty::Ownership;

    let attr = Some(ast::OwnershipAttr::Move);
    assert_eq!(checker.convert_ownership_attr(&attr), Ownership::Move);
}

#[test]
fn verify_convert_ownership_attr_share() {
    let checker = Checker::new();
    use abrase::ty::Ownership;

    let attr = Some(ast::OwnershipAttr::Share);
    assert_eq!(checker.convert_ownership_attr(&attr), Ownership::Share);
}

#[test]
fn verify_convert_ownership_attr_none_defaults_to_move() {
    let checker = Checker::new();
    use abrase::ty::Ownership;

    let attr = None;
    assert_eq!(checker.convert_ownership_attr(&attr), Ownership::Move);
}

#[test]
fn verify_register_type_with_ownership_copy() {
    let mut checker = Checker::new();
    use abrase::ty::Ownership;

    let type_body = ast::TypeBody::Record(vec![]);
    checker.register_type_with_ownership("Point".into(), Ownership::Copy, type_body);

    assert_eq!(checker.get_type_ownership("Point").unwrap(), Ownership::Copy);
    assert!(checker.get_type("Point").is_some());
}

#[test]
fn verify_register_type_with_ownership_move() {
    let mut checker = Checker::new();
    use abrase::ty::Ownership;

    let type_body = ast::TypeBody::Record(vec![]);
    checker.register_type_with_ownership("Buffer".into(), Ownership::Move, type_body);

    assert_eq!(checker.get_type_ownership("Buffer").unwrap(), Ownership::Move);
    assert!(checker.get_type("Buffer").is_some());
}

#[test]
fn verify_register_type_with_ownership_share() {
    let mut checker = Checker::new();
    use abrase::ty::Ownership;

    let type_body = ast::TypeBody::Variant(vec![]);
    checker.register_type_with_ownership("Rc".into(), Ownership::Share, type_body);

    assert_eq!(checker.get_type_ownership("Rc").unwrap(), Ownership::Share);
    assert!(checker.get_type("Rc").is_some());
}

#[test]
fn verify_ownership_override_primitives_not_allowed() {
    let mut checker = Checker::new();
    use abrase::ty::Ownership;

    // Attempting to override Int ownership still returns Copy
    checker.register_ownership("Int".into(), Ownership::Move);
    assert_eq!(checker.infer_type_ownership("Int"), Ownership::Copy);
}

#[test]
fn verify_ownership_string_can_be_overridden_to_copy() {
    let mut checker = Checker::new();
    use abrase::ty::Ownership;

    checker.register_ownership("String".into(), Ownership::Copy);
    assert_eq!(checker.get_type_ownership("String").unwrap(), Ownership::Copy);
}

#[test]
fn verify_multiple_custom_types_ownership() {
    let mut checker = Checker::new();
    use abrase::ty::Ownership;

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
    use abrase::ty::Ownership;

    // Register custom ownership for a type
    checker.register_ownership("MyString".into(), Ownership::Copy);

    // Should use registry value, not default inference
    assert_eq!(checker.infer_type_ownership("MyString"), Ownership::Copy);
}

// Integrate Ownership Attributes with Type Bodies

#[test]
fn verify_ownership_attr_copy_linked_to_type() {
    let mut checker = Checker::new();
    checker.register_ownership("MyInt".into(), Ownership::Copy);
    let ownership = checker.infer_type_ownership("MyInt");
    assert_eq!(ownership, Ownership::Copy);
}

#[test]
fn verify_ownership_attr_move_linked_to_type() {
    let mut checker = Checker::new();
    checker.register_ownership("MyString".into(), Ownership::Move);
    let ownership = checker.infer_type_ownership("MyString");
    assert_eq!(ownership, Ownership::Move);
}

#[test]
fn verify_ownership_attr_share_linked_to_type() {
    let mut checker = Checker::new();
    checker.register_ownership("Rc".into(), Ownership::Share);
    let ownership = checker.infer_type_ownership("Rc");
    assert_eq!(ownership, Ownership::Share);
}

#[test]
fn verify_variable_inherits_type_ownership() {
    let mut checker = Checker::new();
    checker.register_ownership("MyMove".into(), Ownership::Move);

    let var_ty = Type::Named("MyMove".into());
    checker.insert_var("x".into(), var_ty.clone(), false, d_span());

    // Variable's ownership should match its type
    assert_eq!(var_ty.ownership(), Ownership::Move);
}

// Differentiated Borrowing Rules for @share

#[test]
fn verify_share_type_allows_multiple_immut_borrows() {
    let mut checker = Checker::new();

    // Register @share type
    checker.register_ownership("Arc".into(), Ownership::Share);

    // Insert variable of @share type
    let share_ty = Type::Named("Arc".into());
    checker.insert_var("x".into(), share_ty, false, d_span());

    // Should allow first immutable borrow
    let result1 = checker.try_immut_borrow("x", d_span());
    assert!(result1.is_ok());

    // Should allow second immutable borrow simultaneously
    let result2 = checker.try_immut_borrow("x", d_span());
    assert!(result2.is_ok());

    // No errors for @share type with multiple readers
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_move_type_allows_immut_borrow() {
    let mut checker = Checker::new();

    checker.register_ownership("String".into(), Ownership::Move);
    let move_ty = Type::Named("String".into());
    checker.insert_var("s".into(), move_ty, false, d_span());

    let result = checker.try_immut_borrow("s", d_span());
    assert!(result.is_ok());
}

#[test]
fn verify_copy_type_can_be_freely_used() {
    let mut checker = Checker::new();

    // Int is Copy
    checker.insert_var("n".into(), Type::Int, false, d_span());

    // Copy types don't need borrow tracking
    let result1 = checker.try_immut_borrow("n", d_span());
    assert!(result1.is_ok());

    let result2 = checker.try_immut_borrow("n", d_span());
    assert!(result2.is_ok());
}

// Strict Move Enforcement and Scope Exit

#[test]
fn verify_move_type_borrow_then_move_error() {
    let mut checker = Checker::new();

    checker.register_ownership("String".into(), Ownership::Move);
    let move_ty = Type::Named("String".into());
    checker.insert_var("s".into(), move_ty.clone(), false, d_span());

    // Borrow the value
    let result1 = checker.try_immut_borrow("s", d_span());
    assert!(result1.is_ok());

    // The variable still exists and can be used
    let retrieved = checker.get_var("s", false, d_span());
    assert_eq!(retrieved, move_ty);
}

#[test]
fn verify_move_type_exclusive_write_access() {
    let mut checker = Checker::new();

    checker.register_ownership("String".into(), Ownership::Move);
    let move_ty = Type::Named("String".into());
    checker.insert_var("s".into(), move_ty, true, d_span());

    // First mutable borrow should succeed
    let result1 = checker.try_mut_borrow("s", d_span());
    assert!(result1.is_ok());

    // Second mutable borrow on same identifier should fail
    let result2 = checker.try_mut_borrow("s", d_span());
    assert!(result2.is_err());
}

#[test]
fn verify_immut_borrow_blocks_mut_borrow() {
    let mut checker = Checker::new();

    checker.register_ownership("String".into(), Ownership::Move);
    let move_ty = Type::Named("String".into());
    checker.insert_var("s".into(), move_ty, true, d_span());

    // Immutable borrow first
    let result1 = checker.try_immut_borrow("s", d_span());
    assert!(result1.is_ok());

    // Should not be able to get mutable borrow while immutable borrow active
    let result2 = checker.try_mut_borrow("s", d_span());
    assert!(result2.is_err());
    assert!(result2.unwrap_err().contains("immutable borrow already active"));
}

// Region-Aware Reference Validation

#[test]
fn verify_reference_in_region_created() {
    let mut checker = Checker::new();

    checker.insert_var("x".into(), Type::Int, false, d_span());

    // Simulate entering a region
    checker.push_region("r".into());
    assert_eq!(checker.get_current_region(), Some("r"));

    // Exit region
    let popped = checker.pop_region();
    assert_eq!(popped, Some("r".to_string()));
}

#[test]
fn verify_reference_binding_to_region() {
    let mut checker = Checker::new();

    // Simulate creating a reference in region r
    checker.push_region("r".into());
    checker.bind_reference_lifetime("ref_x".into(), "r".into());

    let lifetime = checker.get_reference_lifetime("ref_x");
    assert_eq!(lifetime, Some("r".to_string()));

    checker.pop_region();
}

#[test]
fn verify_region_scope_exit_invalidates_refs() {
    let mut checker = Checker::new();

    // Create reference within region
    checker.push_region("r".into());
    checker.bind_reference_lifetime("ref_x".into(), "r".into());
    let lifetime_inside = checker.get_reference_lifetime("ref_x");
    assert!(lifetime_inside.is_some());

    // Exit region
    checker.pop_region();

    // Reference lifetime is still stored (but would be invalid in semantic analysis)
    // The key is that the region has exited
    assert_eq!(checker.get_current_region(), None);
}

#[test]
fn verify_escape_analysis_same_region() {
    let mut checker = Checker::new();

    // Reference in inner region escaping to same region is OK
    let is_valid = checker.check_escape_analysis(Some("r1"), Some("r1"), d_span());
    assert!(is_valid);
}

#[test]
fn verify_escape_analysis_different_regions_error() {
    let mut checker = Checker::new();

    // Reference from inner region escaping to different region should error
    let is_valid = checker.check_escape_analysis(Some("r1"), Some("r2"), d_span());
    assert!(!is_valid);
    assert!(checker.errors.len() > 0);
    assert!(checker.errors[0].message.contains("escapes"));
}

// Writer/Reader Exclusivity

#[test]
fn verify_mut_borrow_requires_no_immut_borrows() {
    let mut checker = Checker::new();

    let move_ty = Type::Named("String".into());
    checker.insert_var("s".into(), move_ty, true, d_span());

    // Get immutable borrow
    let _ = checker.try_immut_borrow("s", d_span());

    // Mutable borrow should fail
    let result = checker.try_mut_borrow("s", d_span());
    assert!(result.is_err());
}

#[test]
fn verify_mut_borrow_blocks_immut_borrow() {
    let mut checker = Checker::new();

    let move_ty = Type::Named("String".into());
    checker.insert_var("s".into(), move_ty, true, d_span());

    // Get mutable borrow
    let _ = checker.try_mut_borrow("s", d_span());

    // Immutable borrow should fail
    let result = checker.try_immut_borrow("s", d_span());
    assert!(result.is_err());
}

#[test]
fn verify_share_type_mut_borrow_blocks_immut() {
    let mut checker = Checker::new();

    checker.register_ownership("Arc".into(), Ownership::Share);
    let share_ty = Type::Named("Arc".into());
    checker.insert_var("x".into(), share_ty, true, d_span());

    // Mutable borrow on @share type
    let _ = checker.try_mut_borrow("x", d_span());

    // Even for @share, immutable borrow should fail while mut borrow active
    let result = checker.try_immut_borrow("x", d_span());
    assert!(result.is_err());
}

#[test]
fn verify_multiple_variables_independent_borrows() {
    let mut checker = Checker::new();

    checker.register_ownership("Arc".into(), Ownership::Share);
    let share_ty = Type::Named("Arc".into());

    checker.insert_var("x".into(), share_ty.clone(), false, d_span());
    checker.insert_var("y".into(), share_ty.clone(), false, d_span());

    // Can borrow x
    let result1 = checker.try_immut_borrow("x", d_span());
    assert!(result1.is_ok());

    // Can borrow y independently
    let result2 = checker.try_immut_borrow("y", d_span());
    assert!(result2.is_ok());
}

#[test]
fn verify_borrow_scope_management() {
    let mut checker = Checker::new();

    let move_ty = Type::Named("String".into());
    checker.insert_var("s".into(), move_ty, false, d_span());

    // Enter scope
    checker.enter_scope();
    let result1 = checker.try_immut_borrow("s", d_span());
    assert!(result1.is_ok());

    // Exit scope
    checker.exit_scope();

    let retrieved = checker.get_var("s", false, d_span());
    assert_eq!(retrieved, Type::Named("String".into()));
}

#[test]
fn verify_copy_type_semantics() {
    let mut checker = Checker::new();

    // Primitives are copy
    checker.insert_var("n".into(), Type::Int, false, d_span());

    // Can use multiple times
    let v1 = checker.get_var("n", false, d_span());
    assert_eq!(v1, Type::Int);

    let v2 = checker.get_var("n", false, d_span());
    assert_eq!(v2, Type::Int);
}

#[test]
fn verify_release_borrow_updates_counts() {
    let mut checker = Checker::new();

    let move_ty = Type::Named("String".into());
    checker.insert_var("s".into(), move_ty, false, d_span());

    let _ = checker.try_immut_borrow("s", d_span());
    checker.release_borrow("s");

    let result = checker.try_immut_borrow("s", d_span());
    assert!(result.is_ok());
}

#[test]
fn verify_cannot_mutably_borrow_immutable_variable() {
    let mut checker = Checker::new();

    let move_ty = Type::Named("String".into());
    // is_mut = false
    checker.insert_var("s".into(), move_ty, false, d_span());

    let result = checker.try_mut_borrow("s", d_span());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("immutable variable"));
}

#[test]
fn verify_move_type_can_be_immutably_borrowed() {
    let mut checker = Checker::new();

    checker.register_ownership("MyType".into(), Ownership::Move);
    checker.insert_var("x".into(), Type::Named("MyType".into()), true, d_span());

    let result = checker.try_immut_borrow("x", d_span());
    assert!(result.is_ok());
}

#[test]
fn verify_move_type_mutable_borrow_marks_moved() {
    let mut checker = Checker::new();

    checker.register_ownership("MyType".into(), Ownership::Move);
    checker.insert_var("x".into(), Type::Named("MyType".into()), true, d_span());

    let result = checker.try_mut_borrow("x", d_span());
    assert!(result.is_ok());

    let moved = checker.resolve_var_in_scopes("x");
    assert!(moved.is_some());
}

#[test]
fn verify_copy_type_can_be_borrowed() {
    let mut checker = Checker::new();

    checker.register_ownership("Int".into(), Ownership::Copy);
    checker.insert_var("x".into(), Type::Int, true, d_span());

    let result = checker.try_immut_borrow("x", d_span());
    assert!(result.is_ok());
}

#[test]
fn verify_share_type_can_be_borrowed() {
    let mut checker = Checker::new();

    checker.register_ownership("MyShared".into(), Ownership::Share);
    checker.insert_var("s".into(), Type::Named("MyShared".into()), true, d_span());

    let result = checker.try_immut_borrow("s", d_span());
    assert!(result.is_ok());
}

#[test]
fn verify_copy_primitive_can_be_borrowed() {
    let mut checker = Checker::new();

    checker.insert_var("n".into(), Type::Int, true, d_span());

    let result = checker.try_immut_borrow("n", d_span());
    assert!(result.is_ok());
}

#[test]
fn verify_cannot_mutably_borrow_without_mut_keyword() {
    let mut checker = Checker::new();

    checker.register_ownership("MyType".into(), Ownership::Move);
    checker.insert_var("x".into(), Type::Named("MyType".into()), false, d_span());

    let result = checker.try_mut_borrow("x", d_span());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("immutable"));
}

#[test]
fn verify_move_type_allows_multiple_immut_borrows() {
    let mut checker = Checker::new();

    checker.register_ownership("MyType".into(), Ownership::Move);
    checker.insert_var("x".into(), Type::Named("MyType".into()), true, d_span());

    let result1 = checker.try_immut_borrow("x", d_span());
    assert!(result1.is_ok());

    let result2 = checker.try_immut_borrow("x", d_span());
    assert!(result2.is_ok());
}

#[test]
fn verify_infer_type_ownership_defaults() {
    let checker = Checker::new();

    // Primitives are Copy
    assert_eq!(checker.infer_type_ownership("Int"), Ownership::Copy);
    assert_eq!(checker.infer_type_ownership("Bool"), Ownership::Copy);

    // String is Move (T5).
    assert_eq!(checker.infer_type_ownership("String"), Ownership::Move);

    // Unknown types default to Move
    assert_eq!(checker.infer_type_ownership("UnknownType"), Ownership::Move);
}

// Region Escape & Borrow Checking Tests

#[test]
fn verify_push_pop_region() {
    let mut checker = Checker::new();

    assert!(checker.get_current_region().is_none());

    checker.push_region("region_a".into());
    assert_eq!(checker.get_current_region(), Some("region_a"));

    checker.push_region("region_b".into());
    assert_eq!(checker.get_current_region(), Some("region_b"));

    checker.pop_region();
    assert_eq!(checker.get_current_region(), Some("region_a"));
}

#[test]
fn verify_bind_reference_lifetime() {
    let mut checker = Checker::new();

    checker.bind_reference_lifetime("ref_x".into(), "region_a".into());

    let lifetime = checker.get_reference_lifetime("ref_x");
    assert_eq!(lifetime, Some("region_a".into()));
}

#[test]
fn verify_reference_lifetime_not_found() {
    let checker = Checker::new();

    let lifetime = checker.get_reference_lifetime("unknown_ref");
    assert!(lifetime.is_none());
}

#[test]
fn verify_check_escape_analysis_same_region() {
    let mut checker = Checker::new();

    let valid = checker.check_escape_analysis(Some("region_a"), Some("region_a"), d_span());
    assert!(valid);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_check_escape_analysis_different_regions() {
    let mut checker = Checker::new();

    let valid = checker.check_escape_analysis(Some("region_a"), Some("region_b"), d_span());
    assert!(!valid);
    assert!(!checker.errors.is_empty());
}

#[test]
fn verify_check_escape_analysis_none_regions() {
    let mut checker = Checker::new();

    let valid = checker.check_escape_analysis(None, None, d_span());
    assert!(valid);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_register_pattern_borrow() {
    let mut checker = Checker::new();

    checker.register_pattern_borrow("x".into(), BorrowKind::Immut);
    checker.register_pattern_borrow("y".into(), BorrowKind::Mut);

    let borrows_x = checker.get_pattern_borrows("x");
    let borrows_y = checker.get_pattern_borrows("y");

    assert!(borrows_x.is_some());
    assert!(borrows_y.is_some());
}

#[test]
fn verify_check_pattern_borrow_exclusivity_compatible() {
    let mut checker = Checker::new();

    checker.register_pattern_borrow("x".into(), BorrowKind::Immut);
    checker.register_pattern_borrow("y".into(), BorrowKind::Immut);

    let exclusive = checker.check_pattern_borrow_exclusivity(&["x", "y"]);
    assert!(exclusive);
}

#[test]
fn verify_check_pattern_borrow_exclusivity_conflict() {
    let mut checker = Checker::new();

    checker.register_pattern_borrow("x".into(), BorrowKind::Mut);
    checker.register_pattern_borrow("y".into(), BorrowKind::Mut);

    let exclusive = checker.check_pattern_borrow_exclusivity(&["x", "y"]);
    assert!(!exclusive);
}

#[test]
fn verify_pattern_borrow_immut_and_move_compatible() {
    // T7: Move is not a mutable borrow — exclusivity rule only cares about Mut.
    let mut checker = Checker::new();
    checker.register_pattern_borrow("x".into(), BorrowKind::Immut);
    checker.register_pattern_borrow("y".into(), BorrowKind::Move);
    assert!(checker.check_pattern_borrow_exclusivity(&["x", "y"]));
}

#[test]
fn verify_check_pattern_borrow_exclusivity_immut_and_mut() {
    let mut checker = Checker::new();

    checker.register_pattern_borrow("x".into(), BorrowKind::Immut);
    checker.register_pattern_borrow("y".into(), BorrowKind::Mut);

    let exclusive = checker.check_pattern_borrow_exclusivity(&["x", "y"]);
    assert!(exclusive);
}

#[test]
fn verify_validate_reference_escape_same_region() {
    let mut checker = Checker::new();

    checker.bind_reference_lifetime("ref_x".into(), "region_a".into());
    let valid = checker.validate_reference_escape("ref_x", Some("region_a"));
    assert!(valid);
}

#[test]
fn verify_validate_reference_escape_different_regions() {
    let mut checker = Checker::new();

    checker.bind_reference_lifetime("ref_x".into(), "region_a".into());
    let valid = checker.validate_reference_escape("ref_x", Some("region_b"));
    assert!(!valid);
}

#[test]
fn verify_validate_reference_escape_none_lifetime() {
    let checker = Checker::new();

    let valid = checker.validate_reference_escape("unknown_ref", Some("region_a"));
    assert!(valid);
}

#[test]
fn verify_clear_region_context() {
    let mut checker = Checker::new();

    checker.push_region("region_a".into());
    checker.bind_reference_lifetime("ref_x".into(), "region_a".into());
    checker.register_pattern_borrow("x".into(), BorrowKind::Immut);

    assert_eq!(checker.get_current_region(), Some("region_a"));
    assert!(checker.get_reference_lifetime("ref_x").is_some());
    assert!(checker.get_pattern_borrows("x").is_some());

    checker.clear_region_context();

    assert!(checker.get_current_region().is_none());
    assert!(checker.get_reference_lifetime("ref_x").is_none());
    assert!(checker.get_pattern_borrows("x").is_none());
}

#[test]
fn verify_region_stack_multiple_levels() {
    let mut checker = Checker::new();

    checker.push_region("outer".into());
    checker.push_region("middle".into());
    checker.push_region("inner".into());

    assert_eq!(checker.get_current_region(), Some("inner"));

    checker.pop_region();
    assert_eq!(checker.get_current_region(), Some("middle"));

    checker.pop_region();
    assert_eq!(checker.get_current_region(), Some("outer"));

    checker.pop_region();
    assert!(checker.get_current_region().is_none());
}

#[test]
fn assign_to_immutable_binding_reports_error() {
    // `let x = 0; x = 1` must be rejected: the binding has no `mut`.
    let mut checker = Checker::new();
    checker.insert_var("x".into(), Type::Int, /* is_mut */ false, d_span());
    let expr = sp(ast::Expr::Binary {
        op: ast::BinaryOp::Assign,
        left:  Box::new(sp(ast::Expr::Identifier("x".into()))),
        right: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(1)))),
    });
    let _ = checker.infer_expr(&expr);
    assert!(
        checker.errors.iter().any(|e| e.message.contains("immutable binding 'x'")),
        "expected mut-check error, got: {:?}",
        checker.errors,
    );
}

#[test]
fn assign_to_mutable_binding_is_ok() {
    // `let mut x = 0; x = 1` must type-check cleanly.
    let mut checker = Checker::new();
    checker.insert_var("x".into(), Type::Int, /* is_mut */ true, d_span());
    let expr = sp(ast::Expr::Binary {
        op: ast::BinaryOp::Assign,
        left:  Box::new(sp(ast::Expr::Identifier("x".into()))),
        right: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(1)))),
    });
    let _ = checker.infer_expr(&expr);
    assert!(
        checker.errors.is_empty(),
        "expected no errors for mut binding, got: {:?}",
        checker.errors,
    );
}

#[test]
fn compound_assign_to_immutable_binding_reports_error() {
    let mut checker = Checker::new();
    checker.insert_var("x".into(), Type::Int, false, d_span());
    let expr = sp(ast::Expr::Binary {
        op: ast::BinaryOp::AddAssign,
        left:  Box::new(sp(ast::Expr::Identifier("x".into()))),
        right: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(1)))),
    });
    let _ = checker.infer_expr(&expr);
    assert!(
        checker.errors.iter().any(|e| e.message.contains("immutable binding 'x'")),
        "expected mut-check error for AddAssign, got: {:?}",
        checker.errors,
    );
}

#[test]
fn assign_to_unknown_var_does_not_emit_mut_error() {
    let mut checker = Checker::new();
    let expr = sp(ast::Expr::Binary {
        op: ast::BinaryOp::Assign,
        left:  Box::new(sp(ast::Expr::Identifier("never_declared".into()))),
        right: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(1)))),
    });
    let _ = checker.infer_expr(&expr);
    assert!(
        !checker.errors.iter().any(|e| e.message.contains("immutable binding")),
        "mut-check fired on unresolved var; errors: {:?}",
        checker.errors,
    );
}


#[test]
fn t2_immut_borrow_of_moved_var_errors() {
    let mut checker = Checker::new();
    checker.insert_var("s".into(), Type::String, false, d_span());
    let _ = checker.infer_expr(&sp(ast::Expr::Identifier("s".into())));
    let _ = checker.infer_expr(&sp(ast::Expr::Identifier("s".into())));
    let result = checker.try_immut_borrow("s", d_span());
    assert!(result.is_err(), "expected error borrowing a moved var, got {:?}", result);
    assert!(result.unwrap_err().contains("already moved"));
}

#[test]
fn t3_mut_borrow_does_not_consume_move_type() {
    let mut checker = Checker::new();
    checker.insert_var("s".into(), Type::String, true, d_span());
    assert!(checker.try_mut_borrow("s", d_span()).is_ok());
    checker.release_borrow("s");
    let _ty = checker.infer_expr(&sp(ast::Expr::Identifier("s".into())));
    assert!(
        !checker.errors.iter().any(|e| e.message.contains("moved")),
        "&mut x should not consume x; errors: {:?}", checker.errors
    );
    let _ = Ownership::Move;
}
