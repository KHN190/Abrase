use abrase::ast::Span;
use abrase::ty::Type;
use abrase::typeck::Checker;

fn d_span() -> abrase::ast::Span {
    abrase::ast::Span { line: 0, col: 0 }
}

// Public Item Access

#[test]
fn verify_public_function_accessible_from_different_module() {
    let mut checker = Checker::new();

    // Start in module std
    checker.push_module("std".into());
    checker.mark_public("read".into());

    // Move to different module io
    checker.pop_module();
    checker.push_module("io".into());

    // Should be able to access std::read as it's public
    let is_accessible = checker.is_public("read");
    assert!(is_accessible, "Public function should be accessible from other modules");

    checker.pop_module();
}

#[test]
fn verify_public_type_accessible() {
    let mut checker = Checker::new();

    checker.push_module("core".into());
    checker.mark_public("Int".into());

    checker.pop_module();
    checker.push_module("app".into());

    let is_accessible = checker.is_public("Int");
    assert!(is_accessible);

    checker.pop_module();
}

#[test]
fn verify_public_const_accessible() {
    let mut checker = Checker::new();

    checker.push_module("config".into());
    checker.mark_public("MAX_SIZE".into());

    checker.pop_module();
    checker.push_module("server".into());

    let is_accessible = checker.is_public("MAX_SIZE");
    assert!(is_accessible);

    checker.pop_module();
}

// Private Item Access

#[test]
fn verify_private_function_not_accessible_from_different_module() {
    let mut checker = Checker::new();

    // Start in module std
    checker.push_module("std".into());
    checker.mark_private("internal_read".into());

    // Move to different module
    checker.pop_module();
    checker.push_module("app".into());

    // Should NOT be able to access std::internal_read as it's private
    let is_accessible = checker.is_public("internal_read");
    assert!(!is_accessible, "Private function should NOT be accessible from other modules");

    checker.pop_module();
}

#[test]
fn verify_private_type_not_accessible() {
    let mut checker = Checker::new();

    checker.push_module("internal".into());
    checker.mark_private("PrivateStruct".into());

    checker.pop_module();
    checker.push_module("external".into());

    let is_accessible = checker.is_public("PrivateStruct");
    assert!(!is_accessible);

    checker.pop_module();
}

#[test]
fn verify_private_const_not_accessible() {
    let mut checker = Checker::new();

    checker.push_module("impl_detail".into());
    checker.mark_private("INTERNAL_BUFFER_SIZE".into());

    checker.pop_module();
    checker.push_module("user".into());

    let is_accessible = checker.is_public("INTERNAL_BUFFER_SIZE");
    assert!(!is_accessible);

    checker.pop_module();
}

// Same Module Access

#[test]
fn verify_private_item_accessible_in_same_module() {
    let mut checker = Checker::new();

    checker.push_module("utils".into());
    checker.mark_private("helper_fn".into());

    // Still in same module, should be accessible
    let is_accessible = checker.is_item_accessible("helper_fn");
    assert!(is_accessible, "Private items should be accessible within same module");

    checker.pop_module();
}

#[test]
fn verify_public_item_accessible_in_same_module() {
    let mut checker = Checker::new();

    checker.push_module("api".into());
    checker.mark_public("process".into());

    let is_accessible = checker.is_public("process");
    assert!(is_accessible);

    checker.pop_module();
}

// Qualified Name Resolution with Privacy

#[test]
fn verify_qualified_name_public_access() {
    let mut checker = Checker::new();

    // Register std::io::read as public
    checker.push_module("std".into());
    checker.push_module("io".into());
    checker.mark_public("read".into());
    checker.pop_module();
    checker.pop_module();

    // Now access from app module
    checker.push_module("app".into());

    // Should be able to access std.io.read
    let is_accessible = checker.is_qualified_accessible(&["std".into(), "io".into(), "read".into()]);
    assert!(is_accessible, "Public qualified name should be accessible");

    checker.pop_module();
}

#[test]
fn verify_qualified_name_private_access() {
    let mut checker = Checker::new();

    // Register std::internal::helper as private
    checker.push_module("std".into());
    checker.push_module("internal".into());
    checker.mark_private("helper".into());
    checker.pop_module();
    checker.pop_module();

    // Now access from app module
    checker.push_module("app".into());

    // Should NOT be able to access std.internal.helper
    let is_accessible = checker.is_qualified_accessible(&["std".into(), "internal".into(), "helper".into()]);
    assert!(!is_accessible, "Private qualified name should NOT be accessible");

    checker.pop_module();
}

