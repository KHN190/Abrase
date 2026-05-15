#[path = "compiler_codegen_common.rs"]
mod compiler_codegen_common;

use compiler_codegen_common::*;
use abrase::ast::*;
use abrase::vm::Value;

#[test]
fn verify_compile_arithmetic_add() {
    let ast = parse_binary_int(2, BinaryOp::Add, 3);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::Int(5));
}

#[test]
fn verify_compile_arithmetic_sub() {
    let ast = parse_binary_int(10, BinaryOp::Sub, 3);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::Int(7));
}

#[test]
fn verify_compile_arithmetic_mul() {
    let ast = parse_binary_int(3, BinaryOp::Mul, 4);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::Int(12));
}

#[test]
fn verify_compile_arithmetic_div() {
    let ast = parse_binary_int(20, BinaryOp::Div, 4);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::Int(5));
}

#[test]
fn verify_compile_arithmetic_mod() {
    let ast = parse_binary_int(10, BinaryOp::Mod, 3);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::Int(1));
}

#[test]
fn verify_compile_arithmetic_respects_precedence() {
    // 2 + 3 * 4 = 2 + 12 = 14
    let ast = parse_arithmetic_expr();
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::Int(14));
}

#[test]
fn verify_compile_pure_functional_arithmetic() {
    let test_cases = vec![
        (parse_literal_int(0), Value::Int(0)),
        (parse_literal_int(100), Value::Int(100)),
        (parse_binary_int(5, BinaryOp::Add, 3), Value::Int(8)),
        (parse_binary_int(15, BinaryOp::Sub, 7), Value::Int(8)),
        (parse_binary_int(6, BinaryOp::Mul, 7), Value::Int(42)),
        (parse_binary_int(100, BinaryOp::Div, 10), Value::Int(10)),
        (parse_binary_int(17, BinaryOp::Mod, 5), Value::Int(2)),
    ];

    for (ast, expected) in test_cases {
        let result = compile_and_run(&ast).expect("Execution failed");
        assert_eq!(result, expected, "Arithmetic test failed");
    }
}
