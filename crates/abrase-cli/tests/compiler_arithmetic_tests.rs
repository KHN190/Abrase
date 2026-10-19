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
    assert!(result.unwrap().as_float().is_infinite());
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

// ---- ExprId type-table: nested float arith must pick float opcodes, verified
//      against a Rust f64 reference oracle (not hand-computed bit patterns). ----

#[test]
fn nested_float_arith_matches_ieee_oracle() {
    let cases: [(&str, f64); 5] = [
        ("(1.5 + 2.5) * 2.0",            (1.5 + 2.5) * 2.0),
        ("(1.5 + 2.5) * 2.0 - 1.0",      (1.5 + 2.5) * 2.0 - 1.0),
        ("10.0 / 4.0 + 0.25",            10.0 / 4.0 + 0.25),
        ("((3.0 - 1.5) * 4.0) / 2.0",    ((3.0 - 1.5) * 4.0) / 2.0),
        ("1.0 + 2.0 * 3.0 - 4.0 / 2.0",  1.0 + 2.0 * 3.0 - 4.0 / 2.0),
    ];
    for (expr, oracle) in cases {
        let src = format!("fn main() -> Float {{ {expr} }}");
        let (v, live) = run_source_with_heap(&src).unwrap_or_else(|e| panic!("{expr}: {e}"));
        assert_eq!(v, Value::from_float(oracle), "expr `{expr}`: got {:?}, oracle {}", v, oracle);
        assert_eq!(live, 0, "expr `{expr}`: heap must balance");
    }
}

// An int expression in the same program must NOT be typed as float, and a float
// one must NOT be typed as int — the classic span-collision poisoning. Value
// oracle catches an integer opcode emitted for float arith (garbage bit result).
#[test]
fn mixed_int_and_float_in_one_program_dont_poison() {
    let src = r#"
        fn main() -> Float {
            let i = 3 * 4 - 2;
            let f = 1.5 * 4.0 - 2.0;
            f + i.to_f()
        }
    "#;
    // f = 4.0, i = 10 -> 14.0
    assert_eq!(run_source(src), Ok(Value::from_float(14.0)));
}

#[test]
fn nested_mixed_float_int_fuzz_matches_oracle() {
    // Deterministic generator: vary operands by index, build a nested float
    // expression, diff against the Rust f64 evaluation of the same shape.
    for i in 0..40u32 {
        let a = (i as f64) * 0.5 - 7.0;
        let b = (i as f64) * 0.25 + 1.0;
        let c = 2.0 + (i % 5) as f64;
        let d = 1.0 + (i % 3) as f64;
        // shape: ((a + b) * c - d) / (1.0 + b*b)  — all float, nested, mixed ops
        let oracle = ((a + b) * c - d) / (1.0 + b * b);
        let src = format!(
            "fn main() -> Float {{ (({a:?} + {b:?}) * {c:?} - {d:?}) / (1.0 + {b:?} * {b:?}) }}"
        );
        let (v, live) = run_source_with_heap(&src).unwrap_or_else(|e| panic!("i={i}: {e}"));
        assert_eq!(v, Value::from_float(oracle), "i={i}: got {:?}, oracle {}", v, oracle);
        assert_eq!(live, 0, "i={i}: heap must balance");
    }
}

// Synthetic codegen nodes (ExprId::NONE) must still type correctly via the
// structural fallback: string interpolation builds field/ident access nodes at
// compile time, and inlining copies bodies. Both must keep float-vs-int right.
#[test]
fn interpolated_float_uses_float_conversion_not_int() {
    let src = r#"
        fn main() -> String { let f = 2.5; "v={f}" }
    "#;
    assert_eq!(run_source_string(src).as_deref(), Ok("v=2.5"));
}

#[test]
fn inlined_float_fn_result_stays_float() {
    let src = r#"
        fn half() -> Float { 1.5 }
        fn main() -> Float { half() + 2.5 }
    "#;
    assert_eq!(run_source(src), Ok(Value::from_float(4.0)));
}

#[test]
fn int_to_f_small_exact() {
    assert_eq!(run_source("fn main() -> Float { 42.to_f() }"), Ok(Value::from_float(42.0)));
}

#[test]
fn int_to_f_matches_rust_cast_oracle() {
    for n in [0i64, 1, 42, 1 << 52, 1 << 53, (1 << 53) + 1, i64::MAX, -1, -(1 << 53) - 1] {
        let src = format!("fn main() -> Float {{ ({n}).to_f() }}");
        assert_eq!(run_source(&src), Ok(Value::from_float(n as f64)), "n={n}");
    }
}

#[test]
fn int_to_f_beyond_mantissa_loses_precision_no_trap() {
    let big = (1i64 << 53) + 1;
    assert_eq!(run_source(&format!("fn main() -> Float {{ ({big}).to_f() }}")),
        Ok(Value::from_float((1i64 << 53) as f64)),
        "2^53+1 must round down to 2^53 (mantissa limit), not trap");
}

