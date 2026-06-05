// pc→source-line debug info: BytecodeChunk.lines parallel to code (empty =
// stripped). Compiler records the emitting span's line; deleting passes
// (coalesce/copy-prop) must keep the two vectors aligned.

use abrase::bytecode::{Chunk, OpCode};
use abrase::compiler::Compiler;
use abrase::lexer::Lexer;
use abrase::parser::Parser;
use myriad::{DebugEvent, VirtualMachine};
use std::cell::RefCell;
use std::rc::Rc;

fn compile(src: &str) -> abrase::bytecode::Module {
    let mut p = Parser::new(Lexer::new(src)).with_source(src.to_string());
    let ast = p.parse_program();
    assert!(p.errors.is_empty(), "{}", p.pretty_print_errors());
    let mut c = Compiler::new().with_source(src.to_string());
    c.compile_module(&ast).unwrap_or_else(|_| panic!("\n{}", c.pretty_print_errors()))
}

#[test]
fn lines_stay_parallel_to_code_under_all_default_opts() {
    for name in ["nqueens", "mandelbrot", "merge_sort", "coin_change", "primes_gen"] {
        let path = format!("{}/../../examples/{}.abe", env!("CARGO_MANIFEST_DIR"), name);
        let Ok(src) = std::fs::read_to_string(&path) else { continue };
        let module = compile(&src);
        for (i, ch) in module.functions.iter().enumerate() {
            if let Chunk::Bytecode(bc) = ch {
                assert_eq!(bc.lines.len(), bc.code.len(),
                    "{} fn#{}: lines/code misaligned", name, i);
            }
        }
    }
}

const THREE_LINES: &str = r#"fn main() -> Int {
  let x = 5;
  let y = x + 1;
  y
}"#;

#[test]
fn op_maps_to_its_source_line() {
    let module = compile(THREE_LINES);
    let Chunk::Bytecode(bc) = &module.functions[module.entry] else { panic!() };
    // The AddImm from `x + 1` must map to line 3.
    let idx = bc.code.iter().position(|op| matches!(op, OpCode::AddImm(..)))
        .expect("AddImm not found");
    assert_eq!(bc.lines[idx], 3, "AddImm line");
    // The PushConst from `let x = 5` must map to line 2.
    let idx5 = bc.code.iter().position(|op| matches!(op, OpCode::PushConst(..))).unwrap();
    assert_eq!(bc.lines[idx5], 2, "PushConst line");
}

#[test]
fn trace_event_carries_line() {
    let module = compile(THREE_LINES);
    let seen = Rc::new(RefCell::new(Vec::new()));
    let s2 = seen.clone();
    let mut vm = VirtualMachine::new().with_debug_sink(Box::new(move |ev, _| {
        if let DebugEvent::Trace { line, .. } = ev { s2.borrow_mut().push(*line); }
    }));
    vm.run_module(&module).expect("run failed");
    let lines = seen.borrow().clone();
    assert!(lines.contains(&2) && lines.contains(&3), "trace lines: {:?}", lines);
}

#[test]
fn cart_roundtrip_preserves_lines() {
    let module = compile(THREE_LINES);
    let bytes = abrase::bytecode::cartridge::write_pk(&module).expect("encode");
    let back = abrase::bytecode::cartridge::read_pk(&bytes).expect("decode");
    let Chunk::Bytecode(a) = &module.functions[module.entry] else { panic!() };
    let Chunk::Bytecode(b) = &back.functions[back.entry] else { panic!() };
    assert_eq!(a.lines, b.lines);
    assert!(!a.lines.is_empty());
}

#[test]
fn runtime_error_reports_source_line() {
    let src = r#"fn main() -> Int {
  let xs = [1, 2];
  xs[9]
}"#;
    let module = compile(src);
    let mut vm = VirtualMachine::new();
    let err = vm.run_module(&module).unwrap_err();
    assert!(err.contains("@3") || err.contains(":3]"), "no line in error: {}", err);
}

#[test]
fn trace_event_carries_src_file() {
    let mut module = compile(THREE_LINES);
    if let Chunk::Bytecode(b) = &mut module.functions[module.entry] {
        b.src_file = "three.abe".into();
    }
    let seen = Rc::new(RefCell::new(Vec::new()));
    let s2 = seen.clone();
    let mut vm = VirtualMachine::new().with_debug_sink(Box::new(move |ev, _| {
        if let DebugEvent::Trace { file, .. } = ev { s2.borrow_mut().push(file.to_string()); }
    }));
    vm.run_module(&module).expect("run failed");
    assert!(seen.borrow().iter().any(|f| f == "three.abe"), "file missing from trace events");
}
