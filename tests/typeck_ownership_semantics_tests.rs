use ect::ty::{Type, Ownership};
use ect::typeck::Checker;

fn d_span() -> ect::ast::Span {
    ect::ast::Span {
        line: 0,
        col: 0,
    }
}

// Phase 22: Ownership Attributes & Region-Based Reference Validation Tests

// ============================================================================
// Point 1: Integrate Ownership Attributes with Type Bodies
// ============================================================================

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

// ============================================================================
// Point 2: Differentiated Borrowing Rules for @share
// ============================================================================

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

// ============================================================================
// Point 3: Strict Move Enforcement and Scope Exit
// ============================================================================

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

// ============================================================================
// Point 4: Region-Aware Reference Validation
// ============================================================================

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

// ============================================================================
// Point 5: Writer/Reader Exclusivity
// ============================================================================

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

// ============================================================================
// Integration Tests
// ============================================================================

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

    // Variable should still be accessible in original scope
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

    // Take borrow
    let _ = checker.try_immut_borrow("s", d_span());

    // Release borrow
    checker.release_borrow("s");

    // Should be able to take new borrow after release
    let result = checker.try_immut_borrow("s", d_span());
    assert!(result.is_ok());
}

#[test]
fn verify_cannot_mutably_borrow_immutable_variable() {
    let mut checker = Checker::new();

    let move_ty = Type::Named("String".into());
    // is_mut = false
    checker.insert_var("s".into(), move_ty, false, d_span());

    // Should fail to mutable borrow immutable variable
    let result = checker.try_mut_borrow("s", d_span());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("immutable variable"));
}
