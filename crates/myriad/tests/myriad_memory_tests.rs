use polka::{BytecodeChunk, Chunk, Module, OpCode, Register};
use myriad::{Value, VirtualMachine};

fn r(n: u8) -> Register { Register(n) }

fn run(ops: Vec<OpCode>, constants: Vec<Value>) -> Result<Value, String> {
    VirtualMachine::new().run(&Chunk::Bytecode(BytecodeChunk {
        code: ops, constants, reg_count: 256, param_count: 0, string_constants: Vec::new(),
    }))
}

fn run_module_with_param_counts(functions: Vec<(Vec<OpCode>, Vec<Value>, usize, usize)>) -> Result<Value, String> {
    let n = functions.len();
    let chunks: Vec<Chunk> = functions.into_iter().map(|(code, constants, reg_count, param_count)| {
        Chunk::Bytecode(BytecodeChunk { code, constants, reg_count, param_count, string_constants: Vec::new() })
    }).collect();
    let module = Module { functions: chunks, entry: n - 1, device_mask: [0; 32] };
    VirtualMachine::new().run_module(&module)
}

#[test]
fn test_value_int_eq() {
    assert_eq!(Value::from_int(1), Value::from_int(1));
    assert_ne!(Value::from_int(1), Value::from_int(2));
}

#[test]
fn test_value_bool_variants() {
    assert_eq!(Value::from_bool(true), Value::from_bool(true));
    assert_ne!(Value::from_bool(true), Value::from_bool(false));
}

#[test]
fn test_value_float_eq() {
    assert_eq!(Value::from_float(3.14), Value::from_float(3.14));
    assert_ne!(Value::from_float(3.14), Value::from_float(2.71));
}

#[test]
fn test_value_unit_eq() {
    assert_eq!(Value::UNIT, Value::UNIT);
}

#[test]
fn test_value_char_eq() {
    assert_eq!(Value::from_char('a'), Value::from_char('a'));
    assert_ne!(Value::from_char('a'), Value::from_char('b'));
}

#[test]
fn test_value_cross_type_inequality() {
    assert_ne!(Value::from_int(1), Value::from_bool(true));
    assert_ne!(Value::from_int(1), Value::from_float(1.0));
    assert_ne!(Value::from_bool(false), Value::UNIT);
}

#[test]
fn test_handle_round_trip() {
    let v = Value::from_handle(42, 7);
    assert_eq!(v.as_handle(), Some((42, 7)));
    assert!(v.is_handle());
}

#[test]
fn test_none_unit_distinct() {
    assert_ne!(Value::NONE, Value::UNIT);
    assert!(Value::NONE.is_none());
    assert!(Value::UNIT.is_unit());
}

#[test]
fn test_value_size_8_bytes() {
    assert_eq!(std::mem::size_of::<Value>(), 8);
}

// Each Alloc(r, 0xFFFF) charges 65535 * 8 = 524_280 bytes. With MAX_RAM at
// 64 MiB the cap is crossed after ~128 unfreed allocs — 200 unrolled allocs
// gives comfortable headroom regardless of small constant tweaks. The VM
// must surface a graceful "out of memory" error rather than panicking.
#[test]
fn oom_alloc_loop_returns_err_not_panic() {
    let mut code: Vec<OpCode> = (0..200).map(|_| OpCode::Alloc(Register(0), 0xFFFF)).collect();
    code.push(OpCode::Ret(Register(0)));
    let chunk = Chunk::Bytecode(BytecodeChunk {
        code,
        constants: Vec::new(),
        reg_count: 4,
        param_count: 0,
        string_constants: Vec::new(),
    });
    let result = VirtualMachine::new().run(&chunk);
    let err = result.expect_err("excessive alloc must surface an error, not succeed");
    assert!(
        err.contains("out of memory"),
        "OOM error should mention 'out of memory'; got: {}",
        err
    );
}

