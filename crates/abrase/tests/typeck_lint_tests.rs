use abrase::lexer::Lexer;
use abrase::parser::Parser;
use abrase::typeck::Checker;

fn warnings_for(src: &str) -> Vec<String> {
    let mut p = Parser::new(Lexer::new(src)).with_source(src.to_string());
    let ast = p.parse_program();
    assert!(p.errors.is_empty(), "parse error: {}", p.pretty_print_errors());
    let mut checker = Checker::new();
    checker.check_program(&ast);
    checker.warnings.iter().map(|w| format!("{}:{}", w.code, w.message)).collect()
}

fn has_warning(src: &str, code: &str) -> bool {
    warnings_for(src).iter().any(|w| w.starts_with(code))
}

fn no_warnings(src: &str) -> bool {
    warnings_for(src).is_empty()
}

#[test]
fn unused_let_binding_warns() {
    let src = "fn main() -> Int { let x = 5; 0 }";
    assert!(has_warning(src, "unused_variable"), "expected warning for unused `x`");
}

#[test]
fn used_let_binding_no_warn() {
    let src = "fn main() -> Int { let x = 5; x }";
    assert!(no_warnings(src), "used variable must not warn");
}

#[test]
fn underscore_prefix_suppresses_warn() {
    let src = "fn main() -> Int { let _x = 5; 0 }";
    assert!(no_warnings(src), "`_x` must not warn");
}

#[test]
fn wildcard_pattern_no_warn() {
    let src = "fn main() -> Int { let _ = 5; 0 }";
    assert!(no_warnings(src), "`let _` must not warn");
}

#[test]
fn unused_param_warns() {
    let src = "fn f(x: Int, y: Int) -> Int { x } fn main() -> Int { f(1, 2) }";
    let ws = warnings_for(src);
    assert!(ws.iter().any(|w| w.contains("unused parameter") && w.contains("`y`")),
        "expected warning for unused param `y`, got: {:?}", ws);
    assert!(!ws.iter().any(|w| w.contains("`x`")),
        "used param `x` must not warn");
}

#[test]
fn underscore_param_suppresses_warn() {
    let src = "fn f(_x: Int) -> Int { 0 } fn main() -> Int { f(1) }";
    assert!(no_warnings(src), "`_x` param must not warn");
}

#[test]
fn multiple_unused_all_reported() {
    let src = "fn main() -> Int { let a = 1; let b = 2; 0 }";
    let ws = warnings_for(src);
    assert_eq!(ws.len(), 2, "expected 2 warnings, got: {:?}", ws);
}

#[test]
fn used_in_nested_block_no_warn() {
    let src = "fn main() -> Int { let x = 5; if true { x } else { 0 } }";
    assert!(no_warnings(src), "variable used in nested block must not warn");
}

#[test]
fn unused_warning_message_contains_name() {
    let src = "fn main() -> Int { let foobar = 99; 0 }";
    let ws = warnings_for(src);
    assert!(ws.iter().any(|w| w.contains("foobar")), "warning must name the variable");
}

#[test]
fn dead_function_warns() {
    let src = "fn dead() -> Int { 0 } fn main() -> Int { 0 }";
    assert!(has_warning(src, "dead_code"), "unreachable fn must warn");
}

#[test]
fn pub_function_no_dead_warn() {
    let src = "pub fn exported() -> Int { 0 } fn main() -> Int { 0 }";
    let ws = warnings_for(src);
    assert!(!ws.iter().any(|w| w.contains("exported")), "pub fn must not warn");
}

#[test]
fn called_function_no_dead_warn() {
    let src = "fn helper() -> Int { 1 } fn main() -> Int { helper() }";
    assert!(no_warnings(src), "called fn must not warn");
}

#[test]
fn transitively_reachable_no_dead_warn() {
    let src = "fn a() -> Int { 1 } fn b() -> Int { a() } fn main() -> Int { b() }";
    assert!(no_warnings(src), "transitively reachable fn must not warn");
}

#[test]
fn main_no_dead_warn() {
    let src = "fn main() -> Int { 0 }";
    assert!(no_warnings(src), "main must not warn");
}

#[test]
fn dead_type_warns() {
    let src = "type Orphan = { x: Int } fn main() -> Int { 0 }";
    assert!(has_warning(src, "dead_code"), "unused type must warn");
}

#[test]
fn used_type_no_dead_warn() {
    let src = "type P = { x: Int } fn main() -> Int { let p = P { x: 1 }; p.x }";
    let ws = warnings_for(src);
    assert!(!ws.iter().any(|w| w.contains("dead_code") && w.contains("P")),
        "used type must not warn: {:?}", ws);
}

#[test]
fn pub_type_no_dead_warn() {
    let src = "pub type Exported = { x: Int } fn main() -> Int { 0 }";
    let ws = warnings_for(src);
    assert!(!ws.iter().any(|w| w.contains("Exported")), "pub type must not warn");
}

#[test]
fn used_import_no_warn() {
    let src = "fn main() -> Int { 0 }";
    assert!(no_warnings(src));
}
