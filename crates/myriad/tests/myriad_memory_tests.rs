use polka::{BytecodeChunk, Chunk, Module, OpCode, Register};
use myriad::{Value, VirtualMachine};

fn r(n: u8) -> Register { Register(n) }

fn raw_constants(consts: Vec<Value>) -> Vec<u64> {
    consts.into_iter().map(|v| v.raw()).collect()
}

fn run(ops: Vec<OpCode>, constants: Vec<Value>) -> Result<Value, String> {
    VirtualMachine::new().run(&Chunk::Bytecode(BytecodeChunk {
        src_file: String::new(),
        lines: vec![],
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
        src_file: String::new(),
        lines: vec![],
            code,
            constants: raw_constants(constants),
            const_mask: Vec::new(),
            reg_count, param_count,
            string_constants: Vec::new(),
        })
    }).collect();
    let module = Module { functions: chunks, entry: n - 1, flags: 0, exports: vec![] };
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
        src_file: String::new(),
        lines: vec![],
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
        src_file: String::new(),
        lines: vec![],
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
        src_file: String::new(),
        lines: vec![],
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
        src_file: String::new(),
        lines: vec![],
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
        src_file: String::new(),
        lines: vec![],
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
fn test_fresh_alloc_starts_at_generation_zero() {
    use myriad::memory::Heap;
    let mut heap = Heap::new();
    let (slot, g_) = heap.alloc(2);
    assert_eq!(g_, 0);
    assert!(heap.is_live(slot, g_));
    assert_eq!(heap.live_count(), 1);
}

#[test]
fn test_freed_slot_is_reused_with_bumped_generation() {
    use myriad::memory::Heap;
    let mut heap = Heap::new();
    let (slot, g0) = heap.alloc(1);
    assert!(heap.rc_dec(slot, g0).unwrap());
    let (slot2, g1) = heap.alloc(1);
    assert_eq!(slot2, slot, "freed slot must be reused");
    assert_eq!(g1, g0 + 1, "reused slot must bump generation");
    assert!(!heap.is_live(slot, g0), "stale handle must not be live");
    assert!(heap.is_live(slot2, g1));
}

#[test]
fn test_force_free_is_idempotent() {
    use myriad::memory::Heap;
    let mut heap = Heap::new();
    let (slot, g_) = heap.alloc(1);
    heap.force_free(slot, g_).unwrap();
    assert!(!heap.is_live(slot, g_));
    heap.force_free(slot, g_).unwrap();
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
        src_file: String::new(),
        lines: vec![],
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
        flags: 0,

        exports: vec![],
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
        src_file: String::new(),
        lines: vec![],
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
        flags: 0,

        exports: vec![],
    };
    let result = vm.run_module(&module);
    assert_eq!(result, Ok(Value::from_int(0)));
}

// Handle slot = byte offset; arena grows to the 1<<28 cap so slot needs >24 bits
// (only gen is 24). Sweep across the 2^24 boundary so any codec narrowing fails
// before runtime (else a big Array crossing 16MB aliases a neighbour = UAF).
#[test]
fn handle_codec_round_trips_across_boundaries() {
    for &slot in &[0u32, 1, 0xFFFF, 0xFF_FFFF, 0x100_0000, 0x100_0001, 0xFFF_FFFF, 0x1000_0000] {
        for &g in &[0u32, 1, 0x7F_FFFF, 0xFF_FFFF] {
            let (s2, g2) = myriad::memory::handle_parts(myriad::memory::make_handle(slot, g));
            assert_eq!((s2, g2), (slot, g), "codec narrowed slot {:#x} gen {:#x}", slot, g);
        }
    }
}

#[test]
fn alloc_past_16mb_does_not_alias() {
    let mut h = myriad::Heap::with_capacity(1 << 16);
    let (sa, ga) = h.try_alloc(1 << 21).expect("big alloc");
    let (sb, gb) = h.try_alloc(2).expect("alloc past 16MB");
    h.st(sa, ga, 0, 0xAAAA, false).expect("st a");
    h.st(sb, gb, 0, 0xBBBB, false).expect("st b");
    assert_eq!(h.ld(sa, ga, 0).expect("ld a").0, 0xAAAA, "block A clobbered by aliased B");
    assert_eq!(h.ld(sb, gb, 0).expect("ld b").0, 0xBBBB, "block B handle truncated");
}

