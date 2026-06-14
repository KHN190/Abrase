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
fn lib_emit_pure_module_same_shape_pk_register_aot_no_main() {
    let src = r#"
        fn fib(n: Int) -> Int { if n < 2 { n } else { fib(n - 1) + fib(n - 2) } }
        fn main() -> Int { fib(10) }
    "#;
    let module = module_of_src(src);
    let lib = polka_rustc::transpile_module_lib(&module).expect("pure lib emit");
    assert!(lib.contains("pub const PK"), "every lib cart exposes PK, effectful or not");
    assert!(lib.contains("pub fn register_aot"), "every lib cart exposes register_aot (empty when nothing bridges)");
    assert!(!lib.contains("fn main"), "lib must not emit a main; host owns the entry");
}

#[test]
fn hybrid_to_s_builtin_bridged_native_matches_interpreter() {
    let src = r#"
        effect E { op tick() -> Unit }
        fn count(n: Int) -> Int {
            if n <= 0 { 0 } else { let s = n.to_s(); 1 + count(n - 1) }
        }
        fn body() -> <E> Int { E.tick(); count(6) }
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
    assert!(tsrc.contains("alloc_string"), "to_s builtin must be bridged native (inline alloc_string)");

    let (t, t_live) = compile_run_full(&tsrc);
    compare(&i, &t);
    if let Outcome::Ok(_) = i {
        assert_eq!(i_live, t_live, "to_s-bridge live-count mismatch: interp={} transpiled={}", i_live, t_live);
    }
}

fn diff_lib(src: &str) {
    let module = module_of_src(src);
    let mut vm = VirtualMachine::new().with_step_cap(1_000_000);
    myriad::Host::default().install_into(&mut vm);
    let i = match vm.run_module(&module) {
        Ok(v) => Outcome::Ok(v.raw()),
        Err(e) => Outcome::Err(e),
    };
    let i_live = vm.heap_live_count();
    let (t, t_live) = compile_run_lib_full(&module);
    compare(&i, &t);
    if let Outcome::Ok(_) = i {
        assert_eq!(i_live, t_live, "lib live-count mismatch: interp={} transpiled={}", i_live, t_live);
    }
}

#[test]
fn lib_pure_module_host_driven_matches_interpreter() {
    diff_lib(r#"
        fn fib(n: Int) -> Int { if n < 2 { n } else { fib(n - 1) + fib(n - 2) } }
        fn main() -> Int { fib(12) }
    "#);
}

#[test]
fn lib_effect_module_host_driven_matches_interpreter() {
    diff_lib(r#"
        effect E { op tick() -> Unit }
        fn fib(n: Int) -> Int { if n < 2 { n } else { fib(n - 1) + fib(n - 2) } }
        fn body() -> <E> Int { E.tick(); fib(12) }
        fn main() -> Int {
            handle body() {
                return r  => r,
                E.tick _  => resume(())
            }
        }
    "#);
}

#[test]
fn hybrid_all_inline_math_builtins_match_interpreter() {
    let src = r#"
        effect E { op tick() -> Unit }
        fn calc(n: Int, acc: Float) -> Float {
            if n <= 0 { acc } else {
                let x = n.to_f();
                let y = sqrt(x) + sin(x) + cos(x) + flr(x) + ceil(x);
                let z = x.max(y).min(100.0).abs();
                let m = n.max(3).min(99).abs();
                let w = z + m.to_f() + (y.to_i()).to_f();
                calc(n - 1, acc + w)
            }
        }
        fn body() -> <E> Float { E.tick(); calc(8, 0.0) }
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
    for needle in ["fmath::sqrt", "fmath::sin", "fmath::cos", "fmath::floor", "fmath::ceil",
                   "fmath::fmax", "fmath::fmin", "fmath::abs", ".max(", ".min(", "wrapping_abs"] {
        assert!(tsrc.contains(needle), "inline math builtin missing: {}", needle);
    }
    let (t, t_live) = compile_run_full(&tsrc);
    compare(&i, &t);
    if let Outcome::Ok(_) = i {
        assert_eq!(i_live, t_live, "inline-math live-count mismatch: interp={} transpiled={}", i_live, t_live);
    }
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