// Nested Module Privacy

#[test]
fn verify_deeply_nested_public_access() {
    let mut checker = Checker::new();

    // deep.nested.module.function is public
    checker.push_module("deep".into());
    checker.push_module("nested".into());
    checker.push_module("module".into());
    checker.mark_public("function".into());
    checker.pop_module();
    checker.pop_module();
    checker.pop_module();

    // Access from different module
    checker.push_module("other".into());

    let is_accessible = checker.is_qualified_accessible(&[
        "deep".into(),
        "nested".into(),
        "module".into(),
        "function".into(),
    ]);
    assert!(is_accessible);

    checker.pop_module();
}

#[test]
fn verify_deeply_nested_private_blocked() {
    let mut checker = Checker::new();

    // a.b.c.private_item is private
    checker.push_module("a".into());
    checker.push_module("b".into());
    checker.push_module("c".into());
    checker.mark_private("private_item".into());
    checker.pop_module();
    checker.pop_module();
    checker.pop_module();

    // Access from different module
    checker.push_module("x".into());

    let is_accessible = checker.is_qualified_accessible(&[
        "a".into(),
        "b".into(),
        "c".into(),
        "private_item".into(),
    ]);
    assert!(!is_accessible);

    checker.pop_module();
}

// Re-export Privacy

#[test]
fn verify_public_re_export_accessible() {
    let mut checker = Checker::new();

    // std::io has public read (re-exported from internal)
    checker.push_module("std".into());
    checker.push_module("io".into());
    checker.mark_public("read".into());
    checker.pop_module();
    checker.pop_module();

    // Access from app
    checker.push_module("app".into());

    let is_accessible = checker.is_public("read");
    assert!(is_accessible);

    checker.pop_module();
}

// Built-in Items

#[test]
fn verify_builtin_types_always_public() {
    let checker = Checker::new();

    // Built-in types like Int, String should be accessible
    let int_accessible = checker.is_public("Int");
    let string_accessible = checker.is_public("String");

    assert!(int_accessible, "Built-in types should be public");
    assert!(string_accessible, "Built-in types should be public");
}

// Privacy Enforcement in Variable Resolution

#[test]
fn verify_public_variable_can_be_used_across_modules() {
    let mut checker = Checker::new();

    // Register public constant in math module
    checker.push_module("math".into());
    checker.insert_var("PI".into(), Type::Float, false, d_span());
    checker.mark_public("PI".into());
    checker.pop_module();

    // Try to use from app module
    checker.push_module("app".into());

    let is_accessible = checker.is_public("PI");
    assert!(is_accessible);

    checker.pop_module();
}

#[test]
fn verify_private_variable_cannot_be_used_across_modules() {
    let mut checker = Checker::new();

    // Register private buffer in internal module
    checker.push_module("internal".into());
    checker.insert_var("temp_buffer".into(), Type::String, false, d_span());
    checker.mark_private("temp_buffer".into());
    checker.pop_module();

    // Try to use from external module
    checker.push_module("external".into());

    let is_accessible = checker.is_public("temp_buffer");
    assert!(!is_accessible);

    checker.pop_module();
}

// Multiple Items Privacy

#[test]
fn verify_mixed_public_private_items() {
    let mut checker = Checker::new();

    checker.push_module("lib".into());

    // Register multiple items with different visibility
    checker.insert_var("public_api".into(), Type::String, false, d_span());
    checker.mark_public("public_api".into());

    checker.insert_var("private_impl".into(), Type::String, false, d_span());
    checker.mark_private("private_impl".into());

    checker.pop_module();

    // Switch modules
    checker.push_module("user".into());

    assert!(checker.is_public("public_api"));
    assert!(!checker.is_public("private_impl"));

    checker.pop_module();
}

// Integration Tests

#[test]
fn verify_privacy_respects_module_hierarchy() {
    let mut checker = Checker::new();

    // Create module hierarchy: app -> services -> database
    checker.push_module("app".into());
    checker.push_module("services".into());
    checker.push_module("database".into());

    checker.insert_var("query".into(), Type::String, false, d_span());
    checker.mark_public("query".into());

    checker.insert_var("internal_cache".into(), Type::String, false, d_span());
    checker.mark_private("internal_cache".into());

    checker.pop_module();
    checker.pop_module();
    checker.pop_module();

    // Switch to external module
    checker.push_module("external".into());

    // Public should be accessible
    assert!(checker.is_public("query"));

    // Private should not be accessible
    assert!(!checker.is_public("internal_cache"));

    checker.pop_module();
}

