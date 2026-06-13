// Effect transpilation (Model A: explicit-frames interpreter-shape emission).
// Differential is valid here — effect lowering is transpiler-only logic, not
// shared bytecode, so interp-vs-AOT disagreement is a real transpiler bug.
use crate::common::*;
use abrase::{compiler::Compiler, lexer::Lexer, parser::Parser};
use polka_rustc::transpile_module;
use myriad::VirtualMachine;

fn diff_effect(src: &str) {
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
        .unwrap_or_else(|e| panic!("transpile unsupported for effect program: {:?}\n{}", e, src));
    let (t, t_live) = compile_run_full(&tsrc);
    compare(&i, &t);
    if let Outcome::Ok(_) = i {
        assert_eq!(i_live, t_live, "effect heap live-count mismatch: interp={} transpiled={}\n{}", i_live, t_live, src);
    }
}

fn diff_effect_example(name: &str) {
    let path = format!("{}/../../examples/{}.abe", env!("CARGO_MANIFEST_DIR"), name);
    let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {}", path, e));
    diff_effect(&src);
}

// A1 embeds the VM, so even multi-shot resume (generators/backtracking) works.
#[test] fn effect_example_running_sum() { diff_effect_example("running_sum"); }
#[test] fn effect_example_tree_sum()    { diff_effect_example("tree_sum"); }
#[test] fn effect_example_dual_handler(){ diff_effect_example("dual_handler"); }
#[test] fn effect_example_nqueens()     { diff_effect_example("nqueens"); }
#[test] fn effect_example_primes_gen()  { diff_effect_example("primes_gen"); }

#[test]
fn effect_tail_resume_single() {
    diff_effect(r#"
        effect E { op tick() -> Unit }
        fn body() -> <E> Int { E.tick(); 42 }
        fn main() -> Int {
            handle body() {
                return x  => x,
                E.tick _  => resume(())
            }
        }
    "#);
}

#[test]
fn effect_payload_threaded() {
    diff_effect(r#"
        effect Ask { op get() -> Int }
        fn body() -> <Ask> Int { Ask.get() + Ask.get() }
        fn main() -> Int {
            handle body() {
                return x  => x,
                Ask.get _ => resume(7)
            }
        }
    "#);
}

#[test]
fn effect_nontail_fold_single_shot() {
    diff_effect(r#"
        effect Acc { op add(n: Int) -> Unit }
        fn compute() -> <Acc> Unit { Acc.add(10); Acc.add(5); Acc.add(3) }
        fn main() -> Int {
            handle compute() {
                return _   => 0,
                Acc.add n  => n + resume(())
            }
        }
    "#);
}
