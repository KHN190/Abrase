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
fn unused_mut_warns() {
    let src = "fn main() -> Int { let mut x = 5; x }";
    assert!(has_warning(src, "unused_mut"), "never-reassigned mut must warn");
}

#[test]
fn assigned_mut_no_warn() {
    let src = "fn main() -> Int { let mut x = 0; x = x + 1; x }";
    let ws = warnings_for(src);
    assert!(!ws.iter().any(|w| w.starts_with("unused_mut")), "reassigned mut must not warn: {:?}", ws);
}

#[test]
fn underscore_mut_no_warn() {
    let src = "fn main() -> Int { let mut _x = 5; 0 }";
    let ws = warnings_for(src);
    assert!(!ws.iter().any(|w| w.starts_with("unused_mut")), "`_x` must not warn");
}

#[test]
fn unreachable_pattern_after_wildcard_warns() {
    let src = "fn main() -> Int { match 1 { _ => 0, 2 => 99 } }";
    assert!(has_warning(src, "unreachable_pattern"), "arm after `_` must warn");
}

#[test]
fn unreachable_pattern_after_bind_warns() {
    let src = "fn main() -> Int { match 1 { x => x, 2 => 99 } }";
    assert!(has_warning(src, "unreachable_pattern"), "arm after bind catch-all must warn");
}

#[test]
fn last_wildcard_no_warn() {
    let src = "fn main() -> Int { match 1 { 1 => 10, 2 => 20, _ => 0 } }";
    assert!(!has_warning(src, "unreachable_pattern"), "wildcard at end must not warn");
}

#[test]
fn guarded_wildcard_no_warn() {
    let src = "fn main() -> Int { let x = 1; match x { _ if x > 5 => 1, _ => 0 } }";
    let ws = warnings_for(src);
    assert!(!ws.iter().any(|w| w.starts_with("unreachable_pattern")),
        "guarded wildcard must not mark next arm unreachable: {:?}", ws);
}

#[test]
fn infinite_loop_no_break_warns() {
    let src = "fn main() -> Int { loop { 0 }; 0 }";
    assert!(has_warning(src, "infinite_loop"), "`loop` without break must warn");
}

#[test]
fn loop_with_break_no_warn() {
    let src = "fn main() -> Int { loop { break 1 } }";
    assert!(!has_warning(src, "infinite_loop"), "`loop` with break must not warn");
}

#[test]
fn while_loop_no_infinite_warn() {
    let src = "fn main() -> Int { while false { 0 }; 0 }";
    assert!(!has_warning(src, "infinite_loop"), "`while` must not trigger infinite_loop");
}

#[test]
fn nested_break_belongs_to_inner() {
    let src = "fn main() -> Int { loop { loop { break }; 0 }; 0 }";
    let ws = warnings_for(src);
    assert!(ws.iter().any(|w| w.starts_with("infinite_loop")),
        "outer loop without break must warn even if inner loop has break");
}

#[test]
fn unhandled_effect_warns() {
    let src = "effect E { op tick() -> Int } fn main() -> Int { 0 }";
    assert!(has_warning(src, "dead_code"), "unhandled effect must warn");
}

#[test]
fn handled_effect_no_warn() {
    let src = r#"
effect E { op tick() -> Int }
fn body() -> <E> Int { E.tick() }
fn main() -> Int {
  handle body() { return v => v, E.tick => resume(1) }
}
"#;
    let ws = warnings_for(src);
    assert!(!ws.iter().any(|w| w.starts_with("dead_code") && w.contains("`E`")),
        "handled effect must not warn: {:?}", ws);
}

#[test]
fn pub_effect_no_warn() {
    let src = "pub effect E { op tick() -> Int } fn main() -> Int { 0 }";
    let ws = warnings_for(src);
    assert!(!ws.iter().any(|w| w.contains("`E`")), "pub effect must not warn");
}

// ── unused_import ─────────────────────────────────────────────────────────────
// We test at the AST level: `use module::{ item }` where the item is
// never referenced in any expression. The typeck lint fires without needing
// a real file on disk because module resolution happens in the loader, not
// the checker.

#[test]
fn unused_import_warns() {
    // `helper` is imported but never mentioned in the body
    let src = "use lib::{ helper } fn main() -> Int { 0 }";
    assert!(has_warning(src, "unused_import"),
        "imported name never referenced must warn");
}

#[test]
fn used_import_no_warn() {
    // `helper` appears as an identifier in main's body
    let src = "use lib::{ helper } fn main() -> Int { helper() }";
    let ws = warnings_for(src);
    assert!(!ws.iter().any(|w| w.starts_with("unused_import")),
        "referenced import must not warn: {:?}", ws);
}

#[test]
fn unused_import_alias_warns() {
    let src = "use lib::{ helper as h } fn main() -> Int { 0 }";
    let ws = warnings_for(src);
    assert!(ws.iter().any(|w| w.starts_with("unused_import") && w.contains("`h`")),
        "unused aliased import must warn by alias name: {:?}", ws);
}

#[test]
fn used_import_alias_no_warn() {
    let src = "use lib::{ helper as h } fn main() -> Int { h() }";
    let ws = warnings_for(src);
    assert!(!ws.iter().any(|w| w.starts_with("unused_import")),
        "used alias must not warn: {:?}", ws);
}
