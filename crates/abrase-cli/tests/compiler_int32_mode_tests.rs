use abrase::compiler::Compiler;
use abrase::lexer::Lexer;
use abrase::parser::Parser;
use polka::CART_FLAG_INT32_SAFE;

fn compile_with_int32(source: &str, int32: bool) -> Result<polka::Module, String> {
    let mut parser = Parser::new(Lexer::new(source)).with_source(source.to_string());
    let ast = parser.parse_program();
    assert!(parser.errors.is_empty(), "parse errors: {:?}", parser.errors);
    let mut compiler = Compiler::new()
        .with_source(source.to_string())
        .with_int32_mode(int32);
    compiler.compile_module(&ast).map_err(|_| compiler.pretty_print_errors())
}

#[test]
fn int32_mode_accepts_value_in_range() {
    let src = "fn main() -> Int { 2147483647 + (-2147483648) }";
    let m = compile_with_int32(src, true).expect("should compile");
    assert_eq!(m.flags & CART_FLAG_INT32_SAFE, CART_FLAG_INT32_SAFE);
}

#[test]
fn int32_mode_rejects_int_above_i32_max() {
    let src = "fn main() -> Int { 2147483648 }";
    let err = compile_with_int32(src, true).unwrap_err();
    assert!(err.contains("out of i32 range"),
        "expected i32 range error, got: {}", err);
}

#[test]
fn int32_mode_rejects_int_below_i32_min() {
    let src = "fn main() -> Int { -2147483649 }";
    let err = compile_with_int32(src, true).unwrap_err();
    assert!(err.contains("out of i32 range"),
        "expected i32 range error, got: {}", err);
}

#[test]
fn int32_mode_rejects_float_not_representable_as_f32() {
    // 0.1 is famously not exactly representable in either f32 or f64, but its
    // f64 representation is *different* from its f32 representation — so the
    // round-trip check should reject it.
    let src = "fn main() -> Float { 0.1 }";
    let err = compile_with_int32(src, true).unwrap_err();
    assert!(err.contains("not representable as f32"),
        "expected f32 representability error, got: {}", err);
}

#[test]
fn int32_mode_accepts_float_representable_as_f32() {
    // 0.5 = 2^-1, exactly representable in both f32 and f64.
    let src = "fn main() -> Float { 0.5 }";
    compile_with_int32(src, true).expect("should compile");
}

#[test]
fn default_mode_accepts_full_i64_range() {
    let src = "fn main() -> Int { 9223372036854775807 }";
    let m = compile_with_int32(src, false).expect("should compile in default i64 mode");
    assert_eq!(m.flags & CART_FLAG_INT32_SAFE, 0);
}

#[test]
fn default_mode_accepts_f64_only_float() {
    let src = "fn main() -> Float { 0.1 }";
    compile_with_int32(src, false).expect("0.1 is fine in default f64 mode");
}

#[test]
fn int32_mode_validates_pattern_literal() {
    // Pattern literal should also be checked.
    let src = r#"
        fn main() -> Int {
            match 1 {
                2147483648 => 0
                _ => 1
            }
        }
    "#;
    let err = compile_with_int32(src, true).unwrap_err();
    assert!(err.contains("out of i32 range"),
        "expected i32 range error on pattern literal, got: {}", err);
}

#[test]
fn int32_mode_validates_negated_literal_overflow() {
    // -(-i32::MIN) overflow: i32::MIN = -2147483648, so 2147483648 negated
    // is exactly i32::MIN — should still be in range. But 2147483649 negated
    // (= -2147483649) is out of range.
    let src = "fn main() -> Int { -2147483649 }";
    let err = compile_with_int32(src, true).unwrap_err();
    assert!(err.contains("out of i32 range"),
        "expected i32 range error on negated literal, got: {}", err);
}

#[test]
fn cart_roundtrip_preserves_int32_flag() {
    let src = "fn main() -> Int { 42 }";
    let m = compile_with_int32(src, true).expect("should compile");
    let bytes = polka::cartridge::write_pk(&m).expect("write_pk");
    let m2 = polka::cartridge::read_pk(&bytes).expect("read_pk");
    assert_eq!(m2.flags & CART_FLAG_INT32_SAFE, CART_FLAG_INT32_SAFE,
        "INT32_SAFE flag must survive cart write/read roundtrip");
}

