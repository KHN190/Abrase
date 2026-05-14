use ect::typeck::Checker;
use ect::ty::Type;

// ── helpers ──────────────────────────────────────────────────────────────────

fn mk() -> Checker { Checker::new() }

fn path(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|s| s.to_string()).collect()
}

// ── register_module_item / lookup_module_item ─────────────────────────────────

#[test]
fn verify_register_and_lookup_module_item() {
    let mut c = mk();
    c.register_module_item(&path(&["root", "std", "io"]), "File".into(), Type::Named("File".into()));
    let ty = c.lookup_module_item(&path(&["root", "std", "io"]), "File");
    assert_eq!(ty, Some(Type::Named("File".into())));
}

#[test]
fn verify_lookup_missing_item_returns_none() {
    let mut c = mk();
    c.register_module_item(&path(&["root", "std", "io"]), "File".into(), Type::Named("File".into()));
    let ty = c.lookup_module_item(&path(&["root", "std", "io"]), "Stream");
    assert!(ty.is_none());
}

#[test]
fn verify_lookup_wrong_module_returns_none() {
    let mut c = mk();
    c.register_module_item(&path(&["root", "std", "io"]), "File".into(), Type::Named("File".into()));
    let ty = c.lookup_module_item(&path(&["root", "std", "net"]), "File");
    assert!(ty.is_none());
}

#[test]
fn verify_multiple_items_in_same_module() {
    let mut c = mk();
    let m = path(&["root", "std", "io"]);
    c.register_module_item(&m, "File".into(), Type::Named("File".into()));
    c.register_module_item(&m, "Stream".into(), Type::Named("Stream".into()));
    assert_eq!(c.lookup_module_item(&m, "File"),   Some(Type::Named("File".into())));
    assert_eq!(c.lookup_module_item(&m, "Stream"), Some(Type::Named("Stream".into())));
}

#[test]
fn verify_items_in_different_modules_independent() {
    let mut c = mk();
    c.register_module_item(&path(&["root", "a"]), "X".into(), Type::Int);
    c.register_module_item(&path(&["root", "b"]), "X".into(), Type::Bool);
    assert_eq!(c.lookup_module_item(&path(&["root", "a"]), "X"), Some(Type::Int));
    assert_eq!(c.lookup_module_item(&path(&["root", "b"]), "X"), Some(Type::Bool));
}

// ── get_module_items ──────────────────────────────────────────────────────────

#[test]
fn verify_get_module_items_returns_map() {
    let mut c = mk();
    let m = path(&["root", "pkg"]);
    c.register_module_item(&m, "Foo".into(), Type::Int);
    c.register_module_item(&m, "Bar".into(), Type::Bool);
    let items = c.get_module_items(&m).expect("module should exist");
    assert!(items.contains_key("Foo"));
    assert!(items.contains_key("Bar"));
}

#[test]
fn verify_get_module_items_for_unregistered_module_is_none() {
    let c = mk();
    assert!(c.get_module_items(&path(&["root", "missing"])).is_none());
}

// ── resolve_qualified_name (segment-by-segment) ───────────────────────────────

fn setup_std_io(c: &mut Checker) {
    // root → std → io → File
    c.register_module_item(&path(&["root"]),         "std".into(),    Type::Named("module".into()));
    c.register_module_item(&path(&["root", "std"]),  "io".into(),     Type::Named("module".into()));
    c.register_module_item(&path(&["root", "std", "io"]), "File".into(), Type::Named("File".into()));
}

#[test]
fn verify_resolve_three_segment_path_from_root() {
    let mut c = mk();
    setup_std_io(&mut c);
    let resolved = c.resolve_qualified_name(&path(&["std", "io", "File"]));
    assert_eq!(resolved, Some(path(&["root", "std", "io", "File"])));
}

#[test]
fn verify_resolve_single_segment_from_current_module() {
    let mut c = mk();
    c.register_module_item(&path(&["root"]), "File".into(), Type::Named("File".into()));
    // current module is "root" by default
    let resolved = c.resolve_qualified_name(&path(&["File"]));
    assert_eq!(resolved, Some(path(&["root", "File"])));
}

