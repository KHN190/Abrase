use polka::{BytecodeChunk, Chunk, Module, OpCode, Register};
use myriad::{Value, VirtualMachine};

fn r(n: u8) -> Register { Register(n) }

fn raw_constants(consts: Vec<Value>) -> Vec<u64> {
    consts.into_iter().map(|v| v.raw()).collect()
}

fn run(ops: Vec<OpCode>, constants: Vec<Value>) -> Result<Value, String> {
    VirtualMachine::new().run(&Chunk::Bytecode(BytecodeChunk {
        code: ops,
        constants: raw_constants(constants),
        const_mask: Vec::new(),
        reg_count: 64,
        param_count: 0,
        string_constants: Vec::new(),
    }))
}

fn run_module_with_param_counts(functions: Vec<(Vec<OpCode>, Vec<Value>, usize, usize)>) -> Result<Value, String> {
    let n = functions.len();
    let chunks: Vec<Chunk> = functions.into_iter().map(|(code, constants, reg_count, param_count)| {
        Chunk::Bytecode(BytecodeChunk {
            code,
            constants: raw_constants(constants),
            const_mask: Vec::new(),
            reg_count, param_count,
            string_constants: Vec::new(),
        })
    }).collect();
    let module = Module { functions: chunks, entry: n - 1 };
    VirtualMachine::new().run_module(&module)
}

#[test]
fn test_value_int_eq() {
    assert_eq!(Value::from_int(1), Value::from_int(1));
    assert_ne!(Value::from_int(1), Value::from_int(2));
}

#[test]
fn test_value_float_eq() {
    assert_eq!(Value::from_float(3.14), Value::from_float(3.14));
    assert_ne!(Value::from_float(3.14), Value::from_float(2.71));
}

#[test]
fn test_value_char_eq() {
    assert_eq!(Value::from_char('a'), Value::from_char('a'));
    assert_ne!(Value::from_char('a'), Value::from_char('b'));
}

#[test]
fn test_handle_round_trip() {
    let v = Value::from_handle(42, 7);
    assert_eq!(v.as_handle(), (42, 7));
}

#[test]
fn test_handle_none_distinct() {
    assert!(Value::NONE.is_handle_none());
    assert!(!Value::from_handle(0, 0).is_handle_none());
}

#[test]
fn test_value_size_8_bytes() {
    assert_eq!(std::mem::size_of::<Value>(), 8);
}