#[test]
fn cart_roundtrip_clears_flag_in_default_mode() {
    let src = "fn main() -> Int { 42 }";
    let m = compile_with_int32(src, false).expect("should compile");
    let bytes = polka::cartridge::write_pk(&m).expect("write_pk");
    let m2 = polka::cartridge::read_pk(&bytes).expect("read_pk");
    assert_eq!(m2.flags & CART_FLAG_INT32_SAFE, 0);
}

#[test]
fn int32_mode_stores_float_as_f32_bit_pattern() {
    // 0.5 is exactly representable in both f32 and f64:
    //   f64 bits: 0x3FE0000000000000
    //   f32 bits: 0x3F000000
    // In --int32 mode the cart must store the f32 form so a 32-bit runtime
    // can read the low 32 bits as f32 directly.
    let src = "fn main() -> Float { 0.5 }";
    let m = compile_with_int32(src, true).expect("should compile");
    let bc = match &m.functions[m.entry] {
        polka::Chunk::Bytecode(b) => b,
        _ => panic!("entry must be bytecode"),
    };
    let zero_point_five_f32 = (0.5f32).to_bits() as u64;
    assert!(bc.constants.contains(&zero_point_five_f32),
        "INT32_SAFE cart must store 0.5 as f32 bits ({:#x}); constants = {:?}",
        zero_point_five_f32, bc.constants);
    // And must NOT contain the f64 bits.
    let zero_point_five_f64 = (0.5f64).to_bits();
    assert!(!bc.constants.contains(&zero_point_five_f64),
        "INT32_SAFE cart must not store 0.5 as f64 bits ({:#x})", zero_point_five_f64);
}

#[test]
fn default_mode_stores_float_as_f64_bit_pattern() {
    let src = "fn main() -> Float { 0.5 }";
    let m = compile_with_int32(src, false).expect("should compile");
    let bc = match &m.functions[m.entry] {
        polka::Chunk::Bytecode(b) => b,
        _ => panic!("entry must be bytecode"),
    };
    let zero_point_five_f64 = (0.5f64).to_bits();
    assert!(bc.constants.contains(&zero_point_five_f64),
        "default mode must store 0.5 as f64 bits");
}

fn run_int32(src: &str, int32: bool) -> polka::Value {
    let m = compile_with_int32(src, int32).expect("compile");
    let mut vm = myriad::VirtualMachine::new();
    vm.run_module(&m).expect("run")
}

#[test]
fn int32_safe_runtime_stores_float_result_as_f32_bits() {
    // 0.5 + 0.5 = 1.0 in both modes; only storage width differs.
    let v = run_int32("fn main() -> Float { 0.5 + 0.5 }", true);
    assert_eq!(v.raw(), (1.0_f32).to_bits() as u64,
        "INT32_SAFE FAdd result must be f32 bits in low 32 (high 32 = 0)");

    let v = run_int32("fn main() -> Float { 0.5 + 0.5 }", false);
    assert_eq!(v.raw(), (1.0_f64).to_bits(),
        "default mode FAdd result must be f64 bits");
}

#[test]
fn int32_safe_fadd_rounds_to_f32_precision() {
    // 2^24 + 1: f64 holds 16777217 exactly; f32 rounds to 16777216 (mantissa overflow).
    // `id(x)` blocks const-folding so the runtime FAdd actually executes.
    let src = r#"
        fn id(x: Float) -> Float { x }
        fn main() -> Float { id(16777216.0) + id(1.0) }
    "#;
    let v = run_int32(src, true);
    assert_eq!(v.raw(), (16777216.0_f32).to_bits() as u64,
        "INT32_SAFE FAdd must round in f32: 16777216 + 1 = 16777216, not 16777217");

    let v = run_int32(src, false);
    assert_eq!(v.raw(), (16777217.0_f64).to_bits(),
        "default FAdd: 16777216 + 1 = 16777217 exact in f64");
}

#[test]
fn int32_safe_fdiv_rounds_to_f32_precision() {
    let src = r#"
        fn id(x: Float) -> Float { x }
        fn main() -> Float { id(1.0) / id(3.0) }
    "#;
    let v = run_int32(src, true);
    assert_eq!(v.raw(), (1.0_f32 / 3.0_f32).to_bits() as u64,
        "INT32_SAFE FDiv must produce f32 precision");

    let v = run_int32(src, false);
    assert_eq!(v.raw(), (1.0_f64 / 3.0_f64).to_bits(),
        "default FDiv produces f64 precision");
}

