use crate::common::*;
use abrase::{compiler::Compiler, lexer::Lexer, parser::Parser};
use polka_rustc::transpile_module;
use myriad::VirtualMachine;

fn module_of_src(src: &str) -> polka::Module {
    let mut p = Parser::new(Lexer::new(src)).with_source(src.to_string());
    let decls = p.parse_program();
    assert!(p.errors.is_empty(), "parse errors: {:?}", p.errors);
    Compiler::new().compile_module(&decls).unwrap_or_else(|errs| {
        panic!("compile errors: {}", errs.iter()
            .map(|e| format!("{:?}: {}", e.code, e.message)).collect::<Vec<_>>().join("\n"))
    })
}

#[test]
fn hybrid_pure_leaf_runs_native_in_effect_module() {
    let src = r#"
        effect E { op tick() -> Unit }
        fn fib(n: Int) -> Int { if n < 2 { n } else { fib(n - 1) + fib(n - 2) } }
        fn body() -> <E> Int { E.tick(); fib(10) }
        fn main() -> Int {
            handle body() {
                return r  => r,
                E.tick _  => resume(())
            }
        }
    "#;
    let module = module_of_src(src);

    let mut vm = VirtualMachine::new().with_step_cap(1_000_000);
    myriad::Host::default().install_into(&mut vm);
    let i = match vm.run_module(&module) {
        Ok(v) => Outcome::Ok(v.raw()),
        Err(e) => Outcome::Err(e),
    };
    let i_live = vm.heap_live_count();

    let tsrc = transpile_module(&module).expect("transpile effect module");
    assert!(tsrc.contains("match pc"), "hybrid must emit a native body for the pure leaf");

    let (t, t_live) = compile_run_full(&tsrc);
    compare(&i, &t);
    if let Outcome::Ok(_) = i {
        assert_eq!(i_live, t_live, "hybrid heap live-count mismatch: interp={} transpiled={}", i_live, t_live);
    }
}

#[test]
fn lib_emit_exposes_host_injectable_items_no_main() {
    let src = r#"
        effect E { op tick() -> Unit }
        fn fib(n: Int) -> Int { if n < 2 { n } else { fib(n - 1) + fib(n - 2) } }
        fn body() -> <E> Int { E.tick(); fib(10) }
        fn main() -> Int {
            handle body() {
                return r  => r,
                E.tick _  => resume(())
            }
        }
    "#;
    let module = module_of_src(src);
    let lib = polka_rustc::transpile_module_lib(&module).expect("lib emit");
    assert!(lib.contains("pub const PK"), "lib must expose PK for the host to read_pk");
    assert!(lib.contains("pub fn register_aot"), "lib must expose register_aot for host VM");
    assert!(!lib.contains("fn main"), "lib must not emit a main; host owns the entry");
}

#[test]
fn lib_emit_native_module_exposes_run_no_main() {
    let src = r#"
        fn fib(n: Int) -> Int { if n < 2 { n } else { fib(n - 1) + fib(n - 2) } }
        fn main() -> Int { fib(10) }
    "#;
    let module = module_of_src(src);
    let lib = polka_rustc::transpile_module_lib(&module).expect("native lib emit");
    assert!(lib.contains("pub fn run("), "native lib must expose run for the host to drive");
    assert!(lib.contains("&mut dyn myriad::AotNatives"), "run must take host-agnostic AotNatives");
    assert!(!lib.contains("fn main"), "lib must not emit a main; host owns the entry");
}

#[test]
fn lib_emit_cart_exposes_per_frame_step_api() {
    let src = r#"
        @cart
        fn main() -> <frame> Unit {
            let mut i = 0;
            while i < 3 { frame.present(); i = i + 1 };
            halt(0)
        }
    "#;
    let module = module_of_src(src);
    let lib = polka_rustc::transpile_module_lib(&module).expect("cart lib emit");
    assert!(lib.contains("pub type CartState"), "host needs a stable cart-state type");
    assert!(lib.contains("pub fn cart_setup("), "host needs setup to build consts + run module_init");
    assert!(lib.contains("pub fn cart_step("), "host needs a per-frame step entry");
    assert!(!lib.contains("fn main"), "lib must not emit a main; host drives the frame loop");
}

fn diff_hybrid(src: &str) {
    let module = module_of_src(src);
    let mut vm = VirtualMachine::new().with_step_cap(1_000_000);
    myriad::Host::default().install_into(&mut vm);
    let i = match vm.run_module(&module) {
        Ok(v) => Outcome::Ok(v.raw()),
        Err(e) => Outcome::Err(e),
    };
    let i_live = vm.heap_live_count();
    let tsrc = transpile_module(&module).expect("transpile");
    let (t, t_live) = compile_run_full(&tsrc);
    compare(&i, &t);
    if let Outcome::Ok(_) = i {
        assert_eq!(i_live, t_live, "hybrid live-count mismatch: interp={} transpiled={}", i_live, t_live);
    }
}

#[test]
fn hybrid_math_builtin_leaf_inlined_native() {
    let src = r#"
        effect E { op tick() -> Unit }
        fn acc(x: Float, n: Int) -> Float { if n <= 0 { x } else { acc(sqrt(x) + 1.0, n - 1) } }
        fn body() -> <E> Float { E.tick(); acc(100.0, 5) }
        fn main() -> Float {
            handle body() {
                return r  => r,
                E.tick _  => resume(())
            }
        }
    "#;
    let module = module_of_src(src);

    let mut vm = VirtualMachine::new().with_step_cap(1_000_000);
    myriad::Host::default().install_into(&mut vm);
    let i = match vm.run_module(&module) {
        Ok(v) => Outcome::Ok(v.raw()),
        Err(e) => Outcome::Err(e),
    };
    let i_live = vm.heap_live_count();

    let tsrc = transpile_module(&module).expect("transpile effect module");
    assert!(tsrc.contains("fmath::sqrt"), "math builtin must be inlined native, not host.call");

    let (t, t_live) = compile_run_full(&tsrc);
    compare(&i, &t);
    if let Outcome::Ok(_) = i {
        assert_eq!(i_live, t_live, "hybrid math live-count mismatch: interp={} transpiled={}", i_live, t_live);
    }
}

#[test]
fn hybrid_heap_handle_leaf() {
    diff_hybrid(r#"
        effect E { op tick() -> Unit }
        type Pt = { x: Int, y: Int }
        fn mk(a: Int, b: Int) -> Pt { Pt { x: a, y: b } }
        fn sumpt(p: &Pt) -> Int { (*p).x + (*p).y }
        fn body() -> <E> Int { E.tick(); let p = mk(10, 32); sumpt(&p) }
        fn main() -> Int {
            handle body() {
                return r  => r,
                E.tick _  => resume(())
            }
        }
    "#);
}
