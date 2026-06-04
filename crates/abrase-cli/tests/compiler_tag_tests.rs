// Oracle for the "handle tag is truth" invariant (wiki 05 / abrase-internals).
//
// With drop-elision ON, a block-local whose compile-time tag (reg_holds_handle)
// is false gets NO scope-exit Drop. That is sound ONLY if the tag never lies:
// every register that holds a handle at runtime must be tagged true. Where the
// tag under-reports (false negative), eliding its Drop leaks the handle.
//
// These run effect/handler and scalar-heavy programs WITH elision and assert
// both correctness and heap_live_count == 0. They are RED until reg_holds_handle
// is made reliable, and each failure pinpoints an under-tagged codegen site.

use abrase::compiler::Compiler;
use abrase::lexer::Lexer;
use abrase::parser::Parser;
use myriad::{Value, VirtualMachine};

fn run_elided(src: &str) -> (Value, usize) {
    let mut p = Parser::new(Lexer::new(src)).with_source(src.into());
    let ast = p.parse_program();
    assert!(p.errors.is_empty(), "parse: {}", p.pretty_print_errors());
    let mut c = Compiler::new().with_source(src.into()).with_drop_elision(true);
    let module = c.compile_module(&ast)
        .unwrap_or_else(|_| panic!("compile: {}", c.pretty_print_errors()));
    let mut vm = VirtualMachine::new();
    let v = vm.run_module(&module).unwrap_or_else(|e| panic!("vm: {}", e));
    (v, vm.heap_live_count())
}

fn check(src: &str, want: i64) {
    let (v, live) = run_elided(src);
    assert_eq!(v, Value::from_int(want), "value");
    assert_eq!(live, 0, "leak under drop-elision (tag under-reported a handle)");
}

#[test]
fn elision_effect_resume_constant() {
    check("effect E { op tick() -> Int }\n\
           fn body() -> <E> Int { let x = E.tick(); x + 1 }\n\
           fn main() -> Int { handle body() { return v => v, E.tick _ => resume(41) } }", 42);
}

#[test]
fn elision_effect_arm_captures_outer_let() {
    check("effect E { op ask() -> Int }\n\
           fn body() -> <E> Int { E.ask() }\n\
           fn main() -> Int { let base = 100; handle body() { return v => v, E.ask _ => resume(base + 5) } }", 105);
}

#[test]
fn elision_return_arm_transforms() {
    check("effect E { op go() -> Int }\n\
           fn body() -> <E> Int { E.go() }\n\
           fn main() -> Int { handle body() { return v => v * 10, E.go _ => resume(3) } }", 30);
}

#[test]
fn elision_handle_pure_body() {
    check("effect E { op go() -> Int }\n\
           fn pure() -> Int { 7 }\n\
           fn main() -> Int { handle pure() { return v => v, E.go _ => resume(0) } }", 7);
}

#[test]
fn elision_array_and_record_heavy() {
    check("type R = { n: Int }\n\
           fn main() -> Int { let xs = [R { n: 3 }, R { n: 4 }]; let mut s = 0; let mut i = 0; \
             while i < 2 { let r = xs[i]; s = s + r.n; i = i + 1 }; s }", 7);
}

#[test]
fn elision_scalar_loop_correct() {
    check("fn main() -> Int { let xs = [10, 20, 30]; let mut s = 0; let mut i = 0; \
           while i < 3 { let e = xs[i]; s = s + e; i = i + 1 }; s }", 60);
}
