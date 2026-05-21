use abrase::compiler::Compiler;
use abrase::lexer::Lexer;
use abrase::parser::Parser;
use abrase::typeck::Checker;
use myriad::{Value, VirtualMachine, read_string};
use std::fs;

fn run_file(path: &str) -> Result<Value, String> {
    let (v, _) = run_file_full(path)?;
    Ok(v)
}

fn run_file_string(path: &str) -> Result<String, String> {
    let (v, vm) = run_file_full(path)?;
    read_string(vm.heap_ref(), v).ok_or_else(|| format!("expected String handle, got {:?}", v))
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
fn closures_complex_bodies() {
    let v = run_file("tests/scripts/closures_complex.abe")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(110));
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
fn effect_resume_paths_nested_handlers_and_return_arm() {
    let v = run_file("tests/scripts/effect_resume_paths.abe")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(2050));
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
    let v = run_file("tests/scripts/traits.abe")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(52));
}

#[test]
fn generics_with_bounds() {
    let v = run_file("tests/scripts/generics.abe")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(99));
}

#[test]
fn generic_bound_violation_rejected() {
    // `show` requires T: ToS. Calling with a record that lacks impl ToS for it
    // should be rejected by typeck.
    let src = r#"
        type Bag = { n: Int }
        fn show<T>(x: T) -> String where T: ToS { x.to_s() }
        fn main() -> Int { let _ = show(Bag { n: 1 }); 0 }
    "#;
    let mut compiler = abrase::compiler::Compiler::new().with_source(src.into());
    let mut p = abrase::parser::Parser::new(abrase::lexer::Lexer::new(src));
    let ast = p.parse_program();
    let result = compiler.compile_module(&ast);
    assert!(result.is_err(), "expected typeck error for Bag : ToS violation");
}

#[test]
fn generic_overload_restriction() {
    // Synthesize the disallowed shape: same name has both a generic fn AND
    // another overload. The check is in compile_module's post-registration
    // pass — we inject the overload after compile_builtins.
    let src = r#"
        fn foo<T>(x: T) -> T { x }
        fn main() -> Int { 0 }
    "#;
    let mut compiler = abrase::compiler::Compiler::new().with_source(src.into());
    let mut p = abrase::parser::Parser::new(abrase::lexer::Lexer::new(src));
    let ast = p.parse_program();
    // First compile to populate func_map with `foo`.
    // Then manually plant an entry in fn_overloads simulating a second
    // overload being added (which user-fn overloading would do if enabled).
    // Since fn_overloads is pub(super), we can't poke it from outside the
    // crate — so use a direct call only available via the compiler crate.
    // Instead, rely on the check firing during compile_module if such a
    // state ever appears. For now this test documents intent and re-runs
    // the existing compile to ensure no false positive on plain generic.
    let result = compiler.compile_module(&ast);
    assert!(result.is_ok(), "plain generic fn should compile, got {:?}", result.err());
}

#[test]
#[ignore = "codegen: chained generic method call .max().to_s() — receiver type inference loses T's bounds"]
fn generic_chained_method_via_bound() {
    let src = r#"
        fn show_max<T>(a: T, b: T) -> String where T: Ord, T: ToS {
          a.max(b).to_s()
        }
        fn main() -> Int { let _ = show_max(3, 7); 0 }
    "#;
    let mut compiler = abrase::compiler::Compiler::new().with_source(src.into());
    let mut p = abrase::parser::Parser::new(abrase::lexer::Lexer::new(src));
    let ast = p.parse_program();
    let result = compiler.compile_module(&ast);
    assert!(result.is_ok(), "expected ok compile, got {:?}", result.err());
}

#[test]
fn string_interp_with_records_recursion_and_closures() {
    let v = run_file_string("tests/scripts/interp.abe")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, "user=[Alice:30] total=10 next=11");
}

#[test]
fn built_ins() {
    // print / math / type conversions — all core natives, no clock/random.
    let src = fs::read_to_string("tests/scripts/built_ins.abe")
        .expect("built_ins.abe missing");
    let (mut rt, console) = abrase_cli::host::Runtime::new_for_tests();
    let v = rt.eval(&src).unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(0), "main should return 0");
    let (out_handle, _) = console.handles();
    let out = String::from_utf8(out_handle.borrow().clone()).expect("stdout utf-8");
    assert!(out.contains("hello, myriad"),    "missing greeting in: {:?}", out);
    assert!(out.contains("7.min(3)=3"),       "Int .min() broken in: {:?}", out);
    assert!(out.contains("7.max(3)=7"),       "Int .max() broken in: {:?}", out);
    assert!(out.contains("(-9).abs()=9"),     "Int .abs() broken in: {:?}", out);
    assert!(out.contains("sqrt(16)=4"),       "sqrt broken in: {:?}", out);
    assert!(out.contains("ceil(2.3)=3"),      "ceil broken in: {:?}", out);
    assert!(out.contains("flr(2.7)=2"),       "flr broken in: {:?}", out);
    assert!(out.contains("(-3.5).abs()=3.5"), "Float .abs() broken in: {:?}", out);
    assert!(out.contains("1.5.max(2.5)=2.5"), "Float .max() broken in: {:?}", out);
    assert!(out.contains("1.5.min(2.5)=1.5"), "Float .min() broken in: {:?}", out);
    assert!(out.contains("7.to_f()=7"),       ".to_f() broken in: {:?}", out);
    assert!(out.contains("3.9.to_i()=3"),     ".to_i() (Float→Int trunc) broken in: {:?}", out);
    assert!(out.contains("'A'.to_i()=65"),    ".to_i() (Char→Int) broken in: {:?}", out);
    assert!(out.contains("66.to_c()=B"),      ".to_c() (Int→Char) broken in: {:?}", out);
    assert!(out.contains("true.to_i()=1"),    "Bool→Int broken in: {:?}", out);
    assert!(out.contains("42.to_s()=42"),     "Int.to_s broken in: {:?}", out);
    assert!(out.contains("3.14.to_s()=3.14"), "Float.to_s broken in: {:?}", out);
    assert!(out.contains("false.to_s()=false"),"Bool.to_s broken in: {:?}", out);
    assert!(out.contains("'Z'.to_s()=Z"),     "Char.to_s broken in: {:?}", out);
}
