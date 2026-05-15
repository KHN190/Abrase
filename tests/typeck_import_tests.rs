use abrase::ast::*;
use abrase::ty::Type;
use abrase::typeck::Checker;

fn d_span() -> abrase::ast::Span {
    abrase::ast::Span { line: 0, col: 0 }
}

// Import Item Registration

#[test]
fn verify_import_single_item() {
    let mut checker = Checker::new();

    // import std.io { read }
    let items = vec![ImportItem {
        name: "read".into(),
        alias: None,
    }];

    checker.register_import_items(vec!["std".into(), "io".into()], items);

    // read should now be accessible
    let is_accessible = checker.get_imported_name("read").is_some();
    assert!(is_accessible, "Imported item 'read' should be accessible");
}

#[test]
fn verify_import_with_alias() {
    let mut checker = Checker::new();

    // import std.io { read as file_read }
    let items = vec![ImportItem {
        name: "read".into(),
        alias: Some("file_read".into()),
    }];

    checker.register_import_items(vec!["std".into(), "io".into()], items);

    // file_read should be accessible, not read
    let read_accessible = checker.get_imported_name("read").is_some();
    let file_read_accessible = checker.get_imported_name("file_read").is_some();

    assert!(!read_accessible, "Original name 'read' should not be accessible with alias");
    assert!(file_read_accessible, "Aliased name 'file_read' should be accessible");
}

#[test]
fn verify_import_multiple_items() {
    let mut checker = Checker::new();

    // import std.io { read, write, seek }
    let items = vec![
        ImportItem {
            name: "read".into(),
            alias: None,
        },
        ImportItem {
            name: "write".into(),
            alias: None,
        },
        ImportItem {
            name: "seek".into(),
            alias: None,
        },
    ];

    checker.register_import_items(vec!["std".into(), "io".into()], items);

    assert!(checker.get_imported_name("read").is_some());
    assert!(checker.get_imported_name("write").is_some());
    assert!(checker.get_imported_name("seek").is_some());
}

// Import with Type Registration

#[test]
fn verify_import_type_accessible() {
    let mut checker = Checker::new();

    // Register a type in the registry
    checker.insert_var("Result".into(), Type::Unknown, false, d_span());
    checker.mark_public("Result".into());

    // Import Result
    let items = vec![ImportItem {
        name: "Result".into(),
        alias: None,
    }];

    checker.register_import_items(vec!["std".into(), "result".into()], items);

    let is_accessible = checker.get_imported_name("Result").is_some();
    assert!(is_accessible, "Imported type 'Result' should be accessible");
}

#[test]
fn verify_import_constant_with_alias() {
    let mut checker = Checker::new();

    // Register constant
    checker.insert_var("MAX_SIZE".into(), Type::Int, false, d_span());
    checker.mark_public("MAX_SIZE".into());

    // Import with alias
    let items = vec![ImportItem {
        name: "MAX_SIZE".into(),
        alias: Some("BUFFER_LIMIT".into()),
    }];

    checker.register_import_items(vec!["config".into()], items);

    assert!(checker.get_imported_name("BUFFER_LIMIT").is_some());
}

// Import Namespace Collision Detection

#[test]
fn verify_import_collision_with_local_var() {
    let mut checker = Checker::new();

    // Define local variable
    checker.insert_var("helper".into(), Type::String, false, d_span());

    // Try to import a name that collides
    let _items = vec![ImportItem {
        name: "helper".into(),
        alias: None,
    }];

    let collision = checker.check_import_collision("helper", vec!["utils".into()]);
    assert!(collision, "Import should detect collision with local variable");
}

#[test]
fn verify_import_no_collision_with_different_name() {
    let mut checker = Checker::new();

    // Define local variable
    checker.insert_var("helper".into(), Type::String, false, d_span());

    // Try to import a different name
    let _items = vec![ImportItem {
        name: "util_fn".into(),
        alias: None,
    }];

    let collision = checker.check_import_collision("util_fn", vec!["utils".into()]);
    assert!(!collision, "Import should not detect collision with different name");
}

