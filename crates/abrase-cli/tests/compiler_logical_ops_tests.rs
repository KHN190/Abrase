#[path = "compiler_codegen_common.rs"]
mod compiler_codegen_common;

use compiler_codegen_common::*;

#[test]
fn logical_and_both_true() {
    let v = run_source("fn main() -> Bool { true && true }").unwrap();
    assert_eq!(v, Value::from_bool(true));
}

#[test]
fn logical_and_left_false() {
    let v = run_source("fn main() -> Bool { false && true }").unwrap();
    assert_eq!(v, Value::from_bool(false));
}

#[test]
fn logical_and_right_false() {
    let v = run_source("fn main() -> Bool { true && false }").unwrap();
    assert_eq!(v, Value::from_bool(false));
}

#[test]
fn logical_or_both_false() {
    let v = run_source("fn main() -> Bool { false || false }").unwrap();
    assert_eq!(v, Value::from_bool(false));
}

#[test]
fn logical_or_left_true() {
    let v = run_source("fn main() -> Bool { true || false }").unwrap();
    assert_eq!(v, Value::from_bool(true));
}

#[test]
fn logical_or_right_true() {
    let v = run_source("fn main() -> Bool { false || true }").unwrap();
    assert_eq!(v, Value::from_bool(true));
}

#[test]
fn logical_and_short_circuits_division() {
    let src = "fn main() -> Bool { let x: Int = 0; x != 0 && 10 / x > 0 }";
    let v = run_source(src).unwrap();
    assert_eq!(v, Value::from_bool(false));
}

#[test]
fn logical_or_short_circuits_division() {
    let src = "fn main() -> Bool { let x: Int = 0; x == 0 || 10 / x > 0 }";
    let v = run_source(src).unwrap();
    assert_eq!(v, Value::from_bool(true));
}

#[test]
fn logical_and_with_comparisons() {
    let v = run_source("fn main() -> Bool { 3 > 1 && 5 < 10 }").unwrap();
    assert_eq!(v, Value::from_bool(true));
}

#[test]
fn logical_or_with_comparisons() {
    let v = run_source("fn main() -> Bool { 3 > 5 || 5 < 10 }").unwrap();
    assert_eq!(v, Value::from_bool(true));
}