// rc_dec to zero must refund the cell's bytes so subsequent allocs see room
// again. Without the refund, repeated alloc/drop would slowly leak budget.
#[test]
fn oom_freed_cells_refund_budget() {
    // Each iteration: alloc 0xFFFF values, then drop. 200 rounds far exceeds
    // the 20 MiB cap in aggregate — but per round, only one cell is live.
    let code = vec![
        OpCode::PushConst(Register(0), 0),               // i = 200
        // top:
        OpCode::Alloc(Register(1), 0xFFFF),              // big = alloc
        OpCode::Drop(Register(1)),                       // free big
        OpCode::SubImm(Register(0), Register(0), 1),     // i -= 1
        OpCode::Jnz(Register(0), -3),                    // loop while i != 0
        OpCode::PushConst(Register(2), 1),
        OpCode::Ret(Register(2)),
    ];
    let chunk = Chunk::Bytecode(BytecodeChunk {
        code,
        constants: vec![Value::from_int(200), Value::from_int(0)],
        reg_count: 8,
        param_count: 0,
        string_constants: Vec::new(),
    });
    let result = VirtualMachine::new().run(&chunk);
    assert_eq!(result, Ok(Value::from_int(0)),
        "alloc/drop loop must succeed when refund works; got: {:?}", result);
}

#[test]
fn test_handle_allocates_cell_and_resume_frees_it() {
    // Single-shot Resume must reclaim its cell; heap net-zero at exit.
    let mut vm = VirtualMachine::new();
    let chunk = Chunk::Bytecode(BytecodeChunk {
        code: vec![
            OpCode::PushConst(r(0), 0),
            OpCode::Handle(r(3), r(1), 0),
            OpCode::Resume(r(3), r(0)),
            OpCode::Ret(r(3)),
        ],
        constants: vec![Value::from_int(99)],
        reg_count: 256,
        param_count: 0, string_constants: Vec::new(),
    });
    let _ = vm.run(&chunk);
    assert_eq!(vm.heap_live_count(), 0,
        "continuation cell should be reclaimed after single-shot resume");
}

#[test]
fn test_handle_without_dispatch_allocates_no_cell() {
    // Handle defers cell allocation to dispatch.lookup; install alone leaves no live cells.
    let mut vm = VirtualMachine::new();
    let install = Chunk::Bytecode(BytecodeChunk {
        code: vec![
            OpCode::Handle(r(7), r(1), 0),
            OpCode::Ret(r(0)),
        ],
        constants: vec![Value::from_int(0)],
        reg_count: 256,
        param_count: 0, string_constants: Vec::new(),
    });
    let _ = vm.run(&install);
    assert_eq!(vm.heap_live_count(), 0,
        "Handle defers cell allocation to dispatch; no cell at install time");
}

#[test]
fn test_dispatch_lookup_allocates_one_cell() {
    // dispatch.lookup (not Handle) is the point at which exactly one continuation cell is allocated.
    let lookup_port = Value::from_int(
        (polka::DISPATCH_ID as i64) << 8 | polka::DISPATCH_PORT_LOOKUP as i64
    );
    let mut vm = VirtualMachine::new();
    let chunk = Chunk::Bytecode(BytecodeChunk {
        code: vec![
            OpCode::PushConst(r(0), 0),        // r(0) = 0  (lookup key)
            OpCode::PushConst(r(2), 1),        // r(2) = dispatch.lookup port
            OpCode::Handle(r(3), r(1), 0),     // push handler frame (r(1) uninit → no table)
            OpCode::Deo(r(0), r(2)),           // dispatch.lookup → alloc 1 cell
            OpCode::Ret(r(0)),
        ],
        constants: vec![Value::from_int(0), lookup_port],
        reg_count: 256,
        param_count: 0, string_constants: Vec::new(),
    });
    let _ = vm.run(&chunk);
    assert_eq!(vm.heap_live_count(), 1,
        "dispatch.lookup must allocate exactly one continuation cell");
}

#[test]
fn test_resume_on_uninitialized_handler_traps() {
    // Resume before any dispatch.lookup → cell_slot is still the dummy 0 → trap.
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::Handle(r(3), r(1), 0),
            OpCode::Resume(r(3), r(0)),
            OpCode::Ret(r(3)),
        ],
        vec![Value::from_int(99)],
    );
    assert!(result.is_err(), "resume on uninitialized handler must trap, got {:?}", result);
    let err = result.unwrap_err();
    assert!(err.contains("invalid slot") || err.contains("slot 0"),
            "expected invalid-slot error, got: {}", err);
}

