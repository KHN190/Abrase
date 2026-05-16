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
