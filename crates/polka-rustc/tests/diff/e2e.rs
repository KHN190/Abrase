use crate::common::*;
use abrase::{compiler::Compiler, lexer::Lexer, parser::Parser};
use polka_rustc::transpile_module;
use myriad::VirtualMachine;

fn diff_module(module: &polka::Module, label: &str) {
    let mut vm = VirtualMachine::new().with_step_cap(1_000_000);
    myriad::Host::default().install_into(&mut vm);
    let i = match vm.run_module(module) {
        Ok(v) => Outcome::Ok(v.raw()),
        Err(e) => Outcome::Err(e),
    };
    let i_live = vm.heap_live_count();

    let tsrc = transpile_module(module)
        .unwrap_or_else(|e| panic!("transpile unsupported for in-subset program: {:?}\n{}", e, label));
    let (t, t_live) = compile_run_full(&tsrc);
    compare(&i, &t);
    if let Outcome::Ok(_) = i {
        assert_eq!(i_live, t_live, "e2e heap live-count mismatch: interp={} transpiled={}\n{}", i_live, t_live, label);
    }
}

fn e2e(src: &str) {
    let mut p = Parser::new(Lexer::new(src)).with_source(src.to_string());
    let decls = p.parse_program();
    assert!(p.errors.is_empty(), "parse errors: {:?}", p.errors);
    let module = Compiler::new().compile_module(&decls).unwrap_or_else(|errs| {
        panic!("compile errors: {}", errs.iter()
            .map(|e| format!("{:?}: {}", e.code, e.message)).collect::<Vec<_>>().join("\n"))
    });
    diff_module(&module, src);
}

fn e2e_files(entry: &str, files: &[(&str, &str)]) {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!("polka_e2e_{}_{}", std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)));
    std::fs::create_dir_all(&dir).unwrap();
    for (name, src) in files {
        std::fs::write(dir.join(name), src).unwrap();
    }
    let loaded = abrase::loader::load_program(&dir.join(entry))
        .unwrap_or_else(|e| panic!("load error: {:?}", e));
    let module = Compiler::new().with_source(loaded.entry_source.clone())
        .compile_module(&loaded.decls)
        .unwrap_or_else(|_| panic!("compile errors in multi-module program"));
    diff_module(&module, entry);
}

// @cart: persistent main yields a frame via `frame.present()` and resumes mid-
// loop. AOT lowers the entry fn to a resumable state machine; here we drive the
// interpreter frame-by-frame and assert the transpiled binary prints the same.
#[test]
fn e2e_cart_frame_counter() {
    let src = r#"
@cart
fn main() -> <frame> Unit {
  let mut total = 0;
  let mut i = 0;
  while i < 3 {
    total = total + 3;
    frame.present();
    println(i.to_s());
    i = i + 1
  };
  println(total.to_s());
  halt(0)
}
"#;
    let mut p = Parser::new(Lexer::new(src)).with_source(src.to_string());
    let decls = p.parse_program();
    assert!(p.errors.is_empty(), "parse errors: {:?}", p.errors);
    let module = Compiler::new().compile_module(&decls).unwrap_or_else(|errs| {
        panic!("compile errors: {}", errs.iter().map(|e| format!("{:?}: {}", e.code, e.message)).collect::<Vec<_>>().join("\n"))
    });

    let console = myriad::devices::BufferConsole::new();
    let (out, _) = console.handles();
    let mut vm = VirtualMachine::new().with_step_cap(1_000_000);
    myriad::Host::default().with_console(Box::new(console)).install_into(&mut vm);
    vm.run_to_yield(&module).expect("run_to_yield");
    while vm.resume(&module, myriad::Value::from_int(0)).expect("resume") {}
    let interp_out = String::from_utf8(out.borrow().clone()).unwrap();

    let tsrc = transpile_module(&module).expect("transpile @cart");
    let full = compile_run_raw(&tsrc);
    // Drop the trailing `OK <v> <live>` status line; the rest is program output.
    let aot_out: String = full.lines().filter(|l| !l.starts_with("OK ") && !l.starts_with("ERR "))
        .map(|l| format!("{}\n", l)).collect();

    assert_eq!(interp_out, aot_out, "frame output mismatch:\ninterp={:?}\naot={:?}", interp_out, aot_out);
}

#[test]
fn e2e_multi_module_call() {
    e2e_files("main.abe", &[
        ("lib.abe", "pub fn add(a: Int, b: Int) -> Int { a + b }\n"),
        ("main.abe", "use lib::{ add }\nfn main() -> Int { add(20, 22) }\n"),
    ]);
}

#[test]
fn e2e_multi_module_static_and_fn() {
    e2e_files("main.abe", &[
        ("lib.abe", "pub static BASE: Int = 100\npub fn dbl(x: Int) -> Int { x * 2 }\n"),
        ("main.abe", "use lib::{ BASE, dbl }\nfn main() -> Int { BASE + dbl(21) }\n"),
    ]);
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
fn e2e_float_dataflow() {
    e2e(r#"
        fn scale(x: Float, k: Float) -> Float { x * k + x }
        fn main() -> Float {
            let a = 1.5;
            let b = a * 2.0;
            let c = scale(b, 0.5);
            c - a
        }
    "#);
}

#[test]
fn e2e_string_interp_concat_heap() {
    e2e(r#"
        fn main() -> Int {
            let a = "foo";
            let b = "bar";
            let c = "{a}-{b}-{a}";
            println(c);
            0
        }
    "#);
}

#[test]
fn e2e_string_concat_returned_handle() {
    e2e(r#"
        fn join(a: String, b: String) -> String { "{a}{b}" }
        fn main() -> Int {
            let s = join("hello", "world");
            println(s);
            7
        }
    "#);
}

#[test]
fn e2e_int_to_s_native_heap() {
    e2e(r#"
        fn main() -> Int {
            let n = 42;
            let s = n.to_s();
            println(s);
            n
        }
    "#);
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
