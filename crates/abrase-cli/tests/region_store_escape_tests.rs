// A heap value allocated inside a region (loop body, bare block, inlined call)
// and stored into a container that outlives the region must survive region pop.
// Covers all three store targets (variable / record field / array slot), all
// region kinds, Copy-ownership handles (Array<Int>) and unit variants.

use abrase::compiler::Compiler;
use abrase::lexer::Lexer;
use abrase::parser::Parser;
use myriad::VirtualMachine;

fn run(src: &str, inline: bool) -> Result<(i64, usize), String> {
    let mut p = Parser::new(Lexer::new(src)).with_source(src.to_string());
    let ast = p.parse_program();
    if !p.errors.is_empty() { return Err(format!("parse:\n{}", p.pretty_print_errors())); }
    let mut c = Compiler::new().with_source(src.to_string()).with_inline(inline);
    let module = c.compile_module(&ast).map_err(|_| c.pretty_print_errors())?;
    let mut vm = VirtualMachine::new().with_step_cap(1_000_000);
    let v = vm.run_module(&module)?;
    Ok((v.as_int(), vm.heap_live_count()))
}

fn assert_clean(src: &str, expected: i64) {
    for inline in [false, true] {
        let (v, live) = run(src, inline).unwrap_or_else(|e| panic!("inline={}: {}\n{}", inline, e, src));
        assert_eq!(v, expected, "inline={} value\n{}", inline, src);
        assert_eq!(live, 0, "inline={} leaked {} slots\n{}", inline, live, src);
    }
}

const W_INT: &str = "type W = { a: Array<Int> }\n";
const W_STR: &str = "type W = { a: Array<String> }\n";
const P_W: &str = "type P = | A | B\ntype W = { p: P }\n";

#[test]
fn array_int_field_assigned_in_for_body_single_iter() {
    assert_clean(&format!("{W_INT}fn main() -> Int {{ let mut w = W {{ a: [0, 0] }}; for i in 0..1 {{ w.a = [1, 2] }}; w.a[1] }}"), 2);
}

#[test]
fn array_int_field_assigned_in_for_body_multi_iter() {
    assert_clean(&format!("{W_INT}fn main() -> Int {{ let mut w = W {{ a: [0, 0] }}; for i in 0..3 {{ w.a = [i, i + 1] }}; w.a[1] }}"), 3);
}

#[test]
fn array_int_field_assigned_via_local_in_for_body() {
    assert_clean(&format!("{W_INT}fn main() -> Int {{ let mut w = W {{ a: [0, 0] }}; for i in 0..2 {{ let b = [1, 2]; w.a = b }}; w.a[1] }}"), 2);
}

#[test]
fn array_int_field_assigned_in_while_body() {
    assert_clean(&format!("{W_INT}fn main() -> Int {{ let mut w = W {{ a: [0, 0] }}; let mut i = 0; while i < 2 {{ w.a = [5, 6]; i = i + 1 }}; w.a[0] }}"), 5);
}

#[test]
fn array_int_field_assigned_in_bare_block() {
    assert_clean(&format!("{W_INT}fn main() -> Int {{ let mut w = W {{ a: [0, 0] }}; {{ w.a = [1, 2]; }}; w.a[1] }}"), 2);
}

#[test]
fn array_int_field_assigned_outside_region_does_not_leak() {
    assert_clean(&format!("{W_INT}fn main() -> Int {{ let mut w = W {{ a: [0, 0] }}; w.a = [1, 2]; w.a[1] }}"), 2);
}

#[test]
fn array_int_variable_reassigned_in_for_body() {
    assert_clean("fn main() -> Int { let mut x = [0, 0]; for i in 0..2 { x = [1, 2] }; x[1] }", 2);
}

#[test]
fn array_int_variable_reassigned_in_bare_block() {
    assert_clean("fn main() -> Int { let mut x = [0, 0]; { x = [1, 2]; }; x[1] }", 2);
}

#[test]
fn array_int_variable_reassigned_outside_region_does_not_leak() {
    assert_clean("fn main() -> Int { let mut x = [0, 0]; x = [1, 2]; x[1] }", 2);
}

#[test]
fn array_int_nested_slot_assigned_in_for_body() {
    assert_clean("fn main() -> Int { let mut x = [[0, 0]]; for i in 0..2 { x[0] = [1, 2] }; x[0][1] }", 2);
}

#[test]
fn array_int_nested_slot_assigned_outside_region_does_not_leak() {
    assert_clean("fn main() -> Int { let mut x = [[0, 0]]; x[0] = [1, 2]; x[0][1] }", 2);
}

