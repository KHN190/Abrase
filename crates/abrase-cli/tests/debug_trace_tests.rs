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

// ── trace fn filter ──────────────────────────────────────────────────────────
// VM-side bitset (fn_id-indexed): only matching fns emit Trace events. Filter
// checked BEFORE event construction (skips window/mask building).

const TWO_FNS: &str = r#"
fn helper(x: Int) -> Int { if x <= 0 { 0 } else { x + helper(x - 1) } }
fn main() -> Int {
  let a = helper(3);
  a + 1
}
"#;

fn traced_funcs(src: &str, filter: Option<&[&str]>) -> Vec<usize> {
    let mut p = Parser::new(Lexer::new(src)).with_source(src.to_string());
    let ast = p.parse_program();
    assert!(p.errors.is_empty());
    let mut c = Compiler::new().with_source(src.to_string());
    let module = c.compile_module(&ast).unwrap();
    let names = c.fn_names();
    let seen = Rc::new(RefCell::new(Vec::new()));
    let s2 = seen.clone();
    let mut vm = VirtualMachine::new()
        .with_fn_names(names.clone())
        .with_debug_sink(Box::new(move |ev, _| {
            if let DebugEvent::Trace { func, .. } = ev { s2.borrow_mut().push(*func); }
        }));
    if let Some(fns) = filter {
        let mut bits = vec![false; names.len()];
        for f in fns {
            let i = names.iter().position(|n| n == f).expect("fn not found");
            bits[i] = true;
        }
        vm = vm.with_trace_filter(bits);
    }
    vm.run_module(&module).expect("run failed");
    let out = seen.borrow().clone();
    out
}

#[test]
fn filter_passes_only_matching_fns() {
    let src = TWO_FNS;
    let all = traced_funcs(src, None);
    let only_helper = traced_funcs(src, Some(&["helper"]));
    assert!(!only_helper.is_empty(), "helper events missing");
    let helper_id = only_helper[0];
    assert!(only_helper.iter().all(|f| *f == helper_id), "non-helper events leaked");
    assert!(all.iter().any(|f| *f != helper_id), "baseline should contain main too");
    // Filtered helper events == unfiltered helper events (filter loses nothing).
    assert_eq!(only_helper.len(), all.iter().filter(|f| **f == helper_id).count());
}

#[test]
fn no_filter_behavior_unchanged() {
    let all = traced_funcs(TWO_FNS, None);
    let module = {
        let mut p = Parser::new(Lexer::new(TWO_FNS)).with_source(TWO_FNS.to_string());
        let ast = p.parse_program();
        let mut c = Compiler::new().with_source(TWO_FNS.to_string());
        c.compile_module(&ast).unwrap()
    };
    let mut vm = VirtualMachine::new();
    vm.run_module(&module).expect("run failed");
    assert_eq!(all.len() as u64, vm.steps(), "unfiltered trace must fire per step");
}

#[test]
fn empty_filter_silences_everything() {
    let module = {
        let mut p = Parser::new(Lexer::new(TWO_FNS)).with_source(TWO_FNS.to_string());
        let ast = p.parse_program();
        let mut c = Compiler::new().with_source(TWO_FNS.to_string());
        c.compile_module(&ast).unwrap()
    };
    let count = Rc::new(RefCell::new(0u64));
    let c2 = count.clone();
    let mut vm = VirtualMachine::new()
        .with_debug_sink(Box::new(move |ev, _| {
            if matches!(ev, DebugEvent::Trace { .. }) { *c2.borrow_mut() += 1; }
        }))
        .with_trace_filter(vec![]);
    let v = vm.run_module(&module).expect("run failed");
    assert_eq!(v.as_int(), 7);
    assert_eq!(*count.borrow(), 0, "empty filter must silence all trace events");
}

// ── render_value: handle → structural heap dump ─────────────────────────────
// No runtime types: cells render as [v0, v1, …] (record/array same shape),
// scalars as decimal, HANDLE_NONE as "none", depth cap as "…".

fn run_keep_vm(src: &str) -> (Value, VirtualMachine) {
    let module = compile(src);
    let mut vm = VirtualMachine::new();
    let v = vm.run_module(&module).expect("run failed");
    (v, vm)
}

#[test]
fn renders_scalar_as_decimal() {
    let (v, vm) = run_keep_vm("fn main() -> Int { 42 }");
    assert_eq!(vm.render_value(v.raw(), false, 8), "42");
}

#[test]
fn renders_flat_array() {
    let (v, vm) = run_keep_vm("fn main() -> Array<Int> { [10, 20, 30] }");
    assert_eq!(vm.render_value(v.raw(), true, 8), "[10, 20, 30]");
}

#[test]
fn renders_nested_record_structurally() {
    let src = r#"
type P = { x: Int, y: P2 }
type P2 = { a: Int, b: Int }
fn main() -> P { P { x: 12, y: P2 { a: 5, b: 6 } } }
"#;
    let (v, vm) = run_keep_vm(src);
    assert_eq!(vm.render_value(v.raw(), true, 8), "[12, [5, 6]]");
}

#[test]
fn depth_cap_truncates() {
    let (v, vm) = run_keep_vm("fn main() -> Array<Array<Array<Int>>> { [[[1]]] }");
    assert_eq!(vm.render_value(v.raw(), true, 2), "[[…]]");
}

#[test]
fn handle_none_renders_as_none() {
    let (_, vm) = run_keep_vm("fn main() -> Int { 0 }");
    assert_eq!(vm.render_value(polka::HANDLE_NONE, true, 8), "none");
}

#[test]
fn stale_handle_renders_error_not_panic() {
    let (v, mut vm) = run_keep_vm("fn main() -> Array<Int> { [1] }");
    // Free the cell, then render the stale handle: must not panic.
    let rendered_live = vm.render_value(v.raw(), true, 8);
    assert_eq!(rendered_live, "[1]");
    vm.drop_result_for_test(v.raw());
    let rendered = vm.render_value(v.raw(), true, 8);
    assert!(rendered.contains("stale") || rendered.contains("dead"), "got: {}", rendered);
}