#[test]
fn verify_privacy_enforcement_on_qualified_path() {
    let mut checker = Checker::new();

    // Create api.v1.users path
    checker.push_module("api".into());
    checker.push_module("v1".into());
    checker.push_module("users".into());

    checker.insert_var("get_user".into(), Type::String, false, d_span());
    checker.mark_public("get_user".into());

    checker.insert_var("validate_token".into(), Type::String, false, d_span());
    checker.mark_private("validate_token".into());

    checker.pop_module();
    checker.pop_module();
    checker.pop_module();

    // Access from root
    checker.push_module("root".into());

    let get_user_ok = checker.is_qualified_accessible(&["api".into(), "v1".into(), "users".into(), "get_user".into()]);
    let validate_token_ok = checker.is_qualified_accessible(&["api".into(), "v1".into(), "users".into(), "validate_token".into()]);

    assert!(get_user_ok, "Public API should be accessible");
    assert!(!validate_token_ok, "Private helper should not be accessible");

    checker.pop_module();
}

// --- typeck_scope_tests (typeck_privacy_tests) ---

// Visibility & Module Scoping

#[test]
fn verify_push_pop_module() {
    let mut checker = Checker::new();

    assert_eq!(checker.get_current_module(), vec!["root"]);

    checker.push_module("io".into());
    assert_eq!(checker.get_current_module(), vec!["root", "io"]);

    checker.push_module("file".into());
    assert_eq!(checker.get_current_module(), vec!["root", "io", "file"]);

    checker.pop_module();
    assert_eq!(checker.get_current_module(), vec!["root", "io"]);

    checker.pop_module();
    assert_eq!(checker.get_current_module(), vec!["root"]);
}

#[test]
fn verify_pop_module_does_not_pop_root() {
    let mut checker = Checker::new();

    assert_eq!(checker.get_current_module(), vec!["root"]);

    checker.pop_module();
    assert_eq!(checker.get_current_module(), vec!["root"]);
}

#[test]
fn verify_set_current_module() {
    let mut checker = Checker::new();

    checker.set_current_module(vec!["network".into(), "http".into()]);
    assert_eq!(checker.get_current_module(), vec!["network", "http"]);
}

#[test]
fn verify_mark_public() {
    let mut checker = Checker::new();
    checker.push_module("io".into());

    checker.mark_public("Read".into());

    let public_items = checker.get_public_items();
    assert!(public_items.iter().any(|item| item.contains("Read")));
}

#[test]
fn verify_mark_private() {
    let mut checker = Checker::new();
    checker.push_module("io".into());

    checker.mark_private("read_impl".into());

    let private_items = checker.get_private_items();
    assert!(private_items.iter().any(|item| item.contains("read_impl")));
}

#[test]
fn verify_is_public_in_same_module() {
    let mut checker = Checker::new();
    checker.push_module("io".into());

    checker.mark_public("Read".into());

    assert!(checker.is_public("Read"));
}

#[test]
fn verify_is_public_from_root() {
    let mut checker = Checker::new();
    checker.push_module("io".into());

    checker.mark_public("Read".into());

    // Switch to root module
    checker.set_current_module(vec!["root".into()]);

    assert!(checker.is_public("Read"));
}

#[test]
fn verify_is_private_item() {
    let mut checker = Checker::new();
    checker.push_module("io".into());

    checker.mark_private("internal_buffer".into());

    assert!(!checker.is_public("internal_buffer"));
}

#[test]
fn verify_is_accessible_same_module() {
    let mut checker = Checker::new();
    checker.set_current_module(vec!["io".into()]);

    let item_module = vec!["io".into()];
    assert!(checker.is_accessible("Read", &item_module));
}

#[test]
fn verify_is_accessible_public_item() {
    let mut checker = Checker::new();
    checker.push_module("io".into());
    checker.mark_public("Read".into());

    // Switch to different module
    checker.set_current_module(vec!["root".into(), "net".into()]);

    assert!(checker.is_accessible("Read", &["root".into(), "io".into()]));
}

