// NaN-boxed Value: scalar equality and tag round-trips.
use abrase::bytecode::{BytecodeChunk, Chunk, OpCode, Register};
use abrase::vm::{Value, VirtualMachine};

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
