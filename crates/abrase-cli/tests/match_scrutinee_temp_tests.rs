// A match scrutinee that is a loaded handle (field access, index, call result)
// is a temporary; it must be released after the match like any other temp.

use abrase::compiler::Compiler;
use abrase::lexer::Lexer;
use abrase::parser::Parser;
use myriad::VirtualMachine;

fn run(src: &str) -> Result<(i64, usize), String> {
    let mut p = Parser::new(Lexer::new(src)).with_source(src.to_string());
    let ast = p.parse_program();
    if !p.errors.is_empty() { return Err(format!("parse:\n{}", p.pretty_print_errors())); }
    let mut c = Compiler::new().with_source(src.to_string());
    let module = c.compile_module(&ast).map_err(|_| c.pretty_print_errors())?;
    let mut vm = VirtualMachine::new().with_step_cap(1_000_000);
    let v = vm.run_module(&module)?;
    Ok((v.as_int(), vm.heap_live_count()))
}

fn assert_clean(src: &str, expected: i64) {
    let (v, live) = run(src).unwrap_or_else(|e| panic!("{}\n{}", e, src));
    assert_eq!(v, expected, "value\n{}", src);
    assert_eq!(live, 0, "leaked {} slots\n{}", live, src);
}

const P_W: &str = "type P = | A | B\ntype W = { p: P }\n";

#[test]
fn match_on_let_bound_variant_is_clean() {
    assert_clean(&format!("{P_W}fn main() -> Int {{ let w = W {{ p: A }}; let q = w.p; match q {{ A => 1, _ => 0 }} }}"), 1);
}

#[test]
fn match_on_unit_variant_field_releases_scrutinee() {
    assert_clean(&format!("{P_W}fn main() -> Int {{ let w = W {{ p: A }}; match w.p {{ A => 1, _ => 0 }} }}"), 1);
}

#[test]
fn match_on_payload_variant_field_releases_scrutinee() {
    assert_clean("type P = | A | B(Int)\ntype W = { p: P }\nfn main() -> Int { let w = W { p: B(4) }; match w.p { B(n) => n, _ => 0 } }", 4);
}

#[test]
fn match_on_variant_field_in_loop_releases_scrutinee_each_iter() {
    assert_clean(&format!("{P_W}fn main() -> Int {{ let w = W {{ p: B }}; let mut n = 0; for i in 0..3 {{ n = n + match w.p {{ B => 1, _ => 0 }} }}; n }}"), 3);
}

#[test]
fn match_on_variant_array_element_releases_scrutinee() {
    assert_clean("type P = | A | B\nfn main() -> Int { let xs = [A, B]; match xs[1] { B => 1, _ => 0 } }", 1);
}

#[test]
fn match_on_variant_through_ref_field_releases_scrutinee() {
    assert_clean(&format!("{P_W}fn get(w: &W) -> Int {{ match w.p {{ B => 1, _ => 0 }} }}\nfn main() -> Int {{ let w = W {{ p: B }}; get(&w) }}"), 1);
}

#[test]
fn match_on_variant_call_result_releases_scrutinee() {
    assert_clean("type P = | A | B\nfn mk() -> P { B }\nfn main() -> Int { match mk() { B => 1, _ => 0 } }", 1);
}