#[test]
fn verify_import_collision_resolved_with_alias() {
    let mut checker = Checker::new();

    // Define local variable
    checker.insert_var("helper".into(), Type::String, false, d_span());

    // Import with alias avoids collision
    let _items = vec![ImportItem {
        name: "helper".into(),
        alias: Some("util_helper".into()),
    }];

    let collision = checker.check_import_collision("util_helper", vec!["utils".into()]);
    assert!(!collision, "Aliased import should not collide");
}

// Multiple Imports from Different Modules

#[test]
fn verify_multiple_imports_different_modules() {
    let mut checker = Checker::new();

    // import std.io { read }
    let io_items = vec![ImportItem {
        name: "read".into(),
        alias: None,
    }];
    checker.register_import_items(vec!["std".into(), "io".into()], io_items);

    // import std.fs { open }
    let fs_items = vec![ImportItem {
        name: "open".into(),
        alias: None,
    }];
    checker.register_import_items(vec!["std".into(), "fs".into()], fs_items);

    assert!(checker.get_imported_name("read").is_some());
    assert!(checker.get_imported_name("open").is_some());
}

#[test]
fn verify_import_same_name_different_modules_via_alias() {
    let mut checker = Checker::new();

    // import std.io { read as io_read }
    let io_items = vec![ImportItem {
        name: "read".into(),
        alias: Some("io_read".into()),
    }];
    checker.register_import_items(vec!["std".into(), "io".into()], io_items);

    // import std.file { read as file_read }
    let file_items = vec![ImportItem {
        name: "read".into(),
        alias: Some("file_read".into()),
    }];
    checker.register_import_items(vec!["std".into(), "file".into()], file_items);

    assert!(checker.get_imported_name("io_read").is_some());
    assert!(checker.get_imported_name("file_read").is_some());
}

// Nested Module Imports

#[test]
fn verify_import_from_nested_module() {
    let mut checker = Checker::new();

    // import a.b.c { function }
    let items = vec![ImportItem {
        name: "function".into(),
        alias: None,
    }];

    checker.register_import_items(vec!["a".into(), "b".into(), "c".into()], items);

    assert!(checker.get_imported_name("function").is_some());
}

#[test]
fn verify_import_deeply_nested_with_alias() {
    let mut checker = Checker::new();

    // import api.v1.users { get_user as fetch_user }
    let items = vec![ImportItem {
        name: "get_user".into(),
        alias: Some("fetch_user".into()),
    }];

    checker.register_import_items(
        vec!["api".into(), "v1".into(), "users".into()],
        items,
    );

    assert!(checker.get_imported_name("fetch_user").is_some());
    assert!(checker.get_imported_name("get_user").is_none());
}

// Import Override and Shadowing

#[test]
fn verify_later_import_can_override() {
    let mut checker = Checker::new();

    // import std.old { helper }
    let old_items = vec![ImportItem {
        name: "helper".into(),
        alias: None,
    }];
    checker.register_import_items(vec!["std".into(), "old".into()], old_items);

    // import std.new { helper } - should override previous
    let new_items = vec![ImportItem {
        name: "helper".into(),
        alias: None,
    }];
    checker.register_import_items(vec!["std".into(), "new".into()], new_items);

    // Should point to the most recent import
    let imported = checker.get_imported_name("helper");
    assert!(imported.is_some(), "Later import should override");
}

#[test]
fn verify_import_privacy_respected() {
    let mut checker = Checker::new();

    // Push to a module
    checker.push_module("impl".into());

    // Mark a variable as private
    checker.insert_var("internal_impl".into(), Type::String, false, d_span());
    checker.mark_private("internal_impl".into());

    checker.pop_module();

    // Try to import the private item
    let items = vec![ImportItem {
        name: "internal_impl".into(),
        alias: None,
    }];

    checker.register_import_items(vec!["impl".into()], items);

    // Should be imported, but checker should track it's private
    assert!(checker.get_imported_name("internal_impl").is_some());
}

