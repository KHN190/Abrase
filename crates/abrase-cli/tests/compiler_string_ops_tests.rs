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