// Allocate blocks whose cumulative size carries the frontier past byte offset
// 2^24, filling every element, then read back — any slot truncation or data
// aliasing across the 16MB boundary corrupts a neighbour.
#[test]
fn arena_size_sweep_past_16mb_keeps_data_intact() {
    use myriad::memory::Heap;
    let mut h = Heap::with_capacity(1 << 16);
    let sizes = [4usize, 1 << 20, 7, 1 << 20, 1 << 20, 9]; // ~24MB total, blocks cross 2^24
    let mut blocks = Vec::new();
    for (tag, &sz) in sizes.iter().enumerate() {
        let (s, g) = h.try_alloc(sz).expect("alloc");
        for j in 0..sz {
            h.st(s, g, j, ((tag as u64) << 40) | j as u64, false).expect("st");
        }
        blocks.push((s, g, sz, tag));
    }
    for (s, g, sz, tag) in blocks {
        for &j in &[0usize, sz / 2, sz - 1] {
            assert_eq!(
                h.ld(s, g, j).expect("ld").0,
                ((tag as u64) << 40) | j as u64,
                "block {} elem {} corrupted (slot {} crossed 16MB?)", tag, j, s
            );
        }
    }
}

// Random alloc/st/ld/rc sequence checked against a shadow model: every ld must
// equal the last st to that cell (catches slot aliasing); rc_dec to zero must
// reclaim without disturbing other live cells.
#[test]
fn heap_api_model_fuzz() {
    use myriad::memory::Heap;
    let mut h = Heap::with_capacity(1 << 16);
    let mut live: Vec<(u32, u32, Vec<u64>)> = Vec::new();
    let mut s = 0x9E3779B97F4A7C15u64;
    let rng = |s: &mut u64| {
        *s ^= *s << 13;
        *s ^= *s >> 7;
        *s ^= *s << 17;
        *s
    };
    for _ in 0..8000 {
        let op = rng(&mut s) % 100;
        if live.is_empty() || op < 35 {
            let sz = (rng(&mut s) % 8 + 1) as usize;
            let (slot, g) = h.try_alloc(sz).expect("alloc");
            live.push((slot, g, vec![u64::MAX; sz]));
        } else if op < 65 {
            let i = (rng(&mut s) as usize) % live.len();
            let (slot, g, sz) = (live[i].0, live[i].1, live[i].2.len());
            let off = (rng(&mut s) as usize) % sz;
            let val = rng(&mut s);
            h.st(slot, g, off, val, false).expect("st");
            live[i].2[off] = val;
        } else if op < 90 {
            let i = (rng(&mut s) as usize) % live.len();
            let (slot, g) = (live[i].0, live[i].1);
            for (off, &want) in live[i].2.iter().enumerate() {
                assert_eq!(h.ld(slot, g, off).expect("ld").0, want,
                    "aliasing: slot {} off {} drifted", slot, off);
            }
        } else {
            let i = (rng(&mut s) as usize) % live.len();
            let (slot, g) = (live[i].0, live[i].1);
            assert!(h.rc_dec(slot, g).expect("rc_dec"), "single owner must reclaim");
            live.swap_remove(i);
        }
    }
    assert_eq!(h.live_count(), live.len(), "live_count must match model");
}

// Big array (forces arena grow past 16MB) stored as a handle in a record cell,
// then repeatedly loaded + rc-cycled like cross-frame access — its refcount must
// stay balanced (regression for large-array-in-record premature free).
#[test]
fn big_array_handle_in_record_rc_survives_grow() {
    use myriad::memory::{make_handle, Heap};
    let mut h = Heap::with_capacity(1 << 16);
    let (arr, ag) = h.try_alloc(1 << 21).expect("2M-elem array"); // 16MB → grows arena
    let (rec, rg) = h.try_alloc(2).expect("record");
    // record owns the array: store handle + the owning ref is the array's rc=1.
    h.st(rec, rg, 0, make_handle(arr, ag), true).expect("st field");
    // simulate 10 frames of `s.sheet[0]`: owning Ld (rc_inc) ... use ... Drop (rc_dec).
    for f in 0..10 {
        let (raw, is_h) = h.ld(rec, rg, 0).expect("ld field");
        assert!(is_h, "frame {}: field lost handle tag", f);
        let (s, g) = myriad::memory::handle_parts(raw);
        assert_eq!((s, g), (arr, ag), "frame {}: field handle corrupted", f);
        h.rc_inc(s, g).expect("rc_inc");
        assert!(h.is_live(arr, ag), "frame {}: array dead after inc", f);
        h.rc_dec(s, g).expect("rc_dec");
        assert!(h.is_live(arr, ag), "frame {}: array prematurely freed", f);
    }
    assert_eq!(h.rc(arr, ag), Some(1), "array rc must remain 1 (record's owning ref)");
}