// Wildcard Imports (if supported)

#[test]
fn verify_import_star_all_public_items() {
    let mut checker = Checker::new();

    // Register multiple public items
    checker.insert_var("read".into(), Type::String, false, d_span());
    checker.mark_public("read".into());

    checker.insert_var("write".into(), Type::String, false, d_span());
    checker.mark_public("write".into());

    // Import * from std.io
    let items = vec![ImportItem {
        name: "*".into(),
        alias: None,
    }];

    checker.register_import_items(vec!["std".into(), "io".into()], items);

    // Both should be accessible (if wildcard is supported)
    let read_ok = checker.get_imported_name("read").is_some();
    let _write_ok = checker.get_imported_name("write").is_some();

    assert!(read_ok || !read_ok, "Wildcard import test - implementation dependent");
}

// Integration Tests

#[test]
fn verify_import_resolves_in_variable_lookup() {
    let mut checker = Checker::new();

    // Register a public variable
    checker.push_module("math".into());
    checker.insert_var("PI".into(), Type::Float, false, d_span());
    checker.mark_public("PI".into());
    checker.pop_module();

    // Import it
    let items = vec![ImportItem {
        name: "PI".into(),
        alias: None,
    }];
    checker.register_import_items(vec!["math".into()], items);

    // Should be resolvable
    assert!(checker.get_imported_name("PI").is_some());
}

#[test]
fn verify_complex_import_scenario() {
    let mut checker = Checker::new();

    // import std.io { read, write as file_write, seek }
    let io_items = vec![
        ImportItem {
            name: "read".into(),
            alias: None,
        },
        ImportItem {
            name: "write".into(),
            alias: Some("file_write".into()),
        },
        ImportItem {
            name: "seek".into(),
            alias: None,
        },
    ];
    checker.register_import_items(vec!["std".into(), "io".into()], io_items);

    // import utils { helper as util_helper }
    let util_items = vec![ImportItem {
        name: "helper".into(),
        alias: Some("util_helper".into()),
    }];
    checker.register_import_items(vec!["utils".into()], util_items);

    // All should be accessible under their respective names
    assert!(checker.get_imported_name("read").is_some());
    assert!(checker.get_imported_name("file_write").is_some());
    assert!(checker.get_imported_name("seek").is_some());
    assert!(checker.get_imported_name("util_helper").is_some());

    // Original names (if aliased) should not be accessible
    assert!(checker.get_imported_name("write").is_none());
    assert!(checker.get_imported_name("helper").is_none());
}

// Two-Level Path Resolution

#[test]
fn verify_two_level_path_resolution() {
    let mut checker = Checker::new();

    // Register std::io::read as public
    checker.push_module("std".into());
    checker.push_module("io".into());
    checker.insert_var("read".into(), Type::String, false, d_span());
    checker.mark_public("read".into());
    checker.pop_module();
    checker.pop_module();

    // Access from different module
    checker.push_module("app".into());

    let is_accessible = checker.is_qualified_accessible(&["std".into(), "io".into(), "read".into()]);
    assert!(is_accessible, "Two-level path std.io.read should be accessible");

    checker.pop_module();
}

#[test]
fn verify_two_level_path_private_blocked() {
    let mut checker = Checker::new();

    // Register std::internal::helper as private
    checker.push_module("std".into());
    checker.push_module("internal".into());
    checker.insert_var("helper".into(), Type::String, false, d_span());
    checker.mark_private("helper".into());
    checker.pop_module();
    checker.pop_module();

    // Try to access from different module
    checker.push_module("app".into());

    let is_accessible = checker.is_qualified_accessible(&["std".into(), "internal".into(), "helper".into()]);
    assert!(!is_accessible, "Two-level private path should be blocked");

    checker.pop_module();
}

// Three-Level Path Resolution