#[test]
fn int32_safe_fneg_runtime_writes_f32_bits() {
    // `neg(0.5)` forces the FNeg opcode (literal-fold path is bypassed by the
    // function-call indirection). Verifies runtime FNeg narrows under INT32_SAFE.
    let src = r#"
        fn neg(x: Float) -> Float { -x }
        fn main() -> Float { neg(0.5) }
    "#;
    let v = run_int32(src, true);
    assert_eq!(v.raw(), (-0.5_f32).to_bits() as u64,
        "INT32_SAFE FNeg must write f32 bits in low 32 (high 32 = 0)");
    assert_eq!(v.raw() >> 32, 0, "high 32 bits must be zero under INT32_SAFE");

    let v = run_int32(src, false);
    assert_eq!(v.raw(), (-0.5_f64).to_bits(),
        "default FNeg writes f64 bits");
}

#[test]
fn int32_safe_flt_reads_operands_as_f32() {
    // FLt on f32-precision values should produce same boolean as f64,
    // but the read path must narrow correctly. Sanity check: 1.5 < 2.5.
    let v = run_int32("fn main() -> Bool { 1.5 < 2.5 }", true);
    assert_eq!(v.raw(), 1, "INT32_SAFE FLt: 1.5 < 2.5 = true");
    let v = run_int32("fn main() -> Bool { 2.5 < 1.5 }", true);
    assert_eq!(v.raw(), 0, "INT32_SAFE FLt: 2.5 < 1.5 = false");
}

#[test]
fn int32_mode_rejects_out_of_range_in_static() {
    let src = r#"
        static X: Int = 2147483648;
        fn main() -> Int { 0 }
    "#;
    let err = compile_with_int32(src, true).unwrap_err();
    assert!(err.contains("out of i32 range"),
        "expected i32 range error in static, got: {}", err);
}

#[test]
fn int32_mode_rejects_out_of_range_in_const() {
    let src = r#"
        const X: Int = 2147483648;
        fn main() -> Int { 0 }
    "#;
    let err = compile_with_int32(src, true).unwrap_err();
    assert!(err.contains("out of i32 range"),
        "expected i32 range error in const, got: {}", err);
}

#[test]
fn int32_mode_rejects_out_of_range_in_impl_method() {
    let src = r#"
        impl Int {
            fn bad(self) -> Int { 2147483648 }
        }
        fn main() -> Int { 0 }
    "#;
    let err = compile_with_int32(src, true).unwrap_err();
    assert!(err.contains("out of i32 range"),
        "expected i32 range error in impl method, got: {}", err);
}

#[test]
fn int32_mode_rejects_out_of_range_in_if_branch() {
    let src = "fn main() -> Int { if true { 2147483648 } else { 0 } }";
    let err = compile_with_int32(src, true).unwrap_err();
    assert!(err.contains("out of i32 range"), "if branch: {}", err);
}

#[test]
fn int32_mode_rejects_out_of_range_in_while_body() {
    let src = "fn main() -> Int { while false { 2147483648; } 0 }";
    let err = compile_with_int32(src, true).unwrap_err();
    assert!(err.contains("out of i32 range"), "while body: {}", err);
}

#[test]
fn int32_mode_rejects_out_of_range_in_closure() {
    let src = "fn main() -> Int { let f = || 2147483648; 0 }";
    let err = compile_with_int32(src, true).unwrap_err();
    assert!(err.contains("out of i32 range"), "closure: {}", err);
}

#[test]
fn int32_mode_rejects_out_of_range_in_array() {
    let src = "fn main() -> Int { let _ = [2147483648]; 0 }";
    let err = compile_with_int32(src, true).unwrap_err();
    assert!(err.contains("out of i32 range"), "array literal: {}", err);
}

#[test]
fn int32_mode_rejects_out_of_range_in_tuple() {
    let src = "fn main() -> Int { let _ = (2147483648, 1); 0 }";
    let err = compile_with_int32(src, true).unwrap_err();
    assert!(err.contains("out of i32 range"), "tuple literal: {}", err);
}

#[test]
fn int32_mode_rejects_out_of_range_in_match_pattern_range() {
    let src = r#"
        fn main() -> Int {
            match 1 {
                0..2147483648 => 0
                _ => 1
            }
        }
    "#;
    let err = compile_with_int32(src, true).unwrap_err();
    assert!(err.contains("out of i32 range"),
        "expected i32 range error in pattern range, got: {}", err);
}
