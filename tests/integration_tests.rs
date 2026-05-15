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
    assert!(parser.errors.is_empty(),
        "unexpected parse errors in {}: {}", path, parser.pretty_print_errors());
    let mut checker = Checker::new();
    checker.check_program(&ast);
    checker.errors.iter().map(|e| e.message.clone()).collect()
}

#[test]
fn test_fibonacci() {
    // recursion + if/else + arithmetic + function call
    let v = run_file("tests/scripts/fibonacci.ect")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::Int(55));
}

#[test]
fn test_sum_loop() {
    // while + mut + assignment + comparison
    let v = run_file("tests/scripts/sum_loop.ect")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::Int(55));
}

#[test]
fn test_bst() {
    // variant decl + payload + match patterns with binding + wildcard
    // + recursion + conditional construction
    let v = run_file("tests/scripts/bst.ect")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::Int(15));
}

#[test]
fn test_shapes() {
    // record decl + literal + field access + array + indexing + function call
    let v = run_file("tests/scripts/shapes.ect")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::Int(169));
}

#[test]
fn test_memory() {
    // &/* (ref+deref) + Shared (heap alloc/load) + Move (String) + scope-exit drop
    let v = run_file("tests/scripts/memory.ect")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::Int(30));
}

#[test]
fn neg_double_move_string_typeck_errors() {
    let errs = typeck_file("tests/scripts/bad_double_move.ect");
    assert!(errs.iter().any(|m| m.contains("moved")),
        "expected 'moved' error from double-move on String, got: {:?}", errs);
}

#[test]
fn neg_use_after_move_into_call_typeck_errors() {
    let errs = typeck_file("tests/scripts/bad_move_then_use.ect");
    assert!(errs.iter().any(|m| m.contains("moved")),
        "expected 'moved' error from use-after-move into call, got: {:?}", errs);
}

#[test]
fn neg_unknown_record_field_typeck_errors() {
    let errs = typeck_file("tests/scripts/bad_unknown_field.ect");
    assert!(!errs.is_empty(),
        "expected error for unknown record field, got no errors");
}

#[test]
fn neg_array_index_wrong_type_typeck_errors() {
    let errs = typeck_file("tests/scripts/bad_array_index_type.ect");
    assert!(!errs.is_empty(),
        "expected error for non-Int array index, got no errors");
}
