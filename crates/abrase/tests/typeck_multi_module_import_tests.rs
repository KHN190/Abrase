use abrase::ast::{ImportItem, self};
use abrase::ty::Type;
use abrase::typeck::Checker;

fn path(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|s| s.to_string()).collect()
}

// Items registered in two distinct modules resolve independently, and items
// imported from each are both accessible under their (possibly aliased) names.
#[test]
fn imports_from_two_modules_coexist() {
    let mut c = Checker::new();

    c.register_module_item(&path(&["math"]), "PI".into(), Type::Int);
    c.register_module_item(&path(&["text"]), "EMPTY".into(), Type::String);

    c.register_import_items(path(&["math"]), vec![ImportItem { name: "PI".into(), alias: None }]);
    c.register_import_items(path(&["text"]), vec![ImportItem { name: "EMPTY".into(), alias: None }]);

    let pi = c.get_imported_name("PI").expect("PI should be imported");
    let empty = c.get_imported_name("EMPTY").expect("EMPTY should be imported");
    assert_eq!(pi.0, path(&["math"]));
    assert_eq!(empty.0, path(&["text"]));

    // Each name still resolves to the right type in its own module.
    assert_eq!(c.lookup_module_item(&path(&["math"]), "PI"), Some(Type::Int));
    assert_eq!(c.lookup_module_item(&path(&["text"]), "EMPTY"), Some(Type::String));
}

// Importing the same name from two different modules is a collision.
#[test]
fn same_name_from_two_modules_collides() {
    let mut c = Checker::new();
    c.register_import_items(path(&["math"]), vec![ImportItem { name: "X".into(), alias: None }]);
    c.register_import_items(path(&["phys"]), vec![ImportItem { name: "X".into(), alias: None }]);
    assert!(c.has_import_collision("X"), "X imported from two modules should collide");
}

// An alias keeps two same-origin names distinct and accessible.
#[test]
fn aliased_import_from_second_module_is_accessible() {
    let mut c = Checker::new();
    c.register_import_items(path(&["math"]), vec![ImportItem { name: "Vec2".into(), alias: None }]);
    c.register_import_items(
        path(&["phys"]),
        vec![ImportItem { name: "Vec2".into(), alias: Some("PVec".into()) }],
    );
    assert!(c.get_imported_name("Vec2").is_some());
    assert!(c.get_imported_name("PVec").is_some());
    assert!(!c.has_import_collision("Vec2"), "alias should avoid the collision");
}

// Source-level: two import statements from two modules parse into two decls.
#[test]
fn two_import_statements_parse() {
    use abrase::lexer::Lexer;
    use abrase::parser::Parser;
    let src = "import math.{PI}; import text.{EMPTY}; fn main() -> Int { 0 }";
    let mut p = Parser::new(Lexer::new(src)).with_source(src.into());
    let decls = p.parse_program();
    assert!(p.errors.is_empty(), "parse errors: {:?}", p.errors);
    let imports = decls.iter().filter(|d| matches!(d, ast::Decl::Import { .. })).count();
    assert_eq!(imports, 2);
}
