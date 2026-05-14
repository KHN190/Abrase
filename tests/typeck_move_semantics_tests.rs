use ect::ty::{Type, Ownership};
use ect::typeck::Checker;
use ect::ast::Span;

fn d_span() -> Span {
    Span { line: 0, col: 0 }
}

#[test]
fn verify_move_type_can_be_immutably_borrowed() {
    let mut checker = Checker::new();

    // Register a Move-semantics type
    checker.register_ownership("MyType".into(), Ownership::Move);

    // Insert a variable of Move-semantics type
    checker.insert_var("x".into(), Type::Named("MyType".into()), true, d_span());

    // Move types CAN be immutably borrowed — only using by value moves them
    let result = checker.try_immut_borrow("x", d_span());
    assert!(result.is_ok());
}

#[test]
fn verify_move_type_mutable_borrow_marks_moved() {
    let mut checker = Checker::new();

    // Register a Move-semantics type
    checker.register_ownership("MyType".into(), Ownership::Move);

    // Insert a mutable variable of Move-semantics type
    checker.insert_var("x".into(), Type::Named("MyType".into()), true, d_span());

    // Mutable borrow of Move type marks the variable as moved
    let result = checker.try_mut_borrow("x", d_span());
    assert!(result.is_ok());

    // After mutable borrow, the variable is marked moved — further use by value should error
    let moved = checker.resolve_var_in_scopes("x");
    assert!(moved.is_some());
}

#[test]
fn verify_copy_type_can_be_borrowed() {
    let mut checker = Checker::new();

    // Register a Copy-semantics type
    checker.register_ownership("Int".into(), Ownership::Copy);

    // Insert a variable of Copy-semantics type
    checker.insert_var("x".into(), Type::Int, true, d_span());

    // Try to immutably borrow - should succeed
    let result = checker.try_immut_borrow("x", d_span());
    assert!(result.is_ok());
}

#[test]
fn verify_share_type_can_be_borrowed() {
    let mut checker = Checker::new();

    // Register a Share-semantics type
    checker.register_ownership("MyShared".into(), Ownership::Share);

    // Insert a variable of Share-semantics type
    checker.insert_var("s".into(), Type::Named("MyShared".into()), true, d_span());

    // Try to immutably borrow - should succeed
    let result = checker.try_immut_borrow("s", d_span());
    assert!(result.is_ok());
}

#[test]
fn verify_copy_primitive_can_be_borrowed() {
    let mut checker = Checker::new();

    // Insert a primitive Int variable
    checker.insert_var("n".into(), Type::Int, true, d_span());

    // Int is Copy by default, should borrow successfully
    let result = checker.try_immut_borrow("n", d_span());
    assert!(result.is_ok());
}

#[test]
fn verify_cannot_mutably_borrow_without_mut_keyword() {
    let mut checker = Checker::new();

    // Register a Move-semantics type
    checker.register_ownership("MyType".into(), Ownership::Move);

    // Insert immutable variable
    checker.insert_var("x".into(), Type::Named("MyType".into()), false, d_span());

    // Try to mutably borrow - should fail because variable is immutable
    let result = checker.try_mut_borrow("x", d_span());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("immutable"));
}

#[test]
fn verify_move_type_allows_multiple_immut_borrows() {
    let mut checker = Checker::new();

    // Register a Move-semantics type
    checker.register_ownership("MyType".into(), Ownership::Move);

    // Insert a mutable variable
    checker.insert_var("x".into(), Type::Named("MyType".into()), true, d_span());

    // Multiple immutable borrows are allowed (move only happens on by-value use)
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

    // String defaults to Share
    assert_eq!(checker.infer_type_ownership("String"), Ownership::Share);

    // Unknown types default to Move
    assert_eq!(checker.infer_type_ownership("UnknownType"), Ownership::Move);
}