#[test]
fn oom_alloc_loop_returns_err_not_panic() {
    let mut code: Vec<OpCode> = (0..200).map(|_| OpCode::Alloc(Register(0), 0xFFFF)).collect();
    code.push(OpCode::Ret(Register(0)));
    let chunk = Chunk::Bytecode(BytecodeChunk {
        code,
        constants: Vec::new(),
        const_mask: Vec::new(),
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

#[test]
fn oom_freed_cells_refund_budget() {
    let code = vec![
        OpCode::PushConst(Register(0), 0),
        OpCode::Alloc(Register(1), 0xFFFF),
        OpCode::Drop(Register(1)),
        OpCode::SubImm(Register(0), Register(0), 1),
        OpCode::Jnz(Register(0), -3),
        OpCode::PushConst(Register(2), 1),
        OpCode::Ret(Register(2)),
    ];
    let chunk = Chunk::Bytecode(BytecodeChunk {
        code,
        constants: raw_constants(vec![Value::from_int(200), Value::from_int(0)]),
        const_mask: Vec::new(),
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
    let mut vm = VirtualMachine::new();
    let chunk = Chunk::Bytecode(BytecodeChunk {
        code: vec![
            OpCode::PushConst(r(0), 0),
            OpCode::Handle(r(1), 0),
            OpCode::Resume(r(3), r(0)),
            OpCode::Ret(r(3)),
        ],
        constants: raw_constants(vec![Value::from_int(99)]),
        const_mask: Vec::new(),
        reg_count: 64,
        param_count: 0,
        string_constants: Vec::new(),
    });
    let _ = vm.run(&chunk);
    assert_eq!(vm.heap_live_count(), 0,
        "continuation cell should be reclaimed after single-shot resume");
}

#[test]
fn test_handle_without_dispatch_allocates_no_cell() {
    let mut vm = VirtualMachine::new();
    let install = Chunk::Bytecode(BytecodeChunk {
        code: vec![
            OpCode::Handle(r(1), 0),
            OpCode::Ret(r(0)),
        ],
        constants: raw_constants(vec![Value::from_int(0)]),
        const_mask: Vec::new(),
        reg_count: 64,
        param_count: 0,
        string_constants: Vec::new(),
    });
    let _ = vm.run(&install);
    assert_eq!(vm.heap_live_count(), 0,
        "Handle defers cell allocation to dispatch; no cell at install time");
}

#[test]
fn test_dispatch_lookup_allocates_cont_and_snapshot() {
    let lookup_port = Value::from_int(
        (polka::DISPATCH_ID as i64) << 8 | polka::DISPATCH_PORT_LOOKUP as i64
    );
    let mut vm = VirtualMachine::new();
    let chunk = Chunk::Bytecode(BytecodeChunk {
        code: vec![
            OpCode::PushConst(r(0), 0),
            OpCode::PushConst(r(2), 1),
            OpCode::Handle(r(1), 0),
            OpCode::Deo(r(0), r(2)),
            OpCode::Ret(r(0)),
        ],
        constants: raw_constants(vec![Value::from_int(0), lookup_port]),
        const_mask: Vec::new(),
        reg_count: 64,
        param_count: 0,
        string_constants: Vec::new(),
    });
    let _ = vm.run(&chunk);
    assert_eq!(vm.heap_live_count(), 2,
        "dispatch.lookup must allocate one cont cell + one register snapshot");
}

#[test]
fn test_resume_on_uninitialized_handler_traps() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::Handle(r(1), 0),
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

#[test]
fn test_call_dest_out_of_caller_window_traps() {
    let result = run_module_with_param_counts(vec![
        (vec![OpCode::Ret(r(0))], vec![Value::from_int(1)], 1, 0),
        (vec![OpCode::Call(r(9), 0), OpCode::Ret(r(0))], vec![], 2, 0),
    ]);
    assert!(result.is_err(), "call with out-of-window dest must trap");
    let err = result.unwrap_err();
    assert!(err.contains("out of caller window") || err.contains("register window"),
            "expected window error, got: {}", err);
}

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
    let mut heap = Heap::new();
    let (slot, g_) = heap.alloc(1);
    heap.rc_inc(slot, g_).unwrap();
    let freed1 = heap.rc_dec(slot, g_).unwrap();
    assert!(!freed1, "still aliased; must not reclaim");
    let freed2 = heap.rc_dec(slot, g_).unwrap();
    assert!(freed2, "last alias dropped; must reclaim");
    assert_eq!(heap.live_count(), 0);
}

#[test]
fn test_recursive_drop_reclaims_nested_handles() {
    use myriad::memory::Heap;
    let mut heap = Heap::new();
    let (child, cgen) = heap.alloc(1);
    let (parent, pgen) = heap.alloc(1);
    heap.st(parent, pgen, 0, Value::from_handle(child, cgen).raw(), true).unwrap();
    heap.rc_dec(parent, pgen).unwrap();
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
            constants: raw_constants(vec![Value::from_int(0)]),
            const_mask: Vec::new(),
            reg_count: 3,
            param_count: 0,
            string_constants: Vec::new(),
        })],
        entry: 0,
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
            constants: raw_constants(vec![Value::from_int(0)]),
            const_mask: Vec::new(),
            reg_count: 5,
            param_count: 0,
            string_constants: Vec::new(),
        })],
        entry: 0,
    };
    let result = vm.run_module(&module);
    assert_eq!(result, Ok(Value::from_int(0)));
}