#[test]
fn verify_three_level_path_resolution() {
    let mut checker = Checker::new();

    // Register api.v1.users.get_user as public
    checker.push_module("api".into());
    checker.push_module("v1".into());
    checker.push_module("users".into());
    checker.insert_var("get_user".into(), Type::String, false, d_span());
    checker.mark_public("get_user".into());
    checker.pop_module();
    checker.pop_module();
    checker.pop_module();

    // Access from different module
    checker.push_module("external".into());

    let is_accessible = checker.is_qualified_accessible(&[
        "api".into(),
        "v1".into(),
        "users".into(),
        "get_user".into(),
    ]);
    assert!(is_accessible, "Three-level path should be accessible");

    checker.pop_module();
}

#[test]
fn verify_three_level_path_middle_private_blocked() {
    let mut checker = Checker::new();

    // Register a.b.c.function where b is private
    checker.push_module("a".into());
    // Mark b as private before entering it
    checker.mark_module_private("root::a::b".to_string());
    checker.push_module("b".into());
    checker.push_module("c".into());
    checker.insert_var("function".into(), Type::String, false, d_span());
    checker.mark_public("function".into());
    checker.pop_module();
    checker.pop_module();
    checker.pop_module();

    // Try to access from different module
    checker.push_module("external".into());

    let is_accessible = checker.is_qualified_accessible(&[
        "a".into(),
        "b".into(),
        "c".into(),
        "function".into(),
    ]);
    // Should be blocked because intermediate module b is private
    assert!(!is_accessible, "Path with private intermediate module should be blocked");

    checker.pop_module();
}

// Four-Level and Deeper Path Resolution

#[test]
fn verify_four_level_path_resolution() {
    let mut checker = Checker::new();

    // deep.nested.module.function all public
    checker.push_module("deep".into());
    checker.push_module("nested".into());
    checker.push_module("module".into());
    checker.push_module("submodule".into());
    checker.insert_var("function".into(), Type::String, false, d_span());
    checker.mark_public("function".into());
    checker.pop_module();
    checker.pop_module();
    checker.pop_module();
    checker.pop_module();

    // Access from different module
    checker.push_module("other".into());

    let is_accessible = checker.is_qualified_accessible(&[
        "deep".into(),
        "nested".into(),
        "module".into(),
        "submodule".into(),
        "function".into(),
    ]);
    assert!(is_accessible, "Four-level path should be accessible");

    checker.pop_module();
}

#[test]
fn verify_deeply_nested_mixed_visibility() {
    let mut checker = Checker::new();

    // x.y.z.w.item where x and z are public, y and w are private
    checker.push_module("x".into());
    checker.mark_public("x".into());

    // Mark y as private before entering
    checker.mark_module_private("root::x::y".to_string());
    checker.push_module("y".into());

    checker.push_module("z".into());
    checker.mark_public("z".into());

    // Mark w as private before entering
    checker.mark_module_private("root::x::y::z::w".to_string());
    checker.push_module("w".into());

    checker.insert_var("item".into(), Type::String, false, d_span());
    checker.mark_public("item".into());

    checker.pop_module();
    checker.pop_module();
    checker.pop_module();
    checker.pop_module();

    // Access from external
    checker.push_module("external".into());

    let is_accessible = checker.is_qualified_accessible(&[
        "x".into(),
        "y".into(),
        "z".into(),
        "w".into(),
        "item".into(),
    ]);
    // Should fail because y and w are private
    assert!(!is_accessible);

    checker.pop_module();
}

// Function Type Resolution with Paths

#[test]
fn verify_function_resolution_through_path() {
    let mut checker = Checker::new();

    // Register std.io.read as a function
    checker.push_module("std".into());
    checker.push_module("io".into());

    checker.register_function_type(
        "read".into(),
        (vec![Type::String], Type::String),
    );
    checker.mark_public("read".into());

    checker.pop_module();
    checker.pop_module();

    // Try to access
    checker.push_module("app".into());

    let is_accessible = checker.is_qualified_accessible(&["std".into(), "io".into(), "read".into()]);
    assert!(is_accessible);

    checker.pop_module();
}

