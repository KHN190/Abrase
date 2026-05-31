use std::path::PathBuf;

use abrase::ast::Decl;
use abrase::compiler::Compiler;
use abrase::loader::{self, LoadError};
use abrase::typeck::Checker;

fn fixture(rel: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests/fixtures/loader");
    p.push(rel);
    p
}

fn fn_names(decls: &[Decl]) -> Vec<String> {
    decls.iter().filter_map(|d| match d {
        Decl::Fn(f) => Some(f.name.clone()),
        _ => None,
    }).collect()
}

fn typeck_errors(rel: &str) -> Vec<String> {
    let program = loader::load_program(&fixture(rel)).unwrap();
    let mut checker = Checker::new();
    checker.check_program(&program.decls);
    checker.errors.iter().map(|e| e.message.clone()).collect()
}

fn compile(rel: &str) -> (Compiler, Result<abrase::bytecode::Module, Vec<abrase::error::Error>>) {
    let program = loader::load_program(&fixture(rel)).unwrap();
    let mut c = Compiler::new().with_source(program.entry_source.clone());
    let r = c.compile_module(&program.decls);
    (c, r)
}

// ---- Resolver: file discovery, dedup, cycles, missing imports ----

#[test]
fn loads_entry_only_when_no_imports() {
    let p = loader::load_program(&fixture("visuals.abe")).unwrap();
    let names = fn_names(&p.decls);
    assert!(names.contains(&"circle".to_string()));
    assert!(names.contains(&"line".to_string()));
    assert_eq!(p.sources.len(), 1);
}

#[test]
fn concatenates_imported_module_decls() {
    let p = loader::load_program(&fixture("piano.abe")).unwrap();
    let names = fn_names(&p.decls);
    assert!(names.contains(&"main".to_string()));
    assert!(names.contains(&"circle".to_string()));
    assert!(names.contains(&"line".to_string()));
}

#[test]
fn entry_source_is_only_the_entry_file() {
    let p = loader::load_program(&fixture("piano.abe")).unwrap();
    assert!(p.entry_source.contains("use visuals"));
    assert!(!p.entry_source.contains("pub fn circle"));
}

#[test]
fn dotted_import_resolves_under_subdir() {
    let p = loader::load_program(&fixture("with_subdir/main.abe")).unwrap();
    let names = fn_names(&p.decls);
    assert!(names.contains(&"area".to_string()));
    assert!(p.sources.iter().any(|(path, _)| path.ends_with("gfx/shapes.abe")));
}

#[test]
fn diamond_dependency_loads_leaf_once() {
    let p = loader::load_program(&fixture("diamond/top.abe")).unwrap();
    let leaf_count = p.sources.iter().filter(|(path, _)| path.ends_with("leaf.abe")).count();
    assert_eq!(leaf_count, 1, "leaf should be deduplicated across diamond");
    let one_count = fn_names(&p.decls).iter().filter(|n| n == &"one").count();
    assert_eq!(one_count, 1, "`one` should be defined exactly once after dedup");
}

#[test]
fn cycle_is_reported_as_error() {
    let err = loader::load_program(&fixture("cycle/a.abe")).unwrap_err();
    assert!(matches!(err, LoadError::Cycle { .. }), "expected Cycle, got {:?}", err);
}

#[test]
fn missing_import_target_is_reported() {
    let err = loader::load_program(&fixture("missing/main.abe")).unwrap_err();
    assert!(matches!(err, LoadError::MissingImport { .. }), "expected MissingImport, got {:?}", err);
}

// ---- Privacy: pub vs private enforcement across modules ----

#[test]
fn pub_fn_in_imported_module_is_callable() {
    let errors = typeck_errors("privacy_ok/main.abe");
    assert!(errors.is_empty(), "expected clean check, got: {:?}", errors);
}

#[test]
fn private_fn_cannot_be_called_without_import() {
    let errors = typeck_errors("privacy_leak/main.abe");
    assert!(
        errors.iter().any(|e| e.contains("helper") && (e.contains("Undefined") || e.contains("private"))),
        "expected error about private/undefined `helper`, got: {:?}", errors,
    );
}

#[test]
fn private_fn_cannot_be_imported_explicitly() {
    let errors = typeck_errors("privacy_import_priv/main.abe");
    assert!(
        errors.iter().any(|e| e.contains("helper") && e.contains("private")),
        "expected `helper is private` error, got: {:?}", errors,
    );
}

#[test]
fn imported_module_fn_does_not_leak_into_root_scope() {
    let program = loader::load_program(&fixture("privacy_ok/main.abe")).unwrap();
    let mut checker = Checker::new();
    checker.check_program(&program.decls);
    let qualified = checker.get_public_items();
    assert!(qualified.iter().any(|q| q == "lib::double"),
        "expected lib::double public, got items: {:?}", qualified);
    assert!(!qualified.iter().any(|q| q == "lib::helper"),
        "private helper should not be public, got items: {:?}", qualified);
}

// ---- Cross-file fn name collision: codegen mangling ----

#[test]
fn cross_file_same_fn_name_compiles() {
    let (_c, r) = compile("name_collision/main.abe");
    assert!(r.is_ok(), "expected clean compile, got errors: {:?}",
        r.err().map(|errs| errs.iter().map(|e| e.message.clone()).collect::<Vec<_>>()));
}