#[test]
fn array_int_field_assigned_through_mut_ref_in_for_body() {
    assert_clean(&format!("{W_INT}fn set(w: &mut W) -> Unit {{ let b = [1, 2]; w.a = b }}\nfn main() -> Int {{ let mut w = W {{ a: [0, 0] }}; for i in 0..2 {{ set(&mut w) }}; w.a[1] }}"), 2);
}

#[test]
fn array_int_field_assigned_through_inlinable_mut_ref_fn() {
    assert_clean(&format!("{W_INT}fn set(w: &mut W) -> Unit {{ w.a = [1, 2] }}\nfn main() -> Int {{ let mut w = W {{ a: [0, 0] }}; set(&mut w); w.a[1] }}"), 2);
}

#[test]
fn array_string_field_assigned_in_for_body() {
    assert_clean(&format!("{W_STR}fn main() -> Int {{ let mut w = W {{ a: [\"a\"] }}; for i in 0..2 {{ w.a = [\"b\", \"cc\"] }}; w.a[1].len() }}"), 2);
}

#[test]
fn unit_variant_field_assigned_through_inlinable_mut_ref_fn() {
    assert_clean(&format!("{P_W}fn set_b(w: &mut W) -> Unit {{ w.p = B }}\nfn main() -> Int {{ let mut w = W {{ p: A }}; set_b(&mut w); let q = w.p; match q {{ B => 1, _ => 0 }} }}"), 1);
}

#[test]
fn unit_variant_field_assigned_through_inlinable_mut_ref_fn_twice() {
    assert_clean(&format!("{P_W}fn set_b(w: &mut W) -> Unit {{ w.p = B }}\nfn main() -> Int {{ let mut w = W {{ p: A }}; set_b(&mut w); set_b(&mut w); let q = w.p; match q {{ B => 1, _ => 0 }} }}"), 1);
}

#[test]
fn unit_variant_field_assigned_through_inlinable_mut_ref_fn_in_for_body() {
    assert_clean(&format!("{P_W}fn set_b(w: &mut W) -> Unit {{ w.p = B }}\nfn main() -> Int {{ let mut w = W {{ p: A }}; for i in 0..2 {{ set_b(&mut w) }}; let q = w.p; match q {{ B => 1, _ => 0 }} }}"), 1);
}

#[test]
fn unit_variant_field_assigned_through_non_inlinable_mut_ref_fn() {
    assert_clean(&format!("{P_W}fn set_b(w: &mut W) -> Unit {{ let b = B; w.p = b }}\nfn main() -> Int {{ let mut w = W {{ p: A }}; set_b(&mut w); let q = w.p; match q {{ B => 1, _ => 0 }} }}"), 1);
}

#[test]
fn unit_variant_field_assigned_directly_does_not_leak() {
    assert_clean(&format!("{P_W}fn main() -> Int {{ let mut w = W {{ p: A }}; w.p = B; let q = w.p; match q {{ B => 1, _ => 0 }} }}"), 1);
}

#[test]
fn unit_variant_field_assigned_in_for_body() {
    assert_clean(&format!("{P_W}fn main() -> Int {{ let mut w = W {{ p: A }}; for i in 0..2 {{ w.p = B }}; let q = w.p; match q {{ B => 1, _ => 0 }} }}"), 1);
}

#[test]
fn payload_variant_field_assigned_through_inlinable_mut_ref_fn() {
    assert_clean("type P = | A | B(Int)\ntype W = { p: P }\nfn set_b(w: &mut W) -> Unit { w.p = B(7) }\nfn main() -> Int { let mut w = W { p: A }; set_b(&mut w); let q = w.p; match q { B(n) => n, _ => 0 } }", 7);
}

#[test]
fn record_field_assigned_through_inlinable_mut_ref_fn() {
    assert_clean("type I = { v: Int }\ntype W = { i: I }\nfn set(w: &mut W) -> Unit { w.i = I { v: 9 } }\nfn main() -> Int { let mut w = W { i: I { v: 0 } }; set(&mut w); w.i.v }", 9);
}

#[test]
fn local_mut_ref_binding_stores_fresh_array_in_bare_block() {
    assert_clean(&format!("{W_INT}fn main() -> Int {{ let mut w = W {{ a: [0, 0] }}; {{ let r = &mut w; r.a = [1, 2]; }}; w.a[1] }}"), 2);
}

#[test]
fn array_int_field_assigned_in_loop_then_element_mutated() {
    assert_clean(&format!("{W_INT}fn main() -> Int {{ let mut w = W {{ a: [0, 0] }}; for i in 0..2 {{ w.a = [1, 2] }}; w.a[1] = 8; w.a[1] }}"), 8);
}
