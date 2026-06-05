// Tail-resumptive handler specialization: an arm whose only completion is a
// tail `resume(expr)` compiles to a plain call (no cont cell, no register
// snapshot). Semantics must be identical; the win is hidden work + steps.

use abrase::compiler::Compiler;
use abrase::lexer::Lexer;
use abrase::parser::Parser;
use myriad::VirtualMachine;

fn run(src: &str, tail: bool) -> (i64, usize, u64) {
    let mut p = Parser::new(Lexer::new(src)).with_source(src.to_string());
    let ast = p.parse_program();
    assert!(p.errors.is_empty(), "{}", p.pretty_print_errors());
    let mut c = Compiler::new().with_source(src.to_string()).with_tail_resume(tail);
    let module = c.compile_module(&ast)
        .unwrap_or_else(|_| panic!("\n{}", c.pretty_print_errors()));
    let mut vm = VirtualMachine::new();
    let v = vm.run_module(&module).expect("run failed");
    (v.as_int(), vm.heap_live_count(), vm.steps())
}

fn assert_equiv(src: &str) {
    let (v0, h0, _) = run(src, false);
    let (v1, h1, _) = run(src, true);
    assert_eq!((v0, h0), (v1, h1), "behavior diverged");
    assert_eq!(h0, 0, "leak in baseline");
}

const CONST_RESUME: &str = r#"
effect tick { op step() -> Int }
fn body(n: Int) -> <tick> Int {
  let mut acc = 0;
  let mut i = 0;
  while i < n { acc = acc + tick.step(); i = i + 1 };
  acc
}
fn main() -> Int {
  handle body(100) { return v => v, tick.step => resume(1) }
}
"#;

#[test]
fn const_resume_is_equivalent() { assert_equiv(CONST_RESUME); }

// The win is hidden work (cont-cell alloc + register snapshot), which does not
// change the op count — assert the dispatch-table entry carries the TAIL flag.
#[test]
fn const_resume_marks_dispatch_entry_tail() {
    fn entry_consts(src: &str, tail: bool) -> Vec<u64> {
        let mut p = Parser::new(Lexer::new(src)).with_source(src.to_string());
        let ast = p.parse_program();
        let mut c = Compiler::new().with_source(src.to_string()).with_tail_resume(tail);
        let module = c.compile_module(&ast).unwrap();
        match &module.functions[module.entry] {
            abrase::bytecode::Chunk::Bytecode(bc) => bc.constants.clone(),
            _ => panic!("entry not bytecode"),
        }
    }
    let flag = abrase::bytecode::DISPATCH_TAIL_FLAG;
    let on = entry_consts(CONST_RESUME, true);
    let off = entry_consts(CONST_RESUME, false);
    assert!(on.iter().any(|c| c & flag != 0), "no TAIL-flagged table entry with flag on");
    assert!(!off.iter().any(|c| c & flag != 0), "TAIL flag leaked with flag off");
}

const PAYLOAD_COMPUTE: &str = r#"
effect math { op double(x: Int) -> Int }
fn body(n: Int) -> <math> Int {
  let mut acc = 0;
  let mut i = 0;
  while i < n { acc = acc + math.double(i); i = i + 1 };
  acc
}
fn main() -> Int {
  handle body(50) { return v => v, math.double x => resume(x * 2) }
}
"#;

#[test]
fn payload_computation_is_equivalent() { assert_equiv(PAYLOAD_COMPUTE); }

const CAPTURED_ENV: &str = r#"
effect cfg { op base() -> Int }
fn body() -> <cfg> Int { cfg.base() + cfg.base() }
fn main() -> Int {
  let k = 21;
  handle body() { return v => v, cfg.base => resume(k) }
}
"#;

#[test]
fn captured_env_is_equivalent() { assert_equiv(CAPTURED_ENV); }

const MIXED_ARMS: &str = r#"
effect pipe {
  op fast(x: Int) -> Int
  op slow() -> Int
}
fn body() -> <pipe> Int {
  let a = pipe.fast(3);
  let b = pipe.slow();
  pipe.fast(a + b)
}
fn main() -> Int {
  handle body() {
    return v => v,
    pipe.fast x => resume(x + 1),
    pipe.slow => { let t = 5; resume(t * 2) }
  }
}
"#;

#[test]
fn mixed_arms_are_equivalent() { assert_equiv(MIXED_ARMS); }

const THROW_ARM_STAYS_SLOW: &str = r#"
effect guard { op check(x: Int) -> Int }
fn body() -> <guard> Int { guard.check(5) + guard.check(-1) }
fn run() -> <exn<String>> Int {
  handle body() {
    return v => v,
    guard.check x => if x < 0 { throw "neg" } else { resume(x) }
  }
}
fn main() -> Int {
  handle run() { return v => v, exn _ => 0 }
}
"#;

#[test]
fn conditional_throw_arm_is_equivalent() { assert_equiv(THROW_ARM_STAYS_SLOW); }

const NESTED_HANDLE: &str = r#"
effect a { op get() -> Int }
effect b { op get() -> Int }
fn inner() -> <a, b> Int { a.get() + b.get() }
fn outer() -> <a> Int {
  handle inner() { return v => v, b.get => resume(10) }
}
fn main() -> Int {
  handle outer() { return v => v, a.get => resume(1) }
}
"#;

#[test]
fn nested_handles_are_equivalent() { assert_equiv(NESTED_HANDLE); }

const HANDLE_IN_LOOP: &str = r#"
effect t { op n() -> Int }
fn body() -> <t> Int { t.n() + t.n() }
fn main() -> Int {
  let mut acc = 0;
  let mut i = 0;
  while i < 10 {
    acc = acc + handle body() { return v => v, t.n => resume(i) };
    i = i + 1
  };
  acc
}
"#;

#[test]
fn handle_in_loop_is_equivalent() { assert_equiv(HANDLE_IN_LOOP); }

const STATE_VIA_HANDLER: &str = r#"
effect st { op bump(x: Int) -> Int }
fn body(n: Int) -> <st> Int {
  let mut i = 0;
  let mut last = 0;
  while i < n { last = st.bump(last); i = i + 1 };
  last
}
fn main() -> Int {
  handle body(8) { return v => v, st.bump x => resume(x + 3) }
}
"#;

#[test]
fn threaded_state_is_equivalent() { assert_equiv(STATE_VIA_HANDLER); }