#[test]
fn both_helper_fns_register_under_distinct_keys() {
    let (c, r) = compile("name_collision/main.abe");
    r.expect("compile failed");
    let names = c.fn_names();
    let helper_count = names.iter().filter(|n| n.contains("helper")).count();
    assert!(helper_count >= 2,
        "expected both helpers in func_map (entry's bare + lib's mangled), got: {:?}",
        names.iter().filter(|n| n.contains("helper")).collect::<Vec<_>>());
    assert!(names.iter().any(|n| n == "helper"), "entry's bare `helper` missing: {:?}", names);
    assert!(names.iter().any(|n| n == "lib__helper"), "lib's mangled `helper` missing: {:?}", names);
}

#[test]
fn entry_pub_fn_exports_but_imported_pub_fn_does_not() {
    let (_c, r) = compile("name_collision/main.abe");
    let module = r.expect("compile failed");
    let names: Vec<&str> = module.exports.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"main"), "main export missing: {:?}", names);
    assert!(!names.contains(&"double"),
        "imported `double` should not be a cart export: {:?}", names);
    assert!(!names.contains(&"lib__double"),
        "mangled `lib__double` should not be a cart export: {:?}", names);
}

#[test]
fn imported_module_self_recursion_resolves_to_its_own_mangled_name() {
    let (c, r) = compile("name_collision/main.abe");
    let module = r.expect("compile failed");
    let names = c.fn_names();
    let lib_double_idx = names.iter().position(|n| n == "lib__double")
        .expect("lib__double not registered");
    let helper_idx = names.iter().position(|n| n == "lib__helper").expect("lib__helper not registered");
    let entry_helper_idx = names.iter().position(|n| n == "helper").expect("entry helper not registered");
    let helper_id = helper_idx as u16;
    let entry_helper_id = entry_helper_idx as u16;
    let chunk = module.functions.get(lib_double_idx).cloned().expect("missing chunk");
    let abrase::bytecode::Chunk::Bytecode(bc) = chunk else { panic!("expected bytecode chunk"); };
    let calls_lib_helper = bc.code.iter().any(|op| matches!(op,
        abrase::bytecode::OpCode::Call(_, id) if *id == helper_id));
    let calls_entry_helper = bc.code.iter().any(|op| matches!(op,
        abrase::bytecode::OpCode::Call(_, id) if *id == entry_helper_id));
    assert!(calls_lib_helper, "lib::double must call lib::helper, not entry's helper");
    assert!(!calls_entry_helper, "lib::double accidentally calls entry's helper");
}

// ---- Cross-module static: pub readable, private guarded, mut shared ----

#[test]
fn pub_static_is_readable_across_modules() {
    let (_c, r) = compile("static_pub/main.abe");
    assert!(r.is_ok(), "expected clean compile, got: {:?}",
        r.err().map(|errs| errs.iter().map(|e| e.message.clone()).collect::<Vec<_>>()));
}

#[test]
fn private_static_cannot_be_imported() {
    let errors = typeck_errors("static_priv/main.abe");
    assert!(errors.iter().any(|e| e.contains("SECRET") && e.contains("private")),
        "expected 'SECRET is private' error, got: {:?}", errors);
}

#[test]
fn same_name_private_static_in_two_modules_compiles() {
    let (_c, r) = compile("static_collision/main.abe");
    assert!(r.is_ok(), "expected clean compile, got: {:?}",
        r.err().map(|errs| errs.iter().map(|e| e.message.clone()).collect::<Vec<_>>()));
}

#[test]
fn pub_static_mut_supports_cross_module_read_and_write() {
    let (c, r) = compile("static_mut/main.abe");
    let module = r.expect("compile failed");
    let names = c.fn_names();
    let main_idx = names.iter().position(|n| n == "main").expect("main missing");
    let lib_bump_idx = names.iter().position(|n| n == "lib__bump").expect("lib__bump missing");

    let main_chunk = module.functions.get(main_idx).cloned().expect("main chunk");
    let abrase::bytecode::Chunk::Bytecode(main_bc) = main_chunk else { panic!("expected bytecode") };
    let main_has_st = main_bc.code.iter().any(|op| matches!(op, abrase::bytecode::OpCode::St(_, _, _)));
    let main_has_ld = main_bc.code.iter().any(|op| matches!(op, abrase::bytecode::OpCode::Ld(_, _, _)));
    assert!(main_has_st, "main should emit St for `COUNTER = ...`");
    assert!(main_has_ld, "main should emit Ld for reading COUNTER");

    let bump_chunk = module.functions.get(lib_bump_idx).cloned().expect("lib__bump chunk");
    let abrase::bytecode::Chunk::Bytecode(bump_bc) = bump_chunk else { panic!("expected bytecode") };
    let bump_has_st = bump_bc.code.iter().any(|op| matches!(op, abrase::bytecode::OpCode::St(_, _, _)));
    assert!(bump_has_st, "lib::bump should write COUNTER");
}

#[test]
fn cross_module_static_references_same_offset() {
    let (c, r) = compile("static_mut/main.abe");
    let module = r.expect("compile failed");
    let names = c.fn_names();
    let main_idx = names.iter().position(|n| n == "main").unwrap();
    let lib_bump_idx = names.iter().position(|n| n == "lib__bump").unwrap();

    fn st_offsets(chunk: &abrase::bytecode::Chunk) -> Vec<u16> {
        let abrase::bytecode::Chunk::Bytecode(bc) = chunk else { return vec![]; };
        bc.code.iter().filter_map(|op| match op {
            abrase::bytecode::OpCode::St(_, _, off) => Some(*off),
            _ => None,
        }).collect()
    }
    let main_offs = st_offsets(&module.functions[main_idx]);
    let bump_offs = st_offsets(&module.functions[lib_bump_idx]);
    let shared = main_offs.iter().any(|o| bump_offs.contains(o));
    assert!(shared, "main and lib::bump must St into the same COUNTER slot. main={:?} bump={:?}",
        main_offs, bump_offs);
}
