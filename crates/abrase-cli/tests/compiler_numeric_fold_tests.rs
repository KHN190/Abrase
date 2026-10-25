use abrase::{compiler::Compiler, lexer::Lexer, parser::Parser};
use myriad::{Runtime, RuntimeValue};

fn evaluate(source: &str, int32: bool) -> Result<RuntimeValue, String> {
    let mut parser = Parser::new(Lexer::new(source)).with_source(source.into());
    let ast = parser.parse_program();
    if !parser.errors.is_empty() { return Err(parser.pretty_print_errors()); }
    let mut compiler = Compiler::new().with_source(source.into()).with_int32_mode(int32).with_inline(false);
    let module = compiler.compile_module(&ast).map_err(|_| compiler.pretty_print_errors())?;
    Runtime::new(&module)?.run_module(&module)
}

fn float_bits(source: &str, int32: bool) -> u64 {
    match evaluate(source, int32).unwrap() {
        RuntimeValue::Word32(value) => value.raw() as u64,
        RuntimeValue::Word64(value) => value.raw(),
    }
}

fn integer(source: &str, int32: bool) -> Result<i64, String> {
    evaluate(source, int32).map(|value| match value {
        RuntimeValue::Word32(value) => value.as_int() as i64,
        RuntimeValue::Word64(value) => value.as_int(),
    })
}

#[test]
fn float_folding_rounds_each_operation_like_the_selected_runtime() {
    let tiny = format!("{:.149}", f32::from_bits(1) as f64);
    let cases = [
        ("16777216.0 + 1.0 - 16777216.0".to_string(), "id(16777216.0) + id(1.0) - id(16777216.0)".to_string()),
        (format!("{tiny} * 0.5 * 2.0"), format!("id({tiny}) * id(0.5) * id(2.0)")),
        ("340282346638528859811704183484516925440.0 * 2.0 / 2.0".to_string(), "id(340282346638528859811704183484516925440.0) * id(2.0) / id(2.0)".to_string()),
    ];
    for int32 in [true, false] {
        for (folded, opaque) in &cases {
            let folded = format!("fn main() -> Float {{ {folded} }}");
            let opaque = format!("fn id(x: Float) -> Float {{ x }} fn main() -> Float {{ {opaque} }}");
            assert_eq!(float_bits(&folded, int32), float_bits(&opaque, int32), "int32={int32}: {folded}");
        }
    }
}

#[test]
fn float_comparisons_and_const_bindings_observe_f32_rounding() {
    let source = "const N: Float = 16777216.0 + 1.0 - 16777216.0; fn main() -> Float { N }";
    assert_eq!(float_bits(source, true), 0.0f32.to_bits() as u64);
    assert_eq!(float_bits(source, false), 1.0f64.to_bits());
    let source = "fn main() -> Bool { 16777216.0 + 1.0 > 16777216.0 }";
    assert_eq!(integer(source, true), Ok(0));
    assert_eq!(integer(source, false), Ok(1));
}

#[test]
fn overflowed_integer_constants_feed_division_and_shifts_at_their_width() {
    let prefix = "const N: Int = 2147483647 + 1;";
    for int32 in [true, false] {
        for operator in ["/ 2", ">> 1", "<< 32"] {
            let folded = format!("{prefix} fn main() -> Int {{ N {operator} }}");
            let opaque = format!("{prefix} fn id(x: Int) -> Int {{ x }} fn main() -> Int {{ id(N) {operator} }}");
            assert_eq!(integer(&folded, int32), integer(&opaque, int32), "int32={int32}: {operator}");
        }
    }
    assert_eq!(integer(&format!("{prefix} fn main() -> Int {{ N / 2 }}"), true), Ok(-1073741824));
    assert_eq!(integer("const N: Bool = 2147483647 + 1 < 0; fn main() -> Bool { N }", true), Ok(1));
}

#[test]
fn constant_and_runtime_division_overflow_both_trap_at_each_width() {
    for (int32, minimum) in [(true, "-2147483647 - 1"), (false, "-9223372036854775807 - 1")] {
        for operator in ["/", "%"] {
            let folded = format!("const N: Int = {minimum}; fn main() -> Int {{ N {operator} -1 }}");
            let opaque = format!("const N: Int = {minimum}; fn id(x: Int) -> Int {{ x }} fn main() -> Int {{ id(N) {operator} id(-1) }}");
            assert!(integer(&opaque, int32).is_err(), "opaque {int32} {operator}");
            assert!(integer(&folded, int32).is_err(), "folded {int32} {operator}");
        }
    }
}
