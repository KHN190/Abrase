use abrase::typeck::Checker;
use abrase::ty::Type;

// ── helpers ──────────────────────────────────────────────────────────────────

fn mk() -> Checker { Checker::new() }

fn path(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|s| s.to_string()).collect()
}

// Set up root → std → io → File with each level marked pub for cross-module resolution.
fn setup_std_io(c: &mut Checker) {
    c.register_module_item(&path(&["root"]), "std".into(), Type::Named("module".into()));

    // "io" is in root::std — mark public so it's reachable from root.
    c.push_module("std".into()); // current_module = ["root", "std"]
    c.register_module_item(&path(&["root", "std"]), "io".into(), Type::Named("module".into()));
    c.mark_public("io".into()); // inserts "root::std::io" into public_items

    // "File" is in root::std::io — mark public.
    c.push_module("io".into()); // current_module = ["root", "std", "io"]
    c.register_module_item(&path(&["root", "std", "io"]), "File".into(), Type::Named("File".into()));
    c.mark_public("File".into()); // inserts "root::std::io::File" into public_items

    c.pop_module(); // back to ["root", "std"]
    c.pop_module(); // back to ["root"]
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

// ── resolve_qualified_name: segment-by-segment traversal ─────────────────────

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
    // same-module access is always allowed, no mark_public needed
    let resolved = c.resolve_qualified_name(&path(&["File"]));
    assert_eq!(resolved, Some(path(&["root", "File"])));
}

#[test]
fn verify_resolve_relative_from_submodule() {
    let mut c = mk();
    c.set_current_module(path(&["root", "app"]));
    c.register_module_item(&path(&["root", "app"]), "Config".into(), Type::Named("Config".into()));
    // item_module == current_module → always accessible
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
    // Mark "Int" public in root::std so it's accessible from root
    c.push_module("std".into());
    c.register_module_item(&path(&["root", "std"]), "Int".into(), Type::Int);
    c.mark_public("Int".into()); // "root::std::Int" public
    c.pop_module();

    let resolved = c.resolve_qualified_name(&path(&["std", "Int"]));
    assert_eq!(resolved, Some(path(&["root", "std", "Int"])));
}

#[test]
fn verify_resolve_prefers_module_registry_over_qualified_names() {
    let mut c = mk();
    c.register_module_item(&path(&["root"]), "foo".into(), Type::Named("module".into()));
    // Mark "Bar" public in root::foo so cross-module traversal can reach it
    c.push_module("foo".into());
    c.register_module_item(&path(&["root", "foo"]), "Bar".into(), Type::Named("Bar".into()));
    c.mark_public("Bar".into()); // "root::foo::Bar" public
    c.pop_module();
    // Also register a conflicting path via the old mechanism
    c.register_qualified_name("Bar".into(), path(&["different", "Bar"]));

    let resolved = c.resolve_qualified_name(&path(&["foo", "Bar"]));
    // module_registry segment traversal wins over qualified_names fallback
    assert_eq!(resolved, Some(path(&["root", "foo", "Bar"])));
}

#[test]
fn verify_resolve_empty_path_returns_none() {
    let c = mk();
    assert!(c.resolve_qualified_name(&[]).is_none());
}

// ── visibility enforcement during traversal (3.1 fix) ────────────────────────

#[test]
fn verify_private_item_blocks_traversal() {
    let mut c = mk();
    c.register_module_item(&path(&["root"]), "std".into(), Type::Named("module".into()));
    // Register "secret" in root::std but do NOT mark it public
    c.push_module("std".into());
    c.register_module_item(&path(&["root", "std"]), "secret".into(), Type::Int);
    // deliberately not calling mark_public("secret")
    c.pop_module();

    let resolved = c.resolve_qualified_name(&path(&["std", "secret"]));
    assert!(resolved.is_none(), "private item must not be resolvable from outside its module");
}