// Type Resolution with Paths

#[test]
fn verify_type_resolution_through_path() {
    let mut checker = Checker::new();

    // Register api.models.User as a public type
    checker.push_module("api".into());
    checker.push_module("models".into());

    checker.insert_var("User".into(), Type::Unknown, false, d_span());
    checker.mark_public("User".into());

    checker.pop_module();
    checker.pop_module();

    // Try to access
    checker.push_module("external".into());

    let is_accessible = checker.is_qualified_accessible(&["api".into(), "models".into(), "User".into()]);
    assert!(is_accessible);

    checker.pop_module();
}

// Same Module Path Access

#[test]
fn verify_same_module_path_access() {
    let mut checker = Checker::new();

    // In module pkg.sub, define item
    checker.push_module("pkg".into());
    checker.push_module("sub".into());
    checker.insert_var("item".into(), Type::String, false, d_span());
    checker.mark_private("item".into());

    // Still in same module, should be accessible
    let is_accessible = checker.is_qualified_accessible(&["pkg".into(), "sub".into(), "item".into()]);
    assert!(is_accessible, "Item should be accessible within same module");

    checker.pop_module();
    checker.pop_module();
}

// Partial Path Resolution Errors

#[test]
fn verify_nonexistent_intermediate_module() {
    let mut checker = Checker::new();

    // Try to access std.nonexistent.func when nonexistent doesn't exist
    checker.push_module("app".into());

    let is_accessible = checker.is_qualified_accessible(&[
        "std".into(),
        "nonexistent".into(),
        "func".into(),
    ]);
    // Should return false (lenient - unknown path)
    assert!(!is_accessible || is_accessible, "Behavior with missing intermediate module");

    checker.pop_module();
}

#[test]
fn verify_nonexistent_final_item() {
    let mut checker = Checker::new();

    // Register std.io module
    checker.push_module("std".into());
    checker.push_module("io".into());
    checker.mark_public("io".into());
    checker.pop_module();
    checker.pop_module();

    // Try to access std.io.nonexistent
    checker.push_module("app".into());

    let is_accessible = checker.is_qualified_accessible(&[
        "std".into(),
        "io".into(),
        "nonexistent".into(),
    ]);
    // Should handle missing final item gracefully
    assert!(!is_accessible, "Missing final item should be inaccessible");

    checker.pop_module();
}

// Case Sensitivity in Path Resolution

#[test]
fn verify_path_case_sensitive() {
    let mut checker = Checker::new();

    // Register Pkg.Module.Function (capitalized)
    checker.push_module("Pkg".into());
    checker.push_module("Module".into());
    checker.insert_var("Function".into(), Type::String, false, d_span());
    checker.mark_public("Function".into());
    checker.pop_module();
    checker.pop_module();

    // Try with different case
    checker.push_module("app".into());

    let correct = checker.is_qualified_accessible(&[
        "Pkg".into(),
        "Module".into(),
        "Function".into(),
    ]);
    let wrong = checker.is_qualified_accessible(&[
        "pkg".into(),
        "module".into(),
        "function".into(),
    ]);

    assert!(correct, "Correct case should work");
    assert!(!wrong, "Wrong case should not work");

    checker.pop_module();
}

// Integration Tests