//  Call whose `dest` is outside the caller's reg_count must trap.
#[test]
fn test_call_dest_out_of_caller_window_traps() {
    let result = run_module_with_param_counts(vec![
        (vec![OpCode::Ret(r(0))], vec![Value::from_int(1)], 1, 0),
        // Caller has reg_count=2 but Call writes to r9 — out of window.
        (vec![OpCode::Call(r(9), 0), OpCode::Ret(r(0))], vec![], 2, 0),
    ]);
    assert!(result.is_err(), "call with out-of-window dest must trap");
    let err = result.unwrap_err();
    assert!(err.contains("out of caller window") || err.contains("register window"),
            "expected window error, got: {}", err);
}

// Drive Handle/Resume opcodes directly; codegen lowers `handle` to arm-fn Calls.
#[test]
fn test_resume_without_handler_traps() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::Resume(r(1), r(0)),
            OpCode::Ret(r(0)),
        ],
        vec![Value::from_int(7)],
    );
    assert!(result.is_err(), "resume without handler must trap");
}

#[test]
fn test_rc_inc_keeps_cell_alive_until_balanced() {
    use myriad::memory::Heap;
    use myriad::BoxPool;
    let mut heap = Heap::new();
    let mut pool = BoxPool::new();
    let (slot, g_) = heap.alloc(1);
    heap.rc_inc(slot, g_).unwrap();
    let freed1 = heap.rc_dec(slot, g_, &mut pool).unwrap();
    assert!(!freed1, "still aliased; must not reclaim");
    let freed2 = heap.rc_dec(slot, g_, &mut pool).unwrap();
    assert!(freed2, "last alias dropped; must reclaim");
    assert_eq!(heap.live_count(), 0);
}

#[test]
fn test_recursive_drop_reclaims_nested_handles() {
    use myriad::memory::Heap;
    use myriad::BoxPool;
    let mut heap = Heap::new();
    let mut pool = BoxPool::new();
    let (child, cgen) = heap.alloc(1);
    let (parent, pgen) = heap.alloc(1);
    heap.st(parent, pgen, 0, Value::from_handle(child, cgen)).unwrap();
    heap.rc_dec(parent, pgen, &mut pool).unwrap();
    assert_eq!(heap.live_count(), 0, "child must be reclaimed transitively");
}

#[test]
fn test_drop_reclaims_heap_via_rc_dec() {
    let mut vm = VirtualMachine::new();
    let module = Module {
        functions: vec![Chunk::Bytecode(BytecodeChunk {
            code: vec![
                OpCode::Alloc(r(0), 4),
                OpCode::Alloc(r(1), 4),
                OpCode::Drop(r(0)),
                OpCode::Drop(r(1)),
                OpCode::PushConst(r(2), 0),
                OpCode::Ret(r(2)),
            ],
            constants: vec![Value::from_int(0)],
            reg_count: 3,
            param_count: 0, string_constants: Vec::new(),
        })],
        entry: 0,
        device_mask: [0; 32],
    };
    let result = vm.run_module(&module);
    assert_eq!(result, Ok(Value::from_int(0)));
    assert_eq!(vm.heap_live_count(), 0, "all heap cells must be reclaimed");
}

#[test]
fn test_handle_after_free_is_rejected_via_generation() {
    let mut vm = VirtualMachine::new();
    let module = Module {
        functions: vec![Chunk::Bytecode(BytecodeChunk {
            code: vec![
                OpCode::Alloc(r(0), 1),
                OpCode::Copy(r(1), r(0)),
                OpCode::Drop(r(0)),
                OpCode::Drop(r(1)),
                OpCode::Alloc(r(2), 1),
                OpCode::Copy(r(3), r(2)),
                OpCode::Drop(r(2)),
                OpCode::PushConst(r(4), 0),
                OpCode::Ret(r(4)),
            ],
            constants: vec![Value::from_int(0)],
            reg_count: 5,
            param_count: 0, string_constants: Vec::new(),
        })],
        entry: 0,
        device_mask: [0; 32],
    };
    let result = vm.run_module(&module);
    assert_eq!(result, Ok(Value::from_int(0)));
}