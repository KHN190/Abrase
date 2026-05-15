use ect::compiler::Compiler;
use ect::lexer::Lexer;
use ect::parser::Parser;
use ect::vm::{Value, VirtualMachine};
use std::fs;

fn run_file(path: &str) -> Result<Value, String> {
    let source = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let mut parser = Parser::new(Lexer::new(&source)).with_source(source.clone());
    let ast = parser.parse_program();

    if !parser.errors.is_empty() {
        return Err(format!("Parser errors:\n{}", parser.pretty_print_errors()));
    }

    if ast.is_empty() {
        return Err("Parser produced empty AST".to_string());
    }

    let mut compiler = Compiler::new().with_source(source);
    let module = compiler.compile_module(&ast)
        .map_err(|_| compiler.pretty_print_errors())?;

    let mut vm = VirtualMachine::new();
    vm.run_module(&module)
        .map_err(|e| format!("VM error: {}", e))
}

#[test]
fn test_fibonacci() {
    let v = run_file("tests/scripts/fibonacci.ect")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::Int(55));
}

#[test]
fn test_factorial() {
    let v = run_file("tests/scripts/factorial.ect")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::Int(120));
}

#[test]
fn test_sum_loop() {
    let v = run_file("tests/scripts/sum_loop.ect")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::Int(55));
}

#[test]
fn test_power() {
    let v = run_file("tests/scripts/power.ect")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::Int(1024));
}

#[test]
fn test_nested_conditionals() {
    let v = run_file("tests/scripts/classify.ect")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::Int(2));
}

#[test]
fn test_mutual_recursion() {
    let v = run_file("tests/scripts/mutual_recursion.ect")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::Int(1));
}

#[test]
fn test_match_dispatch() {
    let v = run_file("tests/scripts/quadrant.ect")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::Int(3));
}

#[test]
fn test_move_string() {
    let v = run_file("tests/scripts/move_string.ect")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::String("hi".to_string()));
}
