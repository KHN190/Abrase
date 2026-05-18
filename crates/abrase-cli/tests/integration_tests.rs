use abrase::compiler::Compiler;
use abrase::lexer::Lexer;
use abrase::parser::Parser;
use abrase::typeck::Checker;
use myriad::{BoxedValue, Value, VirtualMachine};
use std::fs;

fn run_file(path: &str) -> Result<Value, String> {
    let (v, _) = run_file_full(path)?;
    Ok(v)
}

fn run_file_string(path: &str) -> Result<String, String> {
    let (v, vm) = run_file_full(path)?;
    let idx = v.as_box().ok_or_else(|| format!("expected box, got {:?}", v))?;
    match vm.box_pool().get(idx) {
        Some(BoxedValue::String(s)) => Ok(s.clone()),
        other => Err(format!("expected string, got {:?}", other)),
    }
}

fn run_file_full(path: &str) -> Result<(Value, VirtualMachine), String> {
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
    let v = vm.run_module(&module)
        .map_err(|e| format!("VM error: {}", e))?;
    Ok((v, vm))
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
    assert_eq!(v, Value::from_int(110));
}

#[test]
fn test_bst() {
    let v = run_file("tests/scripts/bst.abe")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(15));
}

#[test]
fn test_shapes() {
    // record decl + literal + field access + array + indexing + function call
    let v = run_file("tests/scripts/shapes.abe")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(169));
}

#[test]
fn test_memory() {
    // &/* (ref+deref) + Shared (heap alloc/load) + Move (String) + scope-exit drop
    let v = run_file("tests/scripts/memory.abe")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(30));
}

#[test]
fn exceptions_ok_and_err_paths() {
    // pipeline(20,4) hits `?` happy path -> Ok(6); pipeline(10,0) throws -> Err -> 1.
    let v = run_file("tests/scripts/exceptions.abe")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(7));
}

#[test]
fn closures_no_single_and_multi_capture() {
    // no_cap(7)=14 + one_cap(3)=8 (captures x=5) + multi_cap(3)=6 (captures a=1,b=2)
    let v = run_file("tests/scripts/closures.abe")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(28));
}

#[test]
fn effect_dispatch_runs() {
    let v = run_file("tests/scripts/effect_dispatch.abe")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(22));
}

#[test]
fn multiple_suspension_points() {
    let v = run_file("tests/scripts/multi_effect.abe")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(20));
}

#[test]
fn region_all_allowed_shapes() {
    let (v, vm) = run_file_full("tests/scripts/region.abe")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(382),
        "11 (a) + 20 (b) + 300 (c) + 30 (d) + 6 (e) + 7 (f) + 8 (g)");
    assert_eq!(vm.heap_live_count(), 0,
        "all region-tagged allocs must be force-freed at exit, got live={}",
        vm.heap_live_count());
}

#[test]
fn effect_handlers_typecheck() {
    let errs = typeck_file("tests/scripts/effect_handlers.abe");
    assert!(errs.is_empty(),
        "expected no typeck errors for effect handler patterns, got: {:?}", errs);
}

#[test]
fn traits_and_generics() {
    let v = run_file("tests/scripts/traits_generics.abe")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(52));
}

#[test]
fn string_interp_with_records_recursion_and_closures() {
    let v = run_file_string("tests/scripts/interp.abe")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, "user=[Alice:30] total=10 next=11");
}

#[test]
fn built_ins() {
    // The bare `run_file` helper uses a no-device VirtualMachine; native fns
    // like print / now / rand need Console + Clock + Random installed, so we
    // go through real VirtualMachine.
    let src = fs::read_to_string("tests/scripts/built_ins.abe")
        .expect("built_ins.abe missing");
    let (mut rt, console) = abrase_cli::host::Runtime::new_for_tests();
    let v = rt.eval(&src).unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(0), "main should return 0");
    let (out_handle, _) = console.handles();
    let out = String::from_utf8(out_handle.borrow().clone()).expect("stdout utf-8");
    assert!(out.contains("hello, myriad"),    "missing greeting in: {:?}", out);
    assert!(out.contains("slept ~"),          "missing sleep line in: {:?}", out);
    assert!(out.contains("rand = "),          "missing rand line in: {:?}", out);
    assert!(out.contains("min=3? 3"),         "math min broken in: {:?}", out);
    assert!(out.contains("max=7? 7"),         "math max broken in: {:?}", out);
    assert!(out.contains("abs=9? 9"),         "math abs broken in: {:?}", out);
    assert!(out.contains("sqrt(16)=4"),       "sqrt broken in: {:?}", out);
    assert!(out.contains("ceil(2.3)=3"),      "ceil broken in: {:?}", out);
    assert!(out.contains("flr(2.7)=2"),       "flr broken in: {:?}", out);
    assert!(out.contains("fabs(3.5)=3.5"),    "abs(Float) overload broken in: {:?}", out);
    assert!(out.contains("fmax(2.5)=2.5"),    "max(Float) overload broken in: {:?}", out);
    assert!(out.contains("fmin(1.5)=1.5"),    "min(Float) overload broken in: {:?}", out);
}