#[test]
fn verify_public_item_accessible_via_traversal() {
    let mut c = mk();
    c.register_module_item(&path(&["root"]), "std".into(), Type::Named("module".into()));
    c.push_module("std".into());
    c.register_module_item(&path(&["root", "std"]), "PubItem".into(), Type::Int);
    c.mark_public("PubItem".into()); // "root::std::PubItem" public
    c.pop_module();

    let resolved = c.resolve_qualified_name(&path(&["std", "PubItem"]));
    assert_eq!(resolved, Some(path(&["root", "std", "PubItem"])));
}

#[test]
fn verify_private_deep_item_blocks_traversal() {
    let mut c = mk();
    setup_std_io(&mut c); // io and File are public
    // Add a private sibling to File
    c.push_module("std".into());
    c.push_module("io".into());
    c.register_module_item(&path(&["root", "std", "io"]), "InternalBuf".into(), Type::Named("InternalBuf".into()));
    // NOT marking InternalBuf public
    c.pop_module();
    c.pop_module();

    let resolved = c.resolve_qualified_name(&path(&["std", "io", "InternalBuf"]));
    assert!(resolved.is_none(), "private deep item must not resolve from root");
}

#[test]
fn verify_same_module_private_item_accessible() {
    let mut c = mk();
    // Register a private item in the current module
    c.register_module_item(&path(&["root"]), "PrivHelper".into(), Type::Int);
    // From root (same module), is_accessible returns true regardless of public_items
    let resolved = c.resolve_qualified_name(&path(&["PrivHelper"]));
    assert_eq!(resolved, Some(path(&["root", "PrivHelper"])),
        "items in the current module are always accessible even without public marking");
}

// ── check_program populates module_registry (3.1 fix: wire-up) ───────────────

#[test]
fn verify_check_program_registers_fn_in_module_registry() {
    use abrase::ast;
    let mut c = mk();
    let decls = vec![ast::Decl::Fn(ast::FnDecl {
        name: "my_fn".into(),
        is_pub: true,
        attrs: vec![],
        params: vec![],
        return_type: None,
        effects: vec![],
        body: ast::Block { stmts: vec![], ret: None },
        generics: vec![],
        where_clause: vec![],
    })];
    c.check_program(&decls);
    // After check_program, module_registry["root"] should contain "my_fn"
    let ty = c.lookup_module_item(&path(&["root"]), "my_fn");
    assert!(ty.is_some(), "check_program should register fn into module_registry");
}

#[test]
fn verify_check_program_registers_type_in_module_registry() {
    use abrase::ast;
    let mut c = mk();
    let decls = vec![ast::Decl::Type {
        attrs: vec![],
        name: "MyType".into(),
        is_pub: true,
        ownership: None,
        generics: vec![],
        body: ast::TypeBody::Record(vec![]),
    }];
    c.check_program(&decls);
    let ty = c.lookup_module_item(&path(&["root"]), "MyType");
    assert_eq!(ty, Some(Type::Named("MyType".into())));
}

#[test]
fn verify_check_program_mod_registers_submodule_and_subsequent_decls() {
    use abrase::ast;
    let mut c = mk();
    let decls = vec![
        ast::Decl::Mod("utils".into()),
        ast::Decl::Fn(ast::FnDecl {
            name: "helper".into(),
            is_pub: true,
            attrs: vec![],
            params: vec![],
            return_type: None,
            effects: vec![],
            body: ast::Block { stmts: vec![], ret: None },
            generics: vec![],
            where_clause: vec![],
        }),
    ];
    c.check_program(&decls);
    // "utils" should be in root
    assert!(c.lookup_module_item(&path(&["root"]), "utils").is_some(),
        "Decl::Mod should register sub-module name in parent");
    // "helper" should be in root::utils (the module was pushed before fn was processed)
    assert!(c.lookup_module_item(&path(&["root", "utils"]), "helper").is_some(),
        "fn declared after Mod should land in that module");
}

// ── 3.2: visibility at every segment ─────────────────────────────────────────

#[test]
fn verify_intermediate_private_module_blocks_access() {
    let mut c = mk();
    c.mark_module_private("root::std::internal".into());
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
