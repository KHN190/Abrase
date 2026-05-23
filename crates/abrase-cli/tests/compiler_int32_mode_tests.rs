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
