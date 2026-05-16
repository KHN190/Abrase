use abrase::compiler::Compiler;
use abrase::lexer::Lexer;
use abrase::parser::Parser;
use abrase::typeck::Checker;
use abrase::vm::{Value, VirtualMachine};
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
fn arithmetic_recursion_and_loop() {
    // fib(10) = 55 via recursion; sum_to(10) = 55 via mut + while; total = 110.
    let v = run_file("tests/scripts/arithmetic.abe")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::Int(110));
}

#[test]
fn test_bst() {
    let v = run_file("tests/scripts/bst.abe")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::Int(15));
}

#[test]
fn test_shapes() {
    // record decl + literal + field access + array + indexing + function call
    let v = run_file("tests/scripts/shapes.abe")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::Int(169));
}

#[test]
fn test_memory() {
    // &/* (ref+deref) + Shared (heap alloc/load) + Move (String) + scope-exit drop
    let v = run_file("tests/scripts/memory.abe")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::Int(30));
}

#[test]
fn exceptions_ok_and_err_paths() {
    // pipeline(20,4) hits `?` happy path -> Ok(6); pipeline(10,0) throws -> Err -> 1.
    let v = run_file("tests/scripts/exceptions.abe")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::Int(7));
}

#[test]
fn closures_no_single_and_multi_capture() {
    // no_cap(7)=14 + one_cap(3)=8 (captures x=5) + multi_cap(3)=6 (captures a=1,b=2)
    let v = run_file("tests/scripts/closures.abe")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::Int(28));
}

#[test]
fn closures_default_capture_leaves_outer_binding_live() {
    // Non-move closure captures clone the outer value, so the outer
    // binding remains usable after the closure expression.
    let v = run_file("tests/scripts/closures_move.abe")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::Int(23)); // (3 + 10) + 10
}

#[test]
fn effect_log_runs() {
    let v = run_file("tests/scripts/effect_log.abe")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::Int(42));
}

#[test]
fn effect_handlers_typecheck() {
    // generator (single-shot resume) and backtracking (multi-shot resume) handlers.
    let errs = typeck_file("tests/scripts/effect_handlers.abe");
    assert!(errs.is_empty(),
        "expected no typeck errors for effect handler patterns, got: {:?}", errs);
}

#[test]
fn traits_and_generics() {
    // id<T> specialized at Bool and Int call sites; (5).double() via trait impl = 10.
    // Result: 42 + 10 = 52.
    let v = run_file("tests/scripts/traits_generics.abe")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::Int(52));
}

#[test]
fn neg_move_errors() {
    // Asserts both double-move (let t=s; let u=s) and use-after-move-into-call
    // produce "moved" errors. Expect at least 2 distinct errors.
    let errs = typeck_file("tests/scripts/bad_moves.abe");
    let moved_count = errs.iter().filter(|m| m.contains("moved")).count();
    assert!(moved_count >= 2,
        "expected >=2 'moved' errors (double-move and use-after-move), got {}: {:?}",
        moved_count, errs);
}

#[test]
fn neg_undefined_ident_typeck_errors() {
    let errs = typeck_file("tests/scripts/bad_bare_variant.abe");
    assert!(errs.iter().any(|m| m.contains("Undefined variable") && m.contains("NoSuchName")),
        "expected 'Undefined variable: NoSuchName', got: {:?}", errs);
}

#[test]
fn neg_record_and_array_errors() {
    // Combines unknown record field (`p.z`) and non-Int array index (`arr[true]`).
    let errs = typeck_file("tests/scripts/bad_records_arrays.abe");
    assert!(errs.len() >= 2,
        "expected >=2 errors (unknown field + bad index type), got: {:?}", errs);
}

#[test]
fn neg_borrow_across_effect_typeck_errors() {
    let errs = typeck_file("tests/scripts/bad_borrow_across_effect.abe");
    assert!(errs.iter().any(|m| m.contains("live across effect operation")),
        "expected borrow-barrier error, got: {:?}", errs);
}

#[test]
fn string_interp_with_records_recursion_and_closures() {
    let v = run_file("tests/scripts/interp.abe")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::String(Box::new("user=[Alice:30] total=10 next=11".to_string())));
}
