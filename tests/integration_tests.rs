use ect::compiler::Compiler;
use ect::lexer::Lexer;
use ect::parser::Parser;
use ect::typeck::Checker;
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

fn typeck_file(path: &str) -> Vec<String> {
    let source = fs::read_to_string(path).expect("script missing");
    let mut parser = Parser::new(Lexer::new(&source)).with_source(source.clone());
    let ast = parser.parse_program();
    assert!(parser.errors.is_empty(), "unexpected parse errors in {}: {}", path, parser.pretty_print_errors());
    let mut checker = Checker::new();
    checker.check_program(&ast);
    checker.errors.iter().map(|e| e.message.clone()).collect()
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

#[test]
fn test_borrow_and_deref() {
    let v = run_file("tests/scripts/borrow.ect")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::Int(42));
}

#[test]
fn test_shared_alloc() {
    let v = run_file("tests/scripts/shared.ect")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::Int(7));
}

#[test]
fn test_drop_at_scope_exit() {
    let v = run_file("tests/scripts/drop_scope.ect")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::String("outer".to_string()));
}

#[test]
fn neg_double_move_string_typeck_errors() {
    let errs = typeck_file("tests/scripts/bad_double_move.ect");
    assert!(
        errs.iter().any(|m| m.contains("moved")),
        "expected 'moved' error from double-move on String, got: {:?}",
        errs
    );
}

#[test]
fn neg_use_after_move_into_call_typeck_errors() {
    let errs = typeck_file("tests/scripts/bad_move_then_use.ect");
    assert!(
        errs.iter().any(|m| m.contains("moved")),
        "expected 'moved' error from use-after-move into call, got: {:?}",
        errs
    );
}

#[test]
fn test_record_field_access() {
    let v = run_file("tests/scripts/point.ect")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::Int(7));
}

#[test]
fn test_variant_match() {
    let v = run_file("tests/scripts/color_variant.ect")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::Int(15));
}

#[test]
fn test_array_index() {
    let v = run_file("tests/scripts/array_index.ect")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::Int(20));
}

#[test]
fn neg_unknown_record_field_typeck_errors() {
    let errs = typeck_file("tests/scripts/bad_unknown_field.ect");
    assert!(
        !errs.is_empty(),
        "expected error for unknown record field, got no errors"
    );
}

#[test]
fn neg_array_index_wrong_type_typeck_errors() {
    let errs = typeck_file("tests/scripts/bad_array_index_type.ect");
    assert!(
        !errs.is_empty(),
        "expected error for non-Int array index, got no errors"
    );
}