#[test]
fn int_to_f_i64_max_no_panic() {
    assert_eq!(run_source(&format!("fn main() -> Float {{ ({}).to_f() }}", i64::MAX)),
        Ok(Value::from_float(i64::MAX as f64)));
}

const MIN_EXPR: &str = "((0 - 9223372036854775807) - 1)";

#[test]
fn int_max_plus_one_wraps_to_min() {
    assert_eq!(run_source("fn main() -> Int { 9223372036854775807 + 1 }"), Ok(Value::from_int(i64::MIN)));
}

#[test]
fn int_mul_overflow_wraps() {
    assert_eq!(run_source("fn main() -> Int { 9223372036854775807 * 2 }"), Ok(Value::from_int(-2)));
}

#[test]
fn int_min_negate_stays_min() {
    let src = format!("fn main() -> Int {{ let x = {MIN_EXPR}; -x }}");
    assert_eq!(run_source(&src), Ok(Value::from_int(i64::MIN)));
}

#[test]
fn int_min_abs_stays_min_no_panic() {
    let src = format!("fn main() -> Int {{ {MIN_EXPR}.abs() }}");
    assert_eq!(run_source(&src), Ok(Value::from_int(i64::MIN)));
}

#[test]
fn int_div_by_zero_traps_not_panic() {
    assert!(run_source("fn main() -> Int { 1 / 0 }").is_err());
}

#[test]
fn int_min_div_neg_one_const_folds_no_panic() {
    let src = format!("fn main() -> Int {{ {MIN_EXPR} / (0 - 1) }}");
    assert!(run_source(&src).is_ok(), "compile-time fold of MIN/-1 must not host-panic");
}

#[test]
fn int_min_div_neg_one_runtime_traps_not_panic() {
    let src = format!("fn main() -> Int {{ let mut d = 0; d = 0 - 1; {MIN_EXPR} / d }}");
    assert!(run_source(&src).is_err(), "runtime MIN/-1 overflow must trap in VM, not host-panic");
}

#[test]
fn int_mod_by_zero_traps_not_panic() {
    assert!(run_source("fn main() -> Int { 1 % 0 }").is_err());
}

#[test]
fn int_min_mod_neg_one_const_folds_no_panic() {
    let src = format!("fn main() -> Int {{ {MIN_EXPR} % (0 - 1) }}");
    assert!(run_source(&src).is_ok(), "compile-time fold of MIN%-1 must not host-panic");
}

#[test]
fn int_min_mod_neg_one_runtime_traps_not_panic() {
    let src = format!("fn main() -> Int {{ let mut d = 0; d = 0 - 1; {MIN_EXPR} % d }}");
    assert!(run_source(&src).is_err(), "runtime MIN%-1 overflow must trap in VM, not host-panic");
}

#[test]
fn int_shl_ge_width_masks_mod_64() {
    assert_eq!(run_source("fn main() -> Int { 1 << 64 }"), Ok(Value::from_int(1)));
}

#[test]
fn int_shl_63_is_min() {
    assert_eq!(run_source("fn main() -> Int { 1 << 63 }"), Ok(Value::from_int(i64::MIN)));
}

#[test]
fn float_div_zero_is_pos_inf() {
    let v = run_source("fn main() -> Float { 1.0 / 0.0 }").unwrap();
    assert!(v.as_float().is_infinite() && v.as_float() > 0.0);
}

#[test]
fn float_neg_div_zero_is_neg_inf() {
    let v = run_source("fn main() -> Float { (0.0 - 1.0) / 0.0 }").unwrap();
    assert!(v.as_float().is_infinite() && v.as_float() < 0.0);
}

#[test]
fn float_zero_div_zero_is_nan() {
    let v = run_source("fn main() -> Float { 0.0 / 0.0 }").unwrap();
    assert!(v.as_float().is_nan());
}

#[test]
fn sqrt_negative_is_nan_no_panic() {
    let v = run_source("fn main() -> Float { sqrt(0.0 - 1.0) }").unwrap();
    assert!(v.as_float().is_nan());
}

#[test]
fn float_nan_to_i_is_zero() {
    assert_eq!(run_source("fn main() -> Int { (0.0 / 0.0).to_i() }"), Ok(Value::from_int(0)));
}

#[test]
fn float_pos_inf_to_i_saturates_max() {
    assert_eq!(run_source("fn main() -> Int { (1.0 / 0.0).to_i() }"), Ok(Value::from_int(i64::MAX)));
}

#[test]
fn float_neg_inf_to_i_saturates_min() {
    assert_eq!(run_source("fn main() -> Int { ((0.0 - 1.0) / 0.0).to_i() }"), Ok(Value::from_int(i64::MIN)));
}

#[test]
fn float_huge_to_i_saturates_max() {
    assert_eq!(run_source("fn main() -> Int { 1e300.to_i() }"), Ok(Value::from_int(i64::MAX)));
}
