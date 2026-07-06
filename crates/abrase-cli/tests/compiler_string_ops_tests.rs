#[path = "compiler_codegen_common.rs"]
mod compiler_codegen_common;

use compiler_codegen_common::*;
use myriad::Value;

#[test]
fn str_len_basic() {
    let r = run_source("fn main() -> Int { \"ab\".len() }").expect("run");
    assert_eq!(r, Value::from_int(2));
}

#[test]
fn str_len_empty() {
    let r = run_source("fn main() -> Int { \"\".len() }").expect("run");
    assert_eq!(r, Value::from_int(0));
}

#[test]
fn str_len_interpolated_fresh_alloc() {
    let src = "fn main() -> Int { let x = \"cd\"; \"ab{x}\".len() }";
    let r = run_source(src).expect("run");
    assert_eq!(r, Value::from_int(4));
}

#[test]
fn str_byte_at_method_first() {
    let r = run_source("fn main() -> Int { \"ab\".byte_at(0) }").expect("run");
    assert_eq!(r, Value::from_int(97));
}

#[test]
fn str_index_first() {
    let r = run_source("fn main() -> Int { \"ab\"[0] }").expect("run");
    assert_eq!(r, Value::from_int(97));
}

#[test]
fn str_index_second() {
    let r = run_source("fn main() -> Int { \"ab\"[1] }").expect("run");
    assert_eq!(r, Value::from_int(98));
}

#[test]
fn str_index_binding() {
    let r = run_source("fn main() -> Int { let s = \"xy\"; s[1] }").expect("run");
    assert_eq!(r, Value::from_int(121));
}

#[test]
fn str_index_equal_len_is_zero() {
    let r = run_source("fn main() -> Int { \"ab\"[2] }").expect("run");
    assert_eq!(r, Value::from_int(0));
}

#[test]
fn str_index_out_of_range_is_zero() {
    let r = run_source("fn main() -> Int { \"ab\"[99] }").expect("run");
    assert_eq!(r, Value::from_int(0));
}

#[test]
fn str_index_negative_is_zero() {
    let r = run_source("fn main() -> Int { \"ab\"[0 - 1] }").expect("run");
    assert_eq!(r, Value::from_int(0));
}

#[test]
fn str_byte_compose_to_char() {
    let src = "fn main() -> Char { \"Az\"[0].to_c() }";
    let r = run_source(src).expect("run");
    assert_eq!(r, Value::from_char('A'));
}

#[test]
fn str_index_in_loop_no_leak() {
    let src = "fn main() -> Int { let mut acc = 0; let mut i = 0; while i < 10 { let x = \"z\"; acc = acc + \"a{x}\"[1]; i = i + 1 }; acc }";
    let (r, live) = run_source_with_heap(src).expect("run");
    assert_eq!(r, Value::from_int(122 * 10));
    assert_eq!(live, 0, "per-iter string temporaries must be freed: {} live", live);
}

#[test]
fn str_ops_no_heap_leak() {
    let src = "fn main() -> Int { let x = \"cd\"; \"ab{x}\".len() + \"ef\"[1] }";
    let (r, live) = run_source_with_heap(src).expect("run");
    assert_eq!(r, Value::from_int(4 + 102));
    assert_eq!(live, 0, "string temporaries must be freed: {} live", live);
}

#[test]
fn str_len_reused_not_consumed() {
    let (v, live) = run_source_with_heap("fn main() -> Int { let s = \"ab\"; s.len() + s.len() }").expect("run");
    assert_eq!(v, Value::from_int(4));
    assert_eq!(live, 0, "receiver must survive read-only method: {live} live");
}

#[test]
fn str_byte_at_reused_not_consumed() {
    let (v, live) = run_source_with_heap("fn main() -> Int { let s = \"ab\"; s.byte_at(0) + s.byte_at(1) }").expect("run");
    assert_eq!(v, Value::from_int(97 + 98));
    assert_eq!(live, 0, "receiver must survive: {live} live");
}

#[test]
fn str_scan_loop_len_and_index() {
    let src = "fn main() -> Int { let s = \"abc\"; let n = s.len(); let mut i = 0; let mut sum = 0; while i < n { sum = sum + s[i]; i = i + 1 }; sum }";
    let (v, live) = run_source_with_heap(src).expect("run");
    assert_eq!(v, Value::from_int(97 + 98 + 99));
    assert_eq!(live, 0, "scan loop must not leak: {live} live");
}

#[test]
fn str_len_then_index_same_binding() {
    let (v, live) = run_source_with_heap("fn main() -> Int { let s = \"xy\"; let n = s.len(); n + s[0] }").expect("run");
    assert_eq!(v, Value::from_int(2 + 120));
    assert_eq!(live, 0, "{live} live");
}

#[test]
fn str_len_temporary_receiver_no_leak() {
    let (v, live) = run_source_with_heap("fn main() -> Int { \"ab\".len() }").expect("run");
    assert_eq!(v, Value::from_int(2));
    assert_eq!(live, 0, "temp receiver must be freed exactly once: {live} live");
}

#[test]
fn str_byte_at_temporary_receiver_no_leak() {
    let (v, live) = run_source_with_heap("fn main() -> Int { \"Az\".byte_at(0) }").expect("run");
    assert_eq!(v, Value::from_int(65));
    assert_eq!(live, 0, "{live} live");
}

#[test]
fn str_receiver_reused_across_statements() {
    let src = "fn main() -> Int { let s = \"abc\"; let a = s.len(); let b = s.byte_at(0); a + b + s[1] }";
    let (v, live) = run_source_with_heap(src).expect("run");
    assert_eq!(v, Value::from_int(3 + 97 + 98));
    assert_eq!(live, 0, "{live} live");
}

#[test]
fn str_len_on_call_temporary_no_leak() {
    let src = "fn mk() -> String { \"hi\" } fn main() -> Int { mk().len() }";
    let (v, live) = run_source_with_heap(src).expect("run");
    assert_eq!(v, Value::from_int(2));
    assert_eq!(live, 0, "call-temp receiver must drop once: {live} live");
}

#[test]
fn str_len_on_array_element_receiver() {
    // receiver is an index projection -> routes through emit_method_call (not mono-lowered);
    // element is an rc-inc'd temp load, move-staging stays balanced, array survives.
    let src = "fn main() -> Int { let a = [\"hi\"]; let n = a[0].len(); n + a[0].byte_at(0) }";
    let (v, live) = run_source_with_heap(src).expect("run");
    assert_eq!(v, Value::from_int(2 + 104));
    assert_eq!(live, 0, "array element read-only method must not leak/corrupt: {live} live");
}