#[test]
fn verify_is_accessible_private_item_different_module() {
    let mut checker = Checker::new();
    checker.push_module("io".into());
    checker.mark_private("internal_buffer".into());

    // Switch to different module
    checker.set_current_module(vec!["root".into(), "net".into()]);

    assert!(!checker.is_accessible("internal_buffer", &["root".into(), "io".into()]));
}

#[test]
fn verify_validate_visibility_public() {
    let mut checker = Checker::new();
    checker.push_module("io".into());
    checker.mark_public("Read".into());

    let span = Span::new(1, 1);
    let result = checker.validate_visibility("Read", &["root".into(), "io".into()], span);

    assert!(result);
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_validate_visibility_private_from_different_module() {
    let mut checker = Checker::new();
    checker.push_module("io".into());
    checker.mark_private("internal_buffer".into());

    // Switch to different module
    checker.set_current_module(vec!["net".into()]);

    let span = Span::new(1, 1);
    let result = checker.validate_visibility("internal_buffer", &["io".into()], span);

    assert!(!result);
    assert_eq!(checker.errors.len(), 1);
}

#[test]
fn verify_get_public_items() {
    let mut checker = Checker::new();
    checker.push_module("io".into());

    checker.mark_public("Read".into());
    checker.mark_public("Write".into());

    let public_items = checker.get_public_items();
    assert_eq!(public_items.len(), 2);
}

#[test]
fn verify_get_private_items() {
    let mut checker = Checker::new();
    checker.push_module("io".into());

    checker.mark_private("buffer_impl".into());
    checker.mark_private("internal_read".into());

    let private_items = checker.get_private_items();
    assert_eq!(private_items.len(), 2);
}

#[test]
fn verify_clear_visibility_context() {
    let mut checker = Checker::new();
    checker.push_module("io".into());
    checker.mark_public("Read".into());
    checker.mark_private("buffer".into());

    assert_eq!(checker.get_current_module(), vec!["root", "io"]);
    assert_eq!(checker.get_public_items().len(), 1);
    assert_eq!(checker.get_private_items().len(), 1);

    checker.clear_visibility_context();

    assert_eq!(checker.get_current_module(), vec!["root"]);
    assert_eq!(checker.get_public_items().len(), 0);
    assert_eq!(checker.get_private_items().len(), 0);
}

#[test]
fn verify_module_hierarchy() {
    let mut checker = Checker::new();

    checker.push_module("io".into());
    assert_eq!(checker.get_current_module(), vec!["root", "io"]);

    checker.push_module("file".into());
    assert_eq!(checker.get_current_module(), vec!["root", "io", "file"]);

    checker.mark_public("FileReader".into());

    checker.pop_module();
    let module = checker.get_current_module();
    assert_eq!(module.len(), 2);
}

#[test]
fn verify_multiple_modules_visibility() {
    let mut checker = Checker::new();

    // Module io
    checker.push_module("io".into());
    checker.mark_public("Read".into());
    checker.pop_module();

    // Module net
    checker.push_module("net".into());
    checker.mark_public("Connection".into());
    checker.pop_module();

    let public_items = checker.get_public_items();
    assert_eq!(public_items.len(), 2);
}

#[test]
fn verify_accessibility_within_module_hierarchy() {
    let mut checker = Checker::new();

    // Create module hierarchy: root -> io
    checker.set_current_module(vec!["root".into(), "io".into()]);

    // Mark item as public in same module
    let io_module = vec!["root".into(), "io".into()];

    // Check accessibility (items in same module are always accessible)
    assert!(checker.is_accessible("SomeFile", &io_module));
}

#[test]
fn verify_visibility_with_qualified_names() {
    let mut checker = Checker::new();

    checker.push_module("io".into());
    checker.mark_public("BufferedReader".into());

    // Verify qualified name
    let public_items = checker.get_public_items();
    let has_qualified = public_items.iter()
        .any(|item| item.contains("io") && item.contains("BufferedReader"));

    assert!(has_qualified);
}

#[test]
fn verify_override_private_to_public() {
    let mut checker = Checker::new();
    checker.push_module("io".into());

    // First mark as private
    checker.mark_private("Item".into());
    let private_items = checker.get_private_items();
    assert!(private_items.len() > 0);

    // Then mark as public (should override)
    checker.mark_public("Item".into());
    let private_items_after = checker.get_private_items();
    assert!(private_items_after.len() == 0);

    let public_items = checker.get_public_items();
    assert!(public_items.len() > 0);
}

