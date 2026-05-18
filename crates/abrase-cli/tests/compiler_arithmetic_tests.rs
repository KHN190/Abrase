#[path = "compiler_codegen_common.rs"]
mod compiler_codegen_common;

use compiler_codegen_common::*;
use myriad::Value;

#[test]
fn verify_compile_arithmetic_add() {
    let ast = parse_binary_int(2, BinaryOp::Add, 3);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::from_int(5));
}

#[test]
fn verify_compile_arithmetic_sub() {
    let ast = parse_binary_int(10, BinaryOp::Sub, 3);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::from_int(7));
}

#[test]
fn verify_compile_arithmetic_mul() {
    let ast = parse_binary_int(3, BinaryOp::Mul, 4);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::from_int(12));
}

#[test]
fn verify_compile_arithmetic_div() {
    let ast = parse_binary_int(20, BinaryOp::Div, 4);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::from_int(5));
}

#[test]
fn verify_compile_arithmetic_mod() {
    let ast = parse_binary_int(10, BinaryOp::Mod, 3);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::from_int(1));
}

#[test]
fn verify_compile_arithmetic_respects_precedence() {
    // 2 + 3 * 4 = 2 + 12 = 14
    let ast = parse_arithmetic_expr();
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::from_int(14));
}

#[test]
fn verify_compile_arithmetic_neg() {
    // Negating a variable emits OpCode::Neg; negating a literal is constant-folded.
    assert_eq!(
        run_source("fn main() -> Int { let x: Int = 5; -x }"),
        Ok(Value::from_int(-5))
    );
}

#[test]
fn verify_compile_arithmetic_neg_of_zero() {
    assert_eq!(
        run_source("fn main() -> Int { let x: Int = 0; -x }"),
        Ok(Value::from_int(0))
    );
}

#[test]
fn verify_compile_addimm() {
    // x + small-literal fuses to AddImm opcode.
    assert_eq!(
        run_source("fn main() -> Int { let x: Int = 10; x + 3 }"),
        Ok(Value::from_int(13))
    );
}

#[test]
fn verify_compile_subimm() {
    assert_eq!(
        run_source("fn main() -> Int { let x: Int = 10; x - 3 }"),
        Ok(Value::from_int(7))
    );
}

#[test]
fn verify_compile_arithmetic_div_by_zero_traps() {
    let result = run_source("fn main() -> Int { 1 / 0 }");
    assert!(result.is_err(), "div by zero must be a runtime error");
    assert!(result.unwrap_err().contains("div by zero"));
}

#[test]
fn verify_compile_arithmetic_mod_by_zero_traps() {
    let result = run_source("fn main() -> Int { 1 % 0 }");
    assert!(result.is_err(), "mod by zero must be a runtime error");
    assert!(result.unwrap_err().contains("mod by zero"));
}

#[test]
fn verify_compile_float_add() {
    assert_eq!(
        run_source("fn main() -> Float { 1.5 + 2.5 }"),
        Ok(Value::from_float(4.0))
    );
}

#[test]
fn verify_compile_float_sub() {
    assert_eq!(
        run_source("fn main() -> Float { 5.0 - 1.5 }"),
        Ok(Value::from_float(3.5))
    );
}

#[test]
fn verify_compile_float_mul() {
    assert_eq!(
        run_source("fn main() -> Float { 2.0 * 3.0 }"),
        Ok(Value::from_float(6.0))
    );
}

#[test]
fn verify_compile_float_div() {
    assert_eq!(
        run_source("fn main() -> Float { 9.0 / 4.0 }"),
        Ok(Value::from_float(2.25))
    );
}

#[test]
fn verify_compile_float_div_produces_infinity() {
    let result = run_source("fn main() -> Float { 1.0 / 0.0 }");
    assert!(result.is_ok());
    assert!(result.unwrap().as_float().unwrap().is_infinite());
}

#[test]
fn verify_compile_float_lt_true() {
    assert_eq!(
        run_source("fn main() -> Bool { 1.5 < 2.5 }"),
        Ok(Value::from_bool(true))
    );
}

#[test]
fn verify_compile_float_lt_false() {
    assert_eq!(
        run_source("fn main() -> Bool { 2.5 < 1.5 }"),
        Ok(Value::from_bool(false))
    );
}

#[test]
fn verify_compile_float_gt_true() {
    // Float `>` is compiled as FLt with swapped operands.
    assert_eq!(
        run_source("fn main() -> Bool { 2.5 > 1.5 }"),
        Ok(Value::from_bool(true))
    );
}

#[test]
fn verify_compile_float_nan_lt_is_false() {
    let src = r#"fn main() -> Bool {
        let x: Float = 0.0;
        let y: Float = 0.0;
        let nan: Float = x / y;
        nan < 1.0
    }"#;
    let result = run_source(src);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Value::from_bool(false));
}
