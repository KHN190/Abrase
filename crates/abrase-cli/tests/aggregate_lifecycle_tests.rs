// Aggregate construction must not consume a live binding used as an element:
// scalar reused (loop counter) must survive; reused move-type handle is rejected.

#[path = "compiler_codegen_common.rs"]
mod compiler_codegen_common;
use compiler_codegen_common::*;

fn val(src: &str) -> i64 {
    run_source(src).unwrap_or_else(|e| panic!("must not error/UAF: {}", e)).as_int()
}

// ---- scalar binding reused as element: must not be consumed (else infinite loop) ----

#[test]
fn array_literal_from_reused_loop_counter() {
    // [i, i] must not consume i; loop must terminate, sum well-defined.
    assert_eq!(val(r#"
fn main() -> Int {
  let mut acc = 0; let mut i = 0;
  while i < 3 { let a = [i, i]; acc = acc + a[0] + a[1]; i = i + 1; }
  acc
}
"#), 6); // (0+0)+(1+1)+(2+2) = 6
}

#[test]
fn tuple_from_reused_loop_counter() {
    assert_eq!(val(r#"
fn main() -> Int {
  let mut acc = 0; let mut i = 0;
  while i < 3 { let t = (i, i); acc = acc + t.0 + t.1; i = i + 1; }
  acc
}
"#), 6);
}

#[test]
fn variant_from_reused_loop_counter() {
    assert_eq!(val(r#"
type Pair = P(Int, Int)
fn main() -> Int {
  let mut acc = 0; let mut i = 0;
  while i < 3 { let p = P(i, i); match p { P(a, b) => { acc = acc + a + b } }; i = i + 1; }
  acc
}
"#), 6);
}

#[test]
fn array_repeat_from_binding() {
    assert_eq!(val(r#"
fn main() -> Int {
  let mut acc = 0; let mut i = 0;
  while i < 3 { let a = [i; 3]; acc = acc + a[0] + a[2]; i = i + 1; }
  acc
}
"#), 6); // 2*(0+1+2)
}

#[test]
fn binding_used_twice_in_one_array() {
    // single iteration, but the binding is used as two elements then read again.
    assert_eq!(val(r#"
fn main() -> Int {
  let x = 7;
  let a = [x, x];
  a[0] + a[1] + x
}
"#), 21);
}

#[test]
fn record_from_reused_binding_control() {
    // compile_record is the correct one; control that it stays correct.
    assert_eq!(val(r#"
type R = { a: Int, b: Int }
fn main() -> Int {
  let mut acc = 0; let mut i = 0;
  while i < 3 { let r = R { a: i, b: i }; acc = acc + r.a + r.b; i = i + 1; }
  acc
}
"#), 6);
}

// ---- handle (move-type) binding reused as element: typeck must reject (move-
//      only, no implicit alias) — so no UAF is even reachable here ----

fn err(src: &str) -> String {
    run_source(src).expect_err("must be rejected")
}

#[test]
fn array_of_reused_handle_binding_rejected() {
    assert!(err(r#"
fn main() -> Int {
  let s = "hi";
  let _pair = [s, s];
  0
}
"#).contains("moved"), "reusing a moved handle as an array element must be a move error");
}

#[test]
fn tuple_of_reused_handle_binding_rejected() {
    assert!(err(r#"
fn main() -> Int {
  let s = "hi";
  let _t = (s, s);
  0
}
"#).contains("moved"), "reusing a moved handle as a tuple element must be a move error");
}

// A handle binding moved ONCE into an aggregate (the reporter's shape) is valid
// and must not corrupt the source path or leak.
#[test]
fn handle_moved_once_into_array_ok() {
    let (v, live) = run_source_with_heap(r#"
fn main() -> Int {
  let s = "hi";
  let a = [s];
  let _first = a[0];
  0
}
"#).expect("single move into array must work");
    assert_eq!(v.as_int(), 0);
    assert_eq!(live, 0);
}

// ---- the reporter's class: native-returned handle owned by a record, accessed
//      across frames, must not be prematurely freed ----

#[test]
fn native_handle_field_survives_cross_frame() {
    let (v, live) = run_source_with_heap(r#"
type Rec = { s: String, t: Int }
fn step(r: &mut Rec) -> Int { let _u = "v{r.s}"; r.t = r.t + 1; r.t }
fn main() -> Int {
  let x = 5;
  let mut rec = Rec { s: "num{x}", t: 0 };
  let mut i = 0;
  while i < 30 { let _ = step(&mut rec); i = i + 1; }
  rec.t
}
"#).expect("native handle field must not UAF");
    assert_eq!(v.as_int(), 30);
    assert_eq!(live, 0);
}
