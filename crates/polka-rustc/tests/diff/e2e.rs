use crate::common::*;
use abrase::{compiler::Compiler, lexer::Lexer, parser::Parser};
use polka_rustc::transpile_module;
use myriad::VirtualMachine;

fn e2e(src: &str) {
    let mut p = Parser::new(Lexer::new(src)).with_source(src.to_string());
    let decls = p.parse_program();
    assert!(p.errors.is_empty(), "parse errors: {:?}", p.errors);
    let module = Compiler::new().compile_module(&decls).unwrap_or_else(|errs| {
        panic!("compile errors: {}", errs.iter()
            .map(|e| format!("{:?}: {}", e.code, e.message)).collect::<Vec<_>>().join("\n"))
    });

    let mut vm = VirtualMachine::new().with_step_cap(1_000_000);
    myriad::Host::default().install_into(&mut vm);
    let i = match vm.run_module(&module) {
        Ok(v) => Outcome::Ok(v.raw()),
        Err(e) => Outcome::Err(e),
    };
    let i_live = vm.heap_live_count();

    let tsrc = transpile_module(&module)
        .unwrap_or_else(|e| panic!("transpile unsupported for in-subset program: {:?}\n{}", e, src));
    let (t, t_live) = compile_run_full(&tsrc);
    compare(&i, &t);
    if let Outcome::Ok(_) = i {
        assert_eq!(i_live, t_live, "e2e heap live-count mismatch: interp={} transpiled={}\nsrc:\n{}", i_live, t_live, src);
    }
}

#[test]
fn e2e_arithmetic_precedence() {
    e2e("fn main() -> Int { 2 + 3 * 4 - 1 }");
}

#[test]
fn e2e_let_bindings() {
    e2e("fn main() -> Int { let x: Int = 5; let y = x * x; y + 1 }");
}

#[test]
fn e2e_unary_neg() {
    e2e("fn main() -> Int { let x: Int = 7; -x }");
}

#[test]
fn e2e_recursive_factorial() {
    e2e(r#"
        fn fact(n: Int) -> Int { if n <= 1 { 1 } else { n * fact(n - 1) } }
        fn main() -> Int { fact(6) }
    "#);
}

#[test]
fn e2e_loop_with_break_value() {
    e2e(r#"
        fn main() -> Int {
            let mut i = 0;
            loop {
                if i == 5 { break i };
                i = i + 1
            }
        }
    "#);
}

#[test]
fn e2e_tuple_index() {
    e2e("fn main() -> Int { let t = (10, 20, 30); t[1] }");
}

#[test]
fn e2e_record_field_sum_heap() {
    e2e(r#"
        type Pt = { x: Int, y: Int }
        fn sum(p: &Pt) -> Int { (*p).x + (*p).y }
        fn main() -> Int {
            let p = Pt { x: 10, y: 32 };
            sum(&p)
        }
    "#);
}

#[test]
fn e2e_float_arithmetic() {
    e2e("fn main() -> Float { 1.5 * 2.0 + 0.25 }");
}

#[test]
fn e2e_println_string_builtin() {
    e2e(r#"fn main() -> Int { println("hi from aot"); 7 }"#);
}

#[test]
fn e2e_nested_conditionals() {
    e2e(r#"
        fn classify(n: Int) -> Int {
            if n < 0 { 0 } else { if n == 0 { 1 } else { 2 } }
        }
        fn main() -> Int { classify(0 - 5) + classify(0) + classify(9) }
    "#);
}
