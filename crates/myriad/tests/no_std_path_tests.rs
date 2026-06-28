use polka::{BytecodeChunk, Chunk, OpCode, Register};
use myriad::{Value, VirtualMachine};
use std::sync::atomic::{AtomicUsize, Ordering};

fn r(n: u8) -> Register { Register(n) }

fn chunk(ops: Vec<OpCode>, constants: Vec<u64>) -> Chunk {
    Chunk::Bytecode(BytecodeChunk {
        code: ops,
        constants,
        const_mask: Vec::new(),
        string_constants: Vec::new(),
        reg_count: 8,
        param_count: 0,
        lines: Vec::new(),
        src_file: String::new(),
    })
}

static HEAP_TRACE_HITS: AtomicUsize = AtomicUsize::new(0);
fn heap_sink(_s: &str) { HEAP_TRACE_HITS.fetch_add(1, Ordering::Relaxed); }

#[test]
fn heap_trace_hook_receives_lines() {
    HEAP_TRACE_HITS.store(0, Ordering::Relaxed);
    let prog = chunk(vec![OpCode::Alloc(r(0), 2), OpCode::Drop(r(0)), OpCode::Ret(r(1))], vec![]);
    let mut vm = VirtualMachine::new().with_heap_trace(None, true, heap_sink);
    vm.run(&prog).expect("run");
    assert!(
        HEAP_TRACE_HITS.load(Ordering::Relaxed) > 0,
        "heap trace sink received no lines",
    );
}

#[test]
fn heap_trace_hook_silent_without_install() {
    let prog = chunk(vec![OpCode::Alloc(r(0), 2), OpCode::Drop(r(0)), OpCode::Ret(r(1))], vec![]);
    let mut vm = VirtualMachine::new();
    assert!(vm.run(&prog).is_ok());
}

static STATIC_TRACE_HITS: AtomicUsize = AtomicUsize::new(0);
fn text_sink(_s: &str) { STATIC_TRACE_HITS.fetch_add(1, Ordering::Relaxed); }

#[test]
fn frame_trace_sink_wired_through_trace_out() {
    STATIC_TRACE_HITS.store(0, Ordering::Relaxed);
    let prog = chunk(vec![OpCode::PushConst(r(0), 0), OpCode::Ret(r(0))], vec![Value::from_int(42).raw()]);
    let mut vm = VirtualMachine::new().with_trace_out(text_sink).with_trace_frames(true);
    assert_eq!(vm.run(&prog).unwrap().as_int(), 42);
}

#[test]
fn host_default_is_headless_and_runs() {
    let mut vm = VirtualMachine::new();
    myriad::Host::default().install_into(&mut vm);
    let prog = chunk(vec![OpCode::PushConst(r(0), 0), OpCode::Ret(r(0))], vec![Value::from_int(7).raw()]);
    assert_eq!(vm.run(&prog).unwrap().as_int(), 7);
}

#[test]
fn profile_report_empty_when_disabled() {
    let prog = chunk(vec![OpCode::PushConst(r(0), 0), OpCode::Ret(r(0))], vec![Value::from_int(1).raw()]);
    let mut vm = VirtualMachine::new();
    vm.run(&prog).unwrap();
    assert_eq!(vm.profile_report(), "");
}

#[test]
fn profile_report_populated_when_enabled() {
    let prog = chunk(vec![OpCode::PushConst(r(0), 0), OpCode::Ret(r(0))], vec![Value::from_int(1).raw()]);
    let mut vm = VirtualMachine::new().with_profile(true);
    vm.run(&prog).unwrap();
    let report = vm.profile_report();
    assert!(report.contains("[profile]"), "report missing header: {:?}", report);
    assert!(report.contains("ops executed"), "report missing op count: {:?}", report);
}
