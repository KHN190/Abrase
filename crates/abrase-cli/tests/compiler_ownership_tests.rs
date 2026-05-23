use abrase::compiler::Compiler;
use abrase::lexer::Lexer;
use abrase::parser::Parser;
use myriad::{Value, VirtualMachine, read_string};

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

fn run_str(source: &str) -> Result<String, String> {
    let mut parser = Parser::new(Lexer::new(source)).with_source(source.to_string());
    let ast = parser.parse_program();
    if !parser.errors.is_empty() {
        return Err(parser.pretty_print_errors());
    }
    let mut compiler = Compiler::new().with_source(source.to_string());
    let module = compiler.compile_module(&ast).map_err(|_| compiler.pretty_print_errors())?;
    let mut vm = VirtualMachine::new();
    let v = vm.run_module(&module)?;
    read_string(vm.heap_ref(), v).ok_or_else(|| format!("expected String handle, got {:?}", v))
}

#[test]
fn copy_int_through_chained_lets() {
    let src = "fn main() -> Int { let x = 5; let y = x; let z = x; z }";
    assert_eq!(run(src), Ok(Value::from_int(5)));
}

#[test]
fn copy_preserves_original_when_y_is_used() {
    let src = "fn main() -> Int { let x = 7; let y = x; x + y }";
    assert_eq!(run(src), Ok(Value::from_int(14)));
}

#[test]
fn copy_int_after_assignment_chain() {
    let src = "fn id(n: Int) -> Int { let m = n; let p = m; p } fn main() -> Int { id(42) }";
    assert_eq!(run(src), Ok(Value::from_int(42)));
}

#[test]
fn ref_unary_produces_reference_value() {
    let src = "fn main() -> Int { let x = 9; let r = &x; x }";
    assert_eq!(run(src), Ok(Value::from_int(9)));
}

#[test]
fn copy_int_multiple_times_returns_sum() {
    let src = "fn main() -> Int { let x = 5; let y = x; let z = x; x + y + z }";
    assert_eq!(run(src), Ok(Value::from_int(15)));
}

#[test]
fn copy_bool_in_bindings() {
    let src = "fn main() -> Int { let b = true; let c = b; let d = b; 1 }";
    assert_eq!(run(src), Ok(Value::from_int(1)));
}

#[test]
fn copy_in_arithmetic_expression() {
    let src = "fn main() -> Int { let a = 10; let b = 20; let c = a; let d = b; c + d }";
    assert_eq!(run(src), Ok(Value::from_int(30)));
}

#[test]
fn reference_in_scope_allows_original_reuse() {
    let src = "fn main() -> Int { let x = 10; let r = &x; x + 5 }";
    assert_eq!(run(src), Ok(Value::from_int(15)));
}

#[test]
fn reference_parameter_in_function() {
    let src = "fn get_one(r: &Int) -> Int { 1 } fn main() -> Int { let x = 42; get_one(&x) }";
    assert_eq!(run(src), Ok(Value::from_int(1)));
}

#[test]
fn drop_at_scope_exit() {
    let src = "fn main() -> Int { let x = 1; { let y = 2; } x }";
    assert_eq!(run(src), Ok(Value::from_int(1)));
}

#[test]
fn move_string_only_once() {
    let src = r#"fn main() -> String { let s = "hello"; let t = s; t }"#;
    assert_eq!(run_str(src), Ok("hello".to_string()));
}

#[test]
fn reassign_clears_moved_flag_simple() {
    // `s` is moved into `t`, then reassigned. The new `s` must be usable.
    let src = r#"
        fn main() -> String {
            let mut s = "a";
            let t = s;
            s = "b";
            s
        }
    "#;
    assert_eq!(run_str(src), Ok("b".to_string()));
}

#[test]
fn reassign_with_self_in_rhs_works() {
    // `s = "{s}y"` moves the old `s` into the interp call, then rebinds `s`
    // to the result. Without the move-clear fix, typeck rejects this.
    let src = r#"
        fn main() -> String {
            let mut s = "x";
            s = "{s}y";
            s
        }
    "#;
    assert_eq!(run_str(src), Ok("xy".to_string()));
}

#[test]
fn reassign_in_while_loop_builds_string() {
    // The full bench-style pattern: mutate a moved value inside a while loop.
    let src = r#"
        fn main() -> String {
            let mut s = "x";
            let mut i = 0;
            while i < 3 {
                s = "{s}y";
                i = i + 1
            };
            s
        }
    "#;
    assert_eq!(run_str(src), Ok("xyyy".to_string()));
}

#[test]
fn ref_to_variant_deref_sums_through_borrow() {
    let src = r#"
        type L = Nil | Cons(Int, L)
        fn sum(xs: &L) -> Int {
            match *xs {
                Nil => 0
                Cons(h, t) => h + sum(&t)
                _ => 0
            }
        }
        fn main() -> Int {
            let lst = Cons(10, Cons(20, Cons(30, Nil)));
            sum(&lst)
        }
    "#;
    assert_eq!(run(src), Ok(Value::from_int(60)));
}

#[test]
fn ref_to_record_deref_reads_fields() {
    let src = r#"
        type Pt = { x: Int, y: Int }
        fn sum(p: &Pt) -> Int { (*p).x + (*p).y }
        fn main() -> Int {
            let p = Pt { x: 10, y: 32 };
            sum(&p)
        }
    "#;
    assert_eq!(run(src), Ok(Value::from_int(42)));
}

#[test]
fn ref_to_variant_passes_through_nested_call() {
    // Stress-test ref/deref consistency: &L flows through two fn boundaries.
    let src = r#"
        type L = Nil | Cons(Int, L)
        fn head(xs: &L) -> Int {
            match *xs {
                Nil => -1
                Cons(h, _) => h
                _ => -1
            }
        }
        fn forward(xs: &L) -> Int { head(xs) }
        fn main() -> Int {
            let lst = Cons(99, Nil);
            forward(&lst)
        }
    "#;
    assert_eq!(run(src), Ok(Value::from_int(99)));
}

#[test]
fn ref_to_int_deref_returns_primitive() {
    // Primitive `&Int` must still wrap (not raw alias), since Int has no handle.
    let src = r#"
        fn deref_int(p: &Int) -> Int { *p }
        fn main() -> Int {
            let n = 42;
            deref_int(&n)
        }
    "#;
    assert_eq!(run(src), Ok(Value::from_int(42)));
}
