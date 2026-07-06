#[path = "compiler_codegen_common.rs"]
mod compiler_codegen_common;

use compiler_codegen_common::*;
use myriad::Value;

#[test]
fn verify_compile_comparison_eq() {
    let ast = parse_binary_int(5, BinaryOp::Eq, 5);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::from_bool(true));
}

#[test]
fn verify_compile_comparison_neq() {
    let ast = parse_binary_int(5, BinaryOp::Neq, 3);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::from_bool(true));
}

#[test]
fn verify_compile_comparison_lt() {
    let ast = parse_binary_int(3, BinaryOp::Lt, 5);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::from_bool(true));
}

#[test]
fn verify_compile_comparison_gt() {
    let ast = parse_binary_int(5, BinaryOp::Gt, 3);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::from_bool(true));
}

#[test]
fn verify_compile_comparison_lte() {
    let ast = parse_binary_int(3, BinaryOp::Lte, 5);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::from_bool(true));
}

#[test]
fn verify_compile_comparison_gte() {
    let ast = parse_binary_int(5, BinaryOp::Gte, 3);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::from_bool(true));
}

#[test]
fn verify_compile_comparison_eq_false() {
    let ast = parse_binary_int(5, BinaryOp::Eq, 3);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::from_bool(false));
}

#[test]
fn verify_compile_comparison_neq_false() {
    let ast = parse_binary_int(5, BinaryOp::Neq, 5);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::from_bool(false));
}

#[test]
fn verify_compile_comparison_lt_false() {
    let ast = parse_binary_int(5, BinaryOp::Lt, 3);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::from_bool(false));
}

#[test]
fn verify_compile_comparison_gt_false() {
    let ast = parse_binary_int(3, BinaryOp::Gt, 5);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::from_bool(false));
}

#[test]
fn verify_compile_comparison_lte_false() {
    let ast = parse_binary_int(5, BinaryOp::Lte, 3);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::from_bool(false));
}

#[test]
fn verify_compile_comparison_gte_false() {
    let ast = parse_binary_int(3, BinaryOp::Gte, 5);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::from_bool(false));
}

#[test]
fn verify_compile_comparison_lte_equal() {
    let ast = parse_binary_int(5, BinaryOp::Lte, 5);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::from_bool(true));
}

#[test]
fn verify_compile_comparison_gte_equal() {
    let ast = parse_binary_int(5, BinaryOp::Gte, 5);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::from_bool(true));
}

#[test]
fn string_eq_same_content_distinct_alloc() {
    let src = "fn main() -> Bool { let x = \"b\"; let a = \"ab\"; let b = \"a{x}\"; a == b }";
    let r = run_source(src).expect("Execution failed");
    assert_eq!(r, Value::from_bool(true), "equal-content strings must compare equal");
}

#[test]
fn string_eq_different_content() {
    let src = "fn main() -> Bool { let a = \"ab\"; let b = \"ac\"; a == b }";
    let r = run_source(src).expect("Execution failed");
    assert_eq!(r, Value::from_bool(false), "different strings must not compare equal");
}

#[test]
fn string_neq_same_content() {
    let src = "fn main() -> Bool { let x = \"b\"; let a = \"ab\"; let b = \"a{x}\"; a != b }";
    let r = run_source(src).expect("Execution failed");
    assert_eq!(r, Value::from_bool(false), "equal-content strings: != must be false");
}

#[test]
fn string_neq_different_content() {
    let src = "fn main() -> Bool { let a = \"ab\"; let b = \"ac\"; a != b }";
    let r = run_source(src).expect("Execution failed");
    assert_eq!(r, Value::from_bool(true), "different strings: != must be true");
}

#[test]
fn string_eq_operands_not_consumed() {
    let src = "fn main() -> Int { let a = \"xy\"; let b = \"xy\"; if a == b { a.len() + b.len() } else { 0 } }";
    let (v, live) = run_source_with_heap(src).expect("run");
    assert_eq!(v, Value::from_int(4), "== must borrow operands, not consume");
    assert_eq!(live, 0, "no leak from borrowed ==: {live} live");
}

#[test]
fn copy_type_method_reuse_unaffected() {
    let (v, live) = run_source_with_heap("fn main() -> Int { let x = 5; x.max(3) + x.min(2) + x }").expect("run");
    assert_eq!(v, Value::from_int(12));
    assert_eq!(live, 0, "{live} live");
}