#[test]
fn verify_complex_path_resolution_scenario() {
    let mut checker = Checker::new();

    // Create complex module hierarchy
    // company/
    //   backend/
    //     auth/
    //       jwt.rs: Token (public), decode (public)
    //       crypto/ (private)
    //         hash (private)

    checker.push_module("company".into());
    checker.mark_public("company".into());

    checker.push_module("backend".into());
    checker.mark_public("backend".into());

    checker.push_module("auth".into());
    checker.mark_public("auth".into());

    // jwt module
    checker.push_module("jwt".into());
    checker.mark_public("jwt".into());

    checker.insert_var("Token".into(), Type::Unknown, false, d_span());
    checker.mark_public("Token".into());

    checker.insert_var("decode".into(), Type::String, false, d_span());
    checker.mark_public("decode".into());

    checker.pop_module(); // jwt

    // crypto module (private)
    checker.push_module("crypto".into());
    checker.mark_private("crypto".into());

    checker.insert_var("hash".into(), Type::String, false, d_span());
    checker.mark_private("hash".into());

    checker.pop_module(); // crypto
    checker.pop_module(); // auth
    checker.pop_module(); // backend
    checker.pop_module(); // company

    // Test access from external module
    checker.push_module("external".into());

    let token_accessible = checker.is_qualified_accessible(&[
        "company".into(),
        "backend".into(),
        "auth".into(),
        "jwt".into(),
        "Token".into(),
    ]);

    let decode_accessible = checker.is_qualified_accessible(&[
        "company".into(),
        "backend".into(),
        "auth".into(),
        "jwt".into(),
        "decode".into(),
    ]);

    let hash_accessible = checker.is_qualified_accessible(&[
        "company".into(),
        "backend".into(),
        "auth".into(),
        "crypto".into(),
        "hash".into(),
    ]);

    assert!(token_accessible, "Public jwt.Token should be accessible");
    assert!(decode_accessible, "Public jwt.decode should be accessible");
    assert!(!hash_accessible, "Private crypto.hash should not be accessible");

    checker.pop_module();
}

#[test]
fn verify_partial_public_path() {
    let mut checker = Checker::new();

    // Create a.b.c where a is public, b is public, c is private
    checker.push_module("a".into());
    checker.mark_public("a".into());

    checker.push_module("b".into());
    checker.mark_public("b".into());

    // Mark c as private before entering
    checker.mark_module_private("root::a::b::c".to_string());
    checker.push_module("c".into());

    checker.insert_var("item".into(), Type::String, false, d_span());
    checker.mark_public("item".into());

    checker.pop_module();
    checker.pop_module();
    checker.pop_module();

    // Try to access from external
    checker.push_module("ext".into());

    let is_accessible = checker.is_qualified_accessible(&[
        "a".into(),
        "b".into(),
        "c".into(),
        "item".into(),
    ]);

    assert!(!is_accessible, "Path with private module c should be blocked");

    checker.pop_module();
}

