use ect::ty::Type;
use ect::typeck::Checker;
use ect::ast::Span;

fn d_span() -> Span {
    Span { line: 0, col: 0 }
}

#[test]
fn verify_import_collision_with_existing_var() {
    let mut checker = Checker::new();

    // Insert a variable in current scope
    checker.insert_var("foo".into(), Type::Int, false, d_span());

    // Check collision - should return true
    let has_collision = checker.check_import_collision("foo", vec!["module".into()]);
    assert!(has_collision);
    assert!(checker.has_import_collision("foo"));
}

#[test]
fn verify_import_collision_from_different_module() {
    let mut checker = Checker::new();

    // Register an import from module1
    checker.register_import_items(
        vec!["module1".into()],
        vec![ect::ast::ImportItem {
            name: "foo".into(),
            alias: None,
        }],
    );

    // Check collision with same name from different module - should return true
    let has_collision = checker.check_import_collision("foo", vec!["module2".into()]);
    assert!(has_collision);
    assert!(checker.has_import_collision("foo"));
}

#[test]
fn verify_no_collision_same_module_reimport() {
    let mut checker = Checker::new();

    // Register an import from module1
    checker.register_import_items(
        vec!["module1".into()],
        vec![ect::ast::ImportItem {
            name: "foo".into(),
            alias: None,
        }],
    );

    // Check collision with same name from same module - should return false
    let has_collision = checker.check_import_collision("foo", vec!["module1".into()]);
    assert!(!has_collision);
}

#[test]
fn verify_no_import_collision_with_unique_name() {
    let mut checker = Checker::new();

    // Check collision with unique name - should return false
    let has_collision = checker.check_import_collision("unique_name", vec!["module".into()]);
    assert!(!has_collision);
    assert!(!checker.has_import_collision("unique_name"));
}

#[test]
fn verify_multiple_import_collisions_tracked() {
    let mut checker = Checker::new();

    checker.insert_var("a".into(), Type::Int, false, d_span());
    checker.insert_var("b".into(), Type::String, false, d_span());

    checker.check_import_collision("a", vec!["module".into()]);
    checker.check_import_collision("b", vec!["module".into()]);

    assert!(checker.has_import_collision("a"));
    assert!(checker.has_import_collision("b"));
    assert_eq!(checker.get_import_collisions(), 2);
}