#[test]
fn verify_resolve_relative_from_submodule() {
    let mut c = mk();
    c.set_current_module(path(&["root", "app"]));
    c.register_module_item(&path(&["root", "app"]), "Config".into(), Type::Named("Config".into()));
    let resolved = c.resolve_qualified_name(&path(&["Config"]));
    assert_eq!(resolved, Some(path(&["root", "app", "Config"])));
}

#[test]
fn verify_resolve_unknown_first_segment_returns_none() {
    let mut c = mk();
    setup_std_io(&mut c);
    let resolved = c.resolve_qualified_name(&path(&["net", "io", "File"]));
    assert!(resolved.is_none());
}

#[test]
fn verify_resolve_unknown_middle_segment_returns_none() {
    let mut c = mk();
    setup_std_io(&mut c);
    let resolved = c.resolve_qualified_name(&path(&["std", "net", "File"]));
    assert!(resolved.is_none());
}

#[test]
fn verify_resolve_unknown_final_segment_returns_none() {
    let mut c = mk();
    setup_std_io(&mut c);
    let resolved = c.resolve_qualified_name(&path(&["std", "io", "Stream"]));
    assert!(resolved.is_none());
}

#[test]
fn verify_resolve_two_segment_path() {
    let mut c = mk();
    c.register_module_item(&path(&["root"]), "std".into(), Type::Named("module".into()));
    c.register_module_item(&path(&["root", "std"]), "Int".into(), Type::Int);
    let resolved = c.resolve_qualified_name(&path(&["std", "Int"]));
    assert_eq!(resolved, Some(path(&["root", "std", "Int"])));
}

#[test]
fn verify_resolve_prefers_module_registry_over_qualified_names() {
    let mut c = mk();
    // Register in module_registry
    c.register_module_item(&path(&["root"]), "foo".into(), Type::Int);
    c.register_module_item(&path(&["root", "foo"]), "Bar".into(), Type::Named("Bar".into()));
    // Also register conflicting path in qualified_names (old mechanism)
    c.register_qualified_name("Bar".into(), path(&["different", "Bar"]));

    let resolved = c.resolve_qualified_name(&path(&["foo", "Bar"]));
    // Should prefer module_registry path
    assert_eq!(resolved, Some(path(&["root", "foo", "Bar"])));
}

#[test]
fn verify_resolve_empty_path_returns_none() {
    let c = mk();
    assert!(c.resolve_qualified_name(&[]).is_none());
}

// ── 3.2: visibility at every segment (existing infrastructure, new coverage) ─

#[test]
fn verify_intermediate_private_module_blocks_access() {
    let mut c = mk();
    // Mark "internal" sub-module as private
    c.mark_module_private("root::std::internal".into());
    // "helper" lives in the private module
    // From outside, access should be denied because intermediate is private
    let accessible = c.is_accessible("helper", &path(&["root", "std", "internal"]));
    assert!(!accessible, "items in private modules should be inaccessible from outside");
}

#[test]
fn verify_public_item_in_public_module_accessible() {
    let mut c = mk();
    c.push_module("std".into());
    c.push_module("io".into());
    c.mark_public("File".into());
    c.pop_module();
    c.pop_module();
    let accessible = c.is_accessible("File", &path(&["root", "std", "io"]));
    assert!(accessible);
}

#[test]
fn verify_item_in_same_module_always_accessible() {
    let mut c = mk();
    c.set_current_module(path(&["root", "mymod"]));
    // Even without explicit marking, same-module items are accessible
    let accessible = c.is_accessible("secret", &path(&["root", "mymod"]));
    assert!(accessible, "same-module items should always be accessible");
}

#[test]
fn verify_is_qualified_accessible_full_path() {
    let mut c = mk();
    c.push_module("pkg".into());
    c.mark_public("Api".into());
    c.pop_module();
    let accessible = c.is_qualified_accessible(&path(&["pkg", "Api"]));
    assert!(accessible);
}

#[test]
fn verify_is_qualified_accessible_private_item_blocked() {
    let mut c = mk();
    c.push_module("pkg".into());
    c.mark_private("Internal".into());
    c.pop_module();
    let accessible = c.is_qualified_accessible(&path(&["pkg", "Internal"]));
    assert!(!accessible);
}