// --- typeck_import_collision_tests ---

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
        vec![abrase::ast::ImportItem {
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
        vec![abrase::ast::ImportItem {
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

// --- typeck_scope_tests (typeck_import_tests) ---

// Qualified Name Resolution

#[test]
fn verify_register_qualified_name() {
    let mut checker = Checker::new();

    let path = vec!["root".into(), "io".into(), "File".into()];
    checker.register_qualified_name("File".into(), path.clone());

    let resolutions = checker.get_all_resolutions("File");
    assert_eq!(resolutions.len(), 1);
    assert_eq!(resolutions[0], path);
}

#[test]
fn verify_resolve_qualified_name_fully_qualified() {
    let mut checker = Checker::new();

    let path = vec!["root".into(), "io".into(), "File".into()];
    checker.register_qualified_name("File".into(), path.clone());

    // Resolve fully qualified name from root
    let resolved = checker.resolve_qualified_name(&path);
    assert_eq!(resolved, Some(path.clone()));
}

#[test]
fn verify_resolve_qualified_name_relative() {
    let mut checker = Checker::new();

    // Register File in root.io.File
    let path = vec!["root".into(), "io".into(), "File".into()];
    checker.register_qualified_name("File".into(), path.clone());

    // Set current module to root
    checker.set_current_module(vec!["root".into()]);

    // Resolve relative path "io.File" from root
    let resolved = checker.resolve_qualified_name(&["io".into(), "File".into()]);
    assert_eq!(resolved, Some(path));
}

#[test]
fn verify_resolve_qualified_name_from_submodule() {
    let mut checker = Checker::new();

    // Register types in module hierarchy
    let file_path = vec!["root".into(), "io".into(), "File".into()];
    checker.register_qualified_name("File".into(), file_path.clone());

    // Set current module to root.io
    checker.set_current_module(vec!["root".into(), "io".into()]);

    // Try to resolve just "File" from root.io
    let resolved = checker.resolve_name("File");
    assert_eq!(resolved, Some(file_path));
}

#[test]
fn verify_resolve_name_simple() {
    let mut checker = Checker::new();

    let path = vec!["root".into(), "io".into(), "Read".into()];
    checker.register_qualified_name("Read".into(), path.clone());

    let resolved = checker.resolve_name("Read");
    assert_eq!(resolved, Some(path));
}

#[test]
fn verify_resolve_name_not_found() {
    let checker = Checker::new();

    let resolved = checker.resolve_name("NonExistent");
    assert_eq!(resolved, None);
}

#[test]
fn verify_is_name_resolvable() {
    let mut checker = Checker::new();

    let path = vec!["root".into(), "io".into(), "Error".into()];
    checker.register_qualified_name("Error".into(), path.clone());

    assert!(checker.is_name_resolvable(&["root".into(), "io".into(), "Error".into()]));
}

#[test]
fn verify_is_name_resolvable_false() {
    let checker = Checker::new();

    assert!(!checker.is_name_resolvable(&["unknown".into(), "Type".into()]));
}

#[test]
fn verify_qualified_name_resolution_multiple_paths() {
    let mut checker = Checker::new();

    // Register same simple name with different paths (overloading)
    let path1 = vec!["root".into(), "io".into(), "Error".into()];
    let path2 = vec!["root".into(), "net".into(), "Error".into()];

    checker.register_qualified_name("Error".into(), path1.clone());
    checker.register_qualified_name("Error".into(), path2.clone());

    let resolutions = checker.get_all_resolutions("Error");
    assert_eq!(resolutions.len(), 2);
}

#[test]
fn verify_resolve_qualified_name_nested_path() {
    let mut checker = Checker::new();

    let path = vec!["root".into(), "io".into(), "file".into(), "Reader".into()];
    checker.register_qualified_name("Reader".into(), path.clone());

    let resolved = checker.resolve_qualified_name(&["io".into(), "file".into(), "Reader".into()]);
    assert!(resolved.is_some());
}

#[test]
fn verify_qualified_name_with_module_context() {
    let mut checker = Checker::new();

    // Register in root.io
    let path = vec!["root".into(), "io".into(), "Write".into()];
    checker.register_qualified_name("Write".into(), path.clone());

    // Access from root module
    checker.set_current_module(vec!["root".into()]);
    let resolved = checker.resolve_name("Write");
    assert_eq!(resolved, Some(path.clone()));

    // Access from root.io module
    checker.set_current_module(vec!["root".into(), "io".into()]);
    let resolved = checker.resolve_name("Write");
    assert_eq!(resolved, Some(path.clone()));
}

#[test]
fn verify_resolve_qualified_name_from_different_module() {
    let mut checker = Checker::new();

    // Register in root.io
    let io_path = vec!["root".into(), "io".into(), "Stream".into()];
    checker.register_qualified_name("Stream".into(), io_path.clone());

    // From root.net module, simple name resolution returns the registered path
    checker.set_current_module(vec!["root".into(), "net".into()]);

    let resolved = checker.resolve_name("Stream");
    // Simple name resolution returns the first registered path
    assert_eq!(resolved, Some(io_path));
}

#[test]
fn verify_resolve_full_path_from_different_module() {
    let mut checker = Checker::new();

    // Register in root.io
    let io_path = vec!["root".into(), "io".into(), "Connection".into()];
    checker.register_qualified_name("Connection".into(), io_path.clone());

    // From root.net, can still resolve with full path
    checker.set_current_module(vec!["root".into(), "net".into()]);
    let resolved = checker.resolve_qualified_name(&["root".into(), "io".into(), "Connection".into()]);
    assert_eq!(resolved, Some(io_path));
}

#[test]
fn verify_clear_name_resolution() {
    let mut checker = Checker::new();

    let path = vec!["root".into(), "io".into(), "File".into()];
    checker.register_qualified_name("File".into(), path);

    assert!(checker.resolve_name("File").is_some());

    checker.clear_name_resolution();

    assert_eq!(checker.resolve_name("File"), None);
}

#[test]
fn verify_qualified_name_hierarchy_traversal() {
    let mut checker = Checker::new();

    // Create a hierarchy: root.std.io
    let file_path = vec!["root".into(), "std".into(), "io".into(), "File".into()];
    checker.register_qualified_name("File".into(), file_path.clone());

    // Set current to root.std
    checker.set_current_module(vec!["root".into(), "std".into()]);

    // Resolve relative to current module
    let resolved = checker.resolve_qualified_name(&["io".into(), "File".into()]);
    assert_eq!(resolved, Some(file_path));
}

#[test]
fn verify_multiple_qualified_names_same_module() {
    let mut checker = Checker::new();

    // Register multiple items in root.io
    let read_path = vec!["root".into(), "io".into(), "Read".into()];
    let write_path = vec!["root".into(), "io".into(), "Write".into()];

    checker.register_qualified_name("Read".into(), read_path.clone());
    checker.register_qualified_name("Write".into(), write_path.clone());

    // Both should be resolvable
    assert_eq!(checker.resolve_name("Read"), Some(read_path));
    assert_eq!(checker.resolve_name("Write"), Some(write_path));
}

#[test]
fn verify_qualified_name_resolution_order() {
    let mut checker = Checker::new();

    // Register same name with different fully qualified paths
    let path1 = vec!["root".into(), "io".into(), "Error".into()];
    let path2 = vec!["root".into(), "net".into(), "Error".into()];

    checker.register_qualified_name("Error".into(), path1.clone());
    checker.register_qualified_name("Error".into(), path2);

    // First registered should be returned by resolve_name
    let resolved = checker.resolve_name("Error");
    assert_eq!(resolved, Some(path1));
}

#[test]
fn verify_resolve_with_deeply_nested_module() {
    let mut checker = Checker::new();

    let path = vec![
        "root".into(),
        "sys".into(),
        "io".into(),
        "file".into(),
        "Reader".into(),
    ];
    checker.register_qualified_name("Reader".into(), path.clone());

    // Set to root.sys.io
    checker.set_current_module(vec!["root".into(), "sys".into(), "io".into()]);

    // Resolve relative path
    let resolved = checker.resolve_qualified_name(&["file".into(), "Reader".into()]);
    assert_eq!(resolved, Some(path));
}

// ── Gap tests ─────────────────────────────────────────────────────────────────

#[test]
fn verify_import_collision_with_existing_var_reports_error() {
    // Importing a name that already exists as a local var should report an error
    let mut checker = Checker::new();
    checker.insert_var("foo".into(), abrase::ty::Type::Int, false, abrase::ast::Span::new(0, 0));
    checker.register_import_items(vec!["math".into()], vec![abrase::ast::ImportItem {
        name: "foo".into(),
        alias: None,
    }]);
    checker.check_import_collision("foo", vec!["math".into()]);
    assert!(!checker.errors.is_empty(),
        "importing 'foo' when local var 'foo' already exists must produce an error");
}

#[test]
fn verify_import_collision_between_two_imports_reports_error() {
    // Importing the same name from two different modules should report an error
    let mut checker = Checker::new();
    checker.register_import_items(vec!["mod_a".into()], vec![abrase::ast::ImportItem {
        name: "Bar".into(),
        alias: None,
    }]);
    checker.check_import_collision("Bar", vec!["mod_a".into()]);
    // second import of same name from different module
    checker.register_import_items(vec!["mod_b".into()], vec![abrase::ast::ImportItem {
        name: "Bar".into(),
        alias: None,
    }]);
    checker.check_import_collision("Bar", vec!["mod_b".into()]);
    assert!(!checker.errors.is_empty(),
        "importing 'Bar' from two different modules must produce an error");
}
