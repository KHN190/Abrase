// DebugEvent::Trace register-window exposure: each trace event carries the
// current fn's register window and its handle-mask, so a host sink can show
// "variable values at this op" (and implement breakpoints on top).

use abrase::compiler::Compiler;
use abrase::lexer::Lexer;
use abrase::parser::Parser;
use myriad::{DebugEvent, VirtualMachine, Value};
use std::cell::RefCell;
use std::rc::Rc;

fn compile(src: &str) -> abrase::bytecode::Module {
    let mut p = Parser::new(Lexer::new(src)).with_source(src.to_string());
    let ast = p.parse_program();
    assert!(p.errors.is_empty(), "{}", p.pretty_print_errors());
    let mut c = Compiler::new().with_source(src.to_string());
    c.compile_module(&ast).unwrap_or_else(|_| panic!("\n{}", c.pretty_print_errors()))
}

#[derive(Clone, Default)]
struct Capture {
    // (func, pc, window values, handle_mask)
    traces: Rc<RefCell<Vec<(usize, usize, Vec<u64>, u128)>>>,
}

fn run_traced(src: &str) -> (Value, Capture) {
    let module = compile(src);
    let cap = Capture::default();
    let sink_cap = cap.traces.clone();
    let mut vm = VirtualMachine::new().with_debug_sink(Box::new(move |ev, _names| {
        if let DebugEvent::Trace { func, pc, window, handle_mask, .. } = ev {
            sink_cap.borrow_mut().push((*func, *pc, window.to_vec(), *handle_mask));
        }
    }));
    let v = vm.run_module(&module).expect("run failed");
    (v, cap)
}

const SCALAR_PROG: &str = r#"
fn main() -> Int {
  let x = 5;
  let y = x + 1;
  y
}
"#;

#[test]
fn window_shows_computed_value_at_the_next_op() {
    let (v, cap) = run_traced(SCALAR_PROG);
    assert_eq!(v.as_int(), 6);
    // After the Add executes, some later trace event's window must contain 6.
    let traces = cap.traces.borrow();
    assert!(traces.iter().any(|(_, _, w, _)| w.contains(&6)),
        "no trace window ever contained the computed value 6");
    // And 5 must be visible before that (the let x binding).
    assert!(traces.iter().any(|(_, _, w, _)| w.contains(&5)));
}

const HANDLE_PROG: &str = r#"
fn main() -> Int {
  let a = [10, 20];
  a[0]
}
"#;

#[test]
fn handle_mask_marks_handle_registers_only() {
    let (v, cap) = run_traced(HANDLE_PROG);
    assert_eq!(v.as_int(), 10);
    let traces = cap.traces.borrow();
    // Some event must show at least one handle-bit set (the array binding)…
    assert!(traces.iter().any(|(_, _, _, m)| *m != 0), "no handle bit ever set");
    // …and on every event, registers holding small scalars (e.g. 10) where the
    // mask bit is SET would be a lie; check consistency: a masked reg's value
    // is never one of our scalar literals.
    for (_, _, w, m) in traces.iter() {
        for (i, val) in w.iter().enumerate() {
            if i < 128 && (m & (1u128 << i)) != 0 {
                assert!(*val != 10 && *val != 20,
                    "scalar literal {} flagged as handle in r{}", val, i);
            }
        }
    }
}

#[test]
fn sink_fires_once_per_executed_op() {
    let module = compile(SCALAR_PROG);
    let count = Rc::new(RefCell::new(0u64));
    let c2 = count.clone();
    let mut vm = VirtualMachine::new().with_debug_sink(Box::new(move |ev, _| {
        if matches!(ev, DebugEvent::Trace { .. }) { *c2.borrow_mut() += 1; }
    }));
    vm.run_module(&module).expect("run failed");
    let steps = vm.steps();
    assert_eq!(*count.borrow(), steps, "sink fired {} times for {} steps", count.borrow(), steps);
}

#[test]
fn sink_does_not_change_results() {
    let (v_traced, _) = run_traced(HANDLE_PROG);
    let module = compile(HANDLE_PROG);
    let mut vm = VirtualMachine::new();
    let v_plain = vm.run_module(&module).expect("run failed");
    assert_eq!(v_traced.as_int(), v_plain.as_int());
    assert_eq!(vm.heap_live_count(), 0);
}
