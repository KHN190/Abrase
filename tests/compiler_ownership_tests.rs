use ect::compiler::Compiler;
use ect::lexer::Lexer;
use ect::parser::Parser;
use ect::vm::{Value, VirtualMachine};

fn run(source: &str) -> Result<Value, String> {
    let mut parser = Parser::new(Lexer::new(source)).with_source(source.to_string());
    let ast = parser.parse_program();
    if !parser.errors.is_empty() {
        return Err(parser.pretty_print_errors());
    }
    let mut compiler = Compiler::new().with_source(source.to_string());
    let module = compiler.compile_module(&ast).map_err(|_| compiler.pretty_print_errors())?;
    let mut vm = VirtualMachine::new();
    vm.run_module(&module)
}

#[test]
fn copy_int_through_chained_lets() {
    let src = "fn main() -> Int { let x = 5; let y = x; let z = x; z }";
    assert_eq!(run(src), Ok(Value::Int(5)));
}

#[test]
fn copy_preserves_original_when_y_is_used() {
    let src = "fn main() -> Int { let x = 7; let y = x; x + y }";
    assert_eq!(run(src), Ok(Value::Int(14)));
}

#[test]
fn move_string_into_new_binding() {
    let src = r#"fn main() -> String { let s = "hi"; let t = s; t }"#;
    assert_eq!(run(src), Ok(Value::String("hi".to_string())));
}

#[test]
fn copy_int_after_assignment_chain() {
    let src = "fn id(n: Int) -> Int { let m = n; let p = m; p } fn main() -> Int { id(42) }";
    assert_eq!(run(src), Ok(Value::Int(42)));
}

#[test]
fn ref_unary_produces_reference_value() {
    let src = "fn main() -> Int { let x = 9; let r = &x; x }";
    assert_eq!(run(src), Ok(Value::Int(9)));
}

#[test]
fn copy_int_multiple_times_returns_sum() {
    let src = "fn main() -> Int { let x = 5; let y = x; let z = x; x + y + z }";
    assert_eq!(run(src), Ok(Value::Int(15)));
}

#[test]
fn copy_bool_in_bindings() {
    let src = "fn main() -> Int { let b = true; let c = b; let d = b; 1 }";
    assert_eq!(run(src), Ok(Value::Int(1)));
}

#[test]
fn copy_in_arithmetic_expression() {
    let src = "fn main() -> Int { let a = 10; let b = 20; let c = a; let d = b; c + d }";
    assert_eq!(run(src), Ok(Value::Int(30)));
}

#[test]
fn reference_in_scope_allows_original_reuse() {
    let src = "fn main() -> Int { let x = 10; let r = &x; x + 5 }";
    assert_eq!(run(src), Ok(Value::Int(15)));
}

#[test]
fn reference_parameter_in_function() {
    let src = "fn get_one(r: &Int) -> Int { 1 } fn main() -> Int { let x = 42; get_one(&x) }";
    assert_eq!(run(src), Ok(Value::Int(1)));
}

#[test]
fn copy_preferred_over_borrow() {
    let src = "fn main() -> Int { let x = 100; let y = x; let z = x; y + z }";
    assert_eq!(run(src), Ok(Value::Int(200)));
}

#[test]
fn drop_at_scope_exit() {
    let src = "fn main() -> Int { let x = 1; { let y = 2; } x }";
    assert_eq!(run(src), Ok(Value::Int(1)));
}

#[test]
fn move_string_only_once() {
    let src = r#"fn main() -> String { let s = "hello"; let t = s; t }"#;
    assert_eq!(run(src), Ok(Value::String("hello".to_string())));
}
