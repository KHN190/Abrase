// OpCode instruction basics
// Other tests should be pushed to region / memory

use polka::{BytecodeChunk, Chunk, NativeChunk, OpCode, Register, Module};
use myriad::{Value, VirtualMachine};
use std::rc::Rc;

fn r(n: u8) -> Register { Register(n) }

fn run(ops: Vec<OpCode>, constants: Vec<Value>) -> Result<Value, String> {
    let reg_count = 256;
    VirtualMachine::new().run(&Chunk::Bytecode(BytecodeChunk {
        code: ops, constants, reg_count, param_count: 0, string_constants: Vec::new(),
    }))
}

fn run_module_with_param_counts(functions: Vec<(Vec<OpCode>, Vec<Value>, usize, usize)>) -> Result<Value, String> {
    let num_functions = functions.len();
    let chunks: Vec<Chunk> = functions
        .into_iter()
        .map(|(code, constants, reg_count, param_count)| {
            Chunk::Bytecode(BytecodeChunk { code, constants, reg_count, param_count, string_constants: Vec::new() })
        })
        .collect();
    let module = Module { functions: chunks, entry: num_functions - 1, device_mask: [0; 32] };
    VirtualMachine::new().run_module(&module)
}

#[test]
fn test_push_const_and_ret() {
    let result = run(
        vec![OpCode::PushConst(r(0), 0), OpCode::Ret(r(0))],
        vec![Value::from_int(42)],
    );
    assert_eq!(result, Ok(Value::from_int(42)));
}

#[test]
fn test_mov() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::Copy(r(1), r(0)),
            OpCode::Ret(r(1)),
        ],
        vec![Value::from_int(7)],
    );
    assert_eq!(result, Ok(Value::from_int(7)));
}

#[test]
fn test_add() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::PushConst(r(1), 1),
            OpCode::Add(r(2), r(0), r(1)),
            OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(3), Value::from_int(4)],
    );
    assert_eq!(result, Ok(Value::from_int(7)));
}

#[test]
fn test_sub() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::PushConst(r(1), 1),
            OpCode::Sub(r(2), r(0), r(1)),
            OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(10), Value::from_int(3)],
    );
    assert_eq!(result, Ok(Value::from_int(7)));
}

#[test]
fn test_mul() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::PushConst(r(1), 1),
            OpCode::Mul(r(2), r(0), r(1)),
            OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(6), Value::from_int(7)],
    );
    assert_eq!(result, Ok(Value::from_int(42)));
}

#[test]
fn test_div() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::PushConst(r(1), 1),
            OpCode::Div(r(2), r(0), r(1)),
            OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(20), Value::from_int(4)],
    );
    assert_eq!(result, Ok(Value::from_int(5)));
}

#[test]
fn test_div_by_zero_traps() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::PushConst(r(1), 1),
            OpCode::Div(r(2), r(0), r(1)),
            OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(10), Value::from_int(0)],
    );
    assert!(result.is_err());
}

#[test]
fn test_mod() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::PushConst(r(1), 1),
            OpCode::Mod(r(2), r(0), r(1)),
            OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(10), Value::from_int(3)],
    );
    assert_eq!(result, Ok(Value::from_int(1)));
}

#[test]
fn test_empty_chunk_returns_unit() {
    assert!(run(vec![], vec![]).is_err());
}

#[test]
fn test_ret_empty_register_errors() {
    let result = run(vec![OpCode::Ret(r(0))], vec![]);
    assert!(result.is_err());
}

#[test]
fn test_eq_true() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::PushConst(r(1), 0),
            OpCode::Eq(r(2), r(0), r(1)),
            OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(5)],
    );
    assert_eq!(result, Ok(Value::from_bool(true)));
}

#[test]
fn test_eq_false() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::PushConst(r(1), 1),
            OpCode::Eq(r(2), r(0), r(1)),
            OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(5), Value::from_int(3)],
    );
    assert_eq!(result, Ok(Value::from_bool(false)));
}

#[test]
fn test_neq_true() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::PushConst(r(1), 1),
            OpCode::Neq(r(2), r(0), r(1)),
            OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(5), Value::from_int(3)],
    );
    assert_eq!(result, Ok(Value::from_bool(true)));
}

#[test]
fn test_neq_false() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::PushConst(r(1), 0),
            OpCode::Neq(r(2), r(0), r(1)),
            OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(5)],
    );
    assert_eq!(result, Ok(Value::from_bool(false)));
}

#[test]
fn test_lt_true() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::PushConst(r(1), 1),
            OpCode::Lt(r(2), r(0), r(1)),
            OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(3), Value::from_int(5)],
    );
    assert_eq!(result, Ok(Value::from_bool(true)));
}

#[test]
fn test_lt_false() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::PushConst(r(1), 1),
            OpCode::Lt(r(2), r(0), r(1)),
            OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(5), Value::from_int(3)],
    );
    assert_eq!(result, Ok(Value::from_bool(false)));
}

#[test]
fn test_gt_true() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::PushConst(r(1), 1),
            OpCode::Gt(r(2), r(0), r(1)),
            OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(5), Value::from_int(3)],
    );
    assert_eq!(result, Ok(Value::from_bool(true)));
}

#[test]
fn test_gt_false() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::PushConst(r(1), 1),
            OpCode::Gt(r(2), r(0), r(1)),
            OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(3), Value::from_int(5)],
    );
    assert_eq!(result, Ok(Value::from_bool(false)));
}

#[test]
fn test_lte_true() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::PushConst(r(1), 1),
            OpCode::Lte(r(2), r(0), r(1)),
            OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(5), Value::from_int(5)],
    );
    assert_eq!(result, Ok(Value::from_bool(true)));
}

#[test]
fn test_lte_false() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::PushConst(r(1), 1),
            OpCode::Lte(r(2), r(0), r(1)),
            OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(5), Value::from_int(3)],
    );
    assert_eq!(result, Ok(Value::from_bool(false)));
}

#[test]
fn test_gte_true() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::PushConst(r(1), 1),
            OpCode::Gte(r(2), r(0), r(1)),
            OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(5), Value::from_int(3)],
    );
    assert_eq!(result, Ok(Value::from_bool(true)));
}

#[test]
fn test_gte_false() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::PushConst(r(1), 1),
            OpCode::Gte(r(2), r(0), r(1)),
            OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(3), Value::from_int(5)],
    );
    assert_eq!(result, Ok(Value::from_bool(false)));
}

#[test]
fn test_jz_takes_jump_when_zero() {
    // Jz on 0 skips one PushConst and lands on the final r1=42 store.
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::Jz(r(0), 1),
            OpCode::PushConst(r(1), 1),
            OpCode::PushConst(r(1), 1),
            OpCode::Ret(r(1)),
        ],
        vec![Value::from_int(0), Value::from_int(42)],
    );
    assert_eq!(result, Ok(Value::from_int(42)));
}

#[test]
fn test_jz_skips_jump_when_nonzero() {
    // Jz on non-zero falls through to the next PushConst.
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::Jz(r(0), 2),
            OpCode::PushConst(r(1), 1),
            OpCode::Ret(r(1)),
        ],
        vec![Value::from_int(1), Value::from_int(99)],
    );
    assert_eq!(result, Ok(Value::from_int(99)));
}

#[test]
fn test_jnz_takes_jump_when_nonzero() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::Jnz(r(0), 1),
            OpCode::PushConst(r(1), 1),
            OpCode::PushConst(r(1), 1),
            OpCode::Ret(r(1)),
        ],
        vec![Value::from_int(5), Value::from_int(42)],
    );
    assert_eq!(result, Ok(Value::from_int(42)));
}

#[test]
fn test_jnz_skips_jump_when_zero() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::Jnz(r(0), 2),
            OpCode::PushConst(r(1), 1),
            OpCode::Ret(r(1)),
        ],
        vec![Value::from_int(0), Value::from_int(99)],
    );
    assert_eq!(result, Ok(Value::from_int(99)));
}

#[test]
fn test_jmp_unconditional() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::Jmp(1),
            OpCode::PushConst(r(1), 1),
            OpCode::Ret(r(0)),
        ],
        vec![Value::from_int(42), Value::from_int(99)],
    );
    assert_eq!(result, Ok(Value::from_int(42)));
}

#[test]
fn test_jz_with_bool_false_takes_jump() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::Jz(r(0), 1),
            OpCode::PushConst(r(1), 1),
            OpCode::PushConst(r(1), 1),
            OpCode::Ret(r(1)),
        ],
        vec![Value::from_bool(false), Value::from_int(42)],
    );
    assert_eq!(result, Ok(Value::from_int(42)));
}

#[test]
fn test_loop_counter() {
    // while i<3 { i += 1 } returns 3.
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::PushConst(r(1), 1),
            OpCode::Lt(r(2), r(0), r(1)),
            OpCode::Jz(r(2), 3),
            OpCode::PushConst(r(3), 2),
            OpCode::Add(r(0), r(0), r(3)),
            OpCode::Jmp(-5),
            OpCode::Ret(r(0)),
        ],
        vec![Value::from_int(0), Value::from_int(3), Value::from_int(1)],
    );
    assert_eq!(result, Ok(Value::from_int(3)));
}

#[test]
fn test_lte_equal_boundary() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::PushConst(r(1), 0),
            OpCode::Lte(r(2), r(0), r(1)),
            OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(5)],
    );
    assert_eq!(result, Ok(Value::from_bool(true)));
}

#[test]
fn test_gte_equal_boundary() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::PushConst(r(1), 0),
            OpCode::Gte(r(2), r(0), r(1)),
            OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(5)],
    );
    assert_eq!(result, Ok(Value::from_bool(true)));
}

#[test]
fn test_jz_falsy_int_zero() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::Jz(r(0), 1),
            OpCode::PushConst(r(1), 1),
            OpCode::PushConst(r(1), 1),
            OpCode::Ret(r(1)),
        ],
        vec![Value::from_int(0), Value::from_int(42)],
    );
    assert_eq!(result, Ok(Value::from_int(42)));
}

#[test]
fn test_jz_truthy_int_nonzero() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::Jz(r(0), 1),
            OpCode::PushConst(r(1), 1),
            OpCode::Ret(r(1)),
        ],
        vec![Value::from_int(7), Value::from_int(99)],
    );
    assert_eq!(result, Ok(Value::from_int(99)));
}

#[test]
fn test_jnz_truthy_int_nonzero() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::Jnz(r(0), 1),
            OpCode::PushConst(r(1), 1),
            OpCode::PushConst(r(1), 1),
            OpCode::Ret(r(1)),
        ],
        vec![Value::from_int(7), Value::from_int(42)],
    );
    assert_eq!(result, Ok(Value::from_int(42)));
}

#[test]
fn test_jnz_falsy_int_zero() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::Jnz(r(0), 1),
            OpCode::PushConst(r(1), 1),
            OpCode::Ret(r(1)),
        ],
        vec![Value::from_int(0), Value::from_int(99)],
    );
    assert_eq!(result, Ok(Value::from_int(99)));
}

#[test]
fn test_mov_empty_source_register_errors() {
    let result = run(
        vec![
            OpCode::Copy(r(0), r(1)),
            OpCode::Ret(r(0)),
        ],
        vec![],
    );
    assert!(result.is_err());
}

#[test]
fn test_jz_empty_register_errors() {
    let result = run(
        vec![
            OpCode::Jz(r(0), 0),
            OpCode::Ret(r(1)),
        ],
        vec![],
    );
    assert!(result.is_err());
}

#[test]
fn test_jz_with_unit_falsy() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::Jz(r(0), 1),
            OpCode::PushConst(r(1), 1),
            OpCode::PushConst(r(1), 1),
            OpCode::Ret(r(1)),
        ],
        vec![Value::UNIT, Value::from_int(42)],
    );
    assert_eq!(result, Ok(Value::from_int(42)));
}

#[test]
fn test_jnz_with_bool_true() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::Jnz(r(0), 1),
            OpCode::PushConst(r(1), 1),
            OpCode::PushConst(r(1), 1),
            OpCode::Ret(r(1)),
        ],
        vec![Value::from_bool(true), Value::from_int(42)],
    );
    assert_eq!(result, Ok(Value::from_int(42)));
}

#[test]
fn test_mod_by_zero_traps() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::PushConst(r(1), 1),
            OpCode::Mod(r(2), r(0), r(1)),
            OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(10), Value::from_int(0)],
    );
    assert!(result.is_err());
}

#[test]
fn test_jnz_empty_register_errors() {
    let result = run(
        vec![
            OpCode::Jnz(r(0), 0),
            OpCode::Ret(r(1)),
        ],
        vec![],
    );
    assert!(result.is_err());
}

#[test]
fn test_add_empty_left_register_errors() {
    let result = run(
        vec![
            OpCode::PushConst(r(1), 0),
            OpCode::Add(r(2), r(0), r(1)),
            OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(5)],
    );
    assert!(result.is_err());
}

#[test]
fn test_add_empty_right_register_errors() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::Add(r(2), r(0), r(1)),
            OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(5)],
    );
    assert!(result.is_err());
}

#[test]
fn test_sub_empty_left_register_errors() {
    let result = run(
        vec![
            OpCode::PushConst(r(1), 0),
            OpCode::Sub(r(2), r(0), r(1)),
            OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(5)],
    );
    assert!(result.is_err());
}

#[test]
fn test_mul_empty_right_register_errors() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::Mul(r(2), r(0), r(1)),
            OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(5)],
    );
    assert!(result.is_err());
}

#[test]
fn test_call_simple() {
    // Callee adds its two params (passed via caller_reg_count + 0..n).
    let result = run_module_with_param_counts(vec![
        (
            vec![
                OpCode::Add(r(0), r(0), r(1)),
                OpCode::Ret(r(0)),
            ],
            vec![],
            2, 2,
        ),
        (
            vec![
                OpCode::PushConst(r(1), 0),
                OpCode::PushConst(r(2), 1),
                OpCode::Call(r(0), 0),
                OpCode::Ret(r(0)),
            ],
            vec![Value::from_int(2), Value::from_int(3)],
            1, 0,
        ),
    ]);
    assert_eq!(result, Ok(Value::from_int(5)));
}

#[test]
fn test_call_passes_args_to_callee() {
    let result = run_module_with_param_counts(vec![
        (
            vec![OpCode::Ret(r(1))],
            vec![],
            2, 2,
        ),
        (
            vec![
                OpCode::PushConst(r(1), 0),
                OpCode::PushConst(r(2), 1),
                OpCode::Call(r(0), 0),
                OpCode::Ret(r(0)),
            ],
            vec![Value::from_int(10), Value::from_int(20)],
            1, 0,
        ),
    ]);
    assert_eq!(result, Ok(Value::from_int(20)));
}

#[test]
fn test_call_return_value_in_dest() {
    let result = run_module_with_param_counts(vec![
        (
            vec![
                OpCode::PushConst(r(1), 0),
                OpCode::Add(r(0), r(0), r(1)),
                OpCode::Ret(r(0)),
            ],
            vec![Value::from_int(1)],
            2, 1,
        ),
        (
            vec![
                OpCode::PushConst(r(1), 0),
                OpCode::Call(r(0), 0),
                OpCode::Ret(r(0)),
            ],
            vec![Value::from_int(5)],
            1, 0,
        ),
    ]);
    assert_eq!(result, Ok(Value::from_int(6)));
}

#[test]
fn test_recursion_simple() {
    // countdown(n) recursively bottoms out at 0; verifies frame stacking on Call.
    let result = run_module_with_param_counts(vec![
        (
            vec![
                OpCode::PushConst(r(1), 0),
                OpCode::Lte(r(2), r(0), r(1)),
                OpCode::Jz(r(2), 2),
                OpCode::PushConst(r(0), 1),
                OpCode::Ret(r(0)),
                OpCode::PushConst(r(1), 2),
                OpCode::Sub(r(0), r(0), r(1)),
                OpCode::Copy(r(4), r(0)),
                OpCode::Call(r(3), 0),
                OpCode::Ret(r(3)),
            ],
            vec![Value::from_int(0), Value::from_int(0), Value::from_int(1)],
            4, 1,
        ),
        (
            vec![
                OpCode::PushConst(r(1), 0),
                OpCode::Call(r(0), 0),
                OpCode::Ret(r(0)),
            ],
            vec![Value::from_int(2)],
            1, 0,
        ),
    ]);
    assert_eq!(result, Ok(Value::from_int(0)));
}

#[test]
fn test_alloc_and_free() {
    let result = run(
        vec![
            OpCode::Alloc(r(0), 8),
            OpCode::Ret(r(0)),
        ],
        vec![],
    );

    match result {
        Ok(v) if v.is_handle() => {},
        _ => panic!("Expected Handle from Alloc, got {:?}", result),
    }
}

#[test]
fn test_store_and_load() {
    let result = run(
        vec![
            OpCode::Alloc(r(0), 8),       // r0 = heap pointer
            OpCode::PushConst(r(1), 0),   // r1 = 42
            OpCode::St(r(1), r(0), 0),    // store 42 at [r0+0]
            OpCode::Ld(r(2), r(0), 0),    // r2 = load from [r0+0]
            OpCode::Ret(r(2)),             // return r2
        ],
        vec![Value::from_int(42)],
    );
    assert_eq!(result, Ok(Value::from_int(42)));
}

#[test]
fn test_store_multiple_fields() {
    let result = run(
        vec![
            OpCode::Alloc(r(0), 16),      // r0 = heap pointer (16 bytes for 2 Int fields)
            OpCode::PushConst(r(1), 0),   // r1 = 10
            OpCode::St(r(1), r(0), 0),    // store 10 at [r0+0]
            OpCode::PushConst(r(2), 1),   // r2 = 20
            OpCode::St(r(2), r(0), 8),    // store 20 at [r0+8]
            OpCode::Ld(r(3), r(0), 0),    // r3 = load from [r0+0] = 10
            OpCode::Ld(r(4), r(0), 8),    // r4 = load from [r0+8] = 20
            OpCode::Add(r(5), r(3), r(4)),// r5 = 10 + 20 = 30
            OpCode::Ret(r(5)),             // return r5
        ],
        vec![Value::from_int(10), Value::from_int(20)],
    );
    assert_eq!(result, Ok(Value::from_int(30)));
}

#[test]
fn test_heap_ld_rejects_stale_generation() {
    use myriad::memory::Heap;
    use myriad::BoxPool;
    let mut heap = Heap::new();
    let mut pool = BoxPool::new();
    let (slot, gen0) = heap.alloc(2);
    heap.rc_dec(slot, gen0, &mut pool).unwrap();
    let (slot2, gen1) = heap.alloc(2);
    assert_eq!(slot2, slot, "free_list should reuse the slot");
    assert_ne!(gen0, gen1, "reused slot must bump its generation");

    let err = heap.ld(slot, gen0, 0).unwrap_err();
    assert!(err.contains("stale handle"), "got: {}", err);

    heap.st(slot2, gen1, 0, Value::from_int(7)).unwrap();
    assert_eq!(heap.ld(slot2, gen1, 0).unwrap(), Value::from_int(7));
}

#[test]
fn test_call_dispatches_to_native_chunk() {
    let caller = BytecodeChunk {
        code: vec![
            OpCode::PushConst(r(0), 0),
            OpCode::PushConst(r(1), 1),
            OpCode::Copy(r(3), r(0)),
            OpCode::Copy(r(4), r(1)),
            OpCode::Call(r(2), 1),
            OpCode::Ret(r(2)),
        ],
        constants: vec![Value::from_int(7), Value::from_int(35)],
        reg_count: 3,
        param_count: 0, string_constants: Vec::new(),
    };
    let native = NativeChunk {
        param_count: 2,
        name: "test_add".into(),
    };
    let module = Module {
        functions: vec![Chunk::Bytecode(caller), Chunk::Native(native)],
        entry: 0, device_mask: [0; 32]
    };
    let mut vm = VirtualMachine::new();
    vm.register_native("test_add", Rc::new(|_ctx: &mut myriad::NativeCtx<'_>, args: &[Value]| {
        let a = args[0].as_int().ok_or("expected int")?;
        let b = args[1].as_int().ok_or("expected int")?;
        Ok(Value::from_int(a + b))
    }));
    let result = vm.run_module(&module);
    assert_eq!(result, Ok(Value::from_int(42)));
}

#[test]
fn test_native_chunk_propagates_error() {
    let caller = BytecodeChunk {
        code: vec![
            OpCode::PushConst(r(0), 0),
            OpCode::Copy(r(2), r(0)),
            OpCode::Call(r(1), 1),
            OpCode::Ret(r(1)),
        ],
        constants: vec![Value::from_int(0)],
        reg_count: 2,
        param_count: 0, string_constants: Vec::new(),
    };
    let native = NativeChunk {
        param_count: 1,
        name: "test_boom".into(),
    };
    let module = Module {
        functions: vec![Chunk::Bytecode(caller), Chunk::Native(native)],
        entry: 0, device_mask: [0; 32]
    };
    let mut vm = VirtualMachine::new();
    vm.register_native("test_boom", Rc::new(|_ctx: &mut myriad::NativeCtx<'_>, _args: &[Value]| Err("boom".to_string())));
    let result = vm.run_module(&module);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("boom"));
}

#[test]
fn test_jmp_negative_offset_underflow_traps() {
    // pc 0: Jmp -10  — target would be pc = -9, must trap.
    let result = run(
        vec![
            OpCode::Jmp(-10),
            OpCode::PushConst(r(0), 0),
            OpCode::Ret(r(0)),
        ],
        vec![Value::from_int(42)],
    );
    assert!(result.is_err(), "negative PC must trap, got {:?}", result);
    let err = result.unwrap_err();
    assert!(err.contains("branch") || err.contains("out of range"),
            "expected branch range error, got: {}", err);
}

#[test]
fn test_jz_negative_offset_underflow_traps() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::Jz(r(0), -100),
            OpCode::Ret(r(0)),
        ],
        vec![Value::from_int(0)],
    );
    assert!(result.is_err(), "negative PC via Jz must trap, got {:?}", result);
}

#[test]
fn test_jnz_negative_offset_underflow_traps() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::Jnz(r(0), -100),
            OpCode::Ret(r(0)),
        ],
        vec![Value::from_int(1)],
    );
    assert!(result.is_err(), "negative PC via Jnz must trap, got {:?}", result);
}

#[test]
fn test_jz_not_taken_skips_validation() {
    // Untaken Jz must not validate its offset, even if it's wildly invalid.
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::Jz(r(0), -10000),  // not taken because r0 is non-zero
            OpCode::Ret(r(0)),
        ],
        vec![Value::from_int(7)],
    );
    assert_eq!(result, Ok(Value::from_int(7)));
}

#[test]
fn test_module_load_rejects_oversize_reg_count() {
    let bad = BytecodeChunk {
        code: vec![OpCode::Ret(r(0))],
        constants: vec![],
        reg_count: 257,
        param_count: 0, string_constants: Vec::new(),
    };
    let module = Module {
        functions: vec![Chunk::Bytecode(bad)],
        entry: 0, device_mask: [0; 32]
    };
    let result = VirtualMachine::new().run_module(&module);
    assert!(result.is_err(), "oversize reg_count must be rejected");
    let err = result.unwrap_err();
    assert!(err.contains("reg_count") && err.contains("frame budget"),
            "expected frame-budget error, got: {}", err);
}

#[test]
fn test_module_load_rejects_param_count_exceeds_reg_count() {
    let bad = BytecodeChunk {
        code: vec![OpCode::Ret(r(0))],
        constants: vec![],
        reg_count: 2,
        param_count: 5, string_constants: Vec::new(),
    };
    let module = Module {
        functions: vec![Chunk::Bytecode(bad)],
        entry: 0, device_mask: [0; 32]
    };
    let result = VirtualMachine::new().run_module(&module);
    assert!(result.is_err(), "param_count > reg_count must be rejected");
}

#[test]
fn test_module_load_accepts_exact_frame_budget() {
    // reg_count == FRAME_REGS (256) is OK.
    let chunk = BytecodeChunk {
        code: vec![
            OpCode::PushConst(r(0), 0),
            OpCode::Ret(r(0)),
        ],
        constants: vec![Value::from_int(7)],
        reg_count: 256,
        param_count: 0, string_constants: Vec::new(),
    };
    let module = Module {
        functions: vec![Chunk::Bytecode(chunk)],
        entry: 0, device_mask: [0; 32]
    };
    let result = VirtualMachine::new().run_module(&module);
    assert_eq!(result, Ok(Value::from_int(7)));
}

#[test]
fn test_jmp_past_end_traps() {
    // pc > code.len() is invalid (pc == code.len() is the legal fall-off-end).
    let result = run(
        vec![
            OpCode::Jmp(10),
            OpCode::Ret(r(0)),
        ],
        vec![],
    );
    assert!(result.is_err(), "branch past end must trap, got {:?}", result);
}

#[test]
fn test_call_reg_out_of_range_fn_id_traps() {
    let caller = BytecodeChunk {
        code: vec![
            OpCode::PushConst(r(1), 0),
            OpCode::CallReg(r(2), r(1)),
            OpCode::Ret(r(2)),
        ],
        constants: vec![Value::from_int(99999)],
        reg_count: 3,
        param_count: 0, string_constants: Vec::new(),
    };
    let module = Module {
        functions: vec![Chunk::Bytecode(caller)],
        entry: 0, device_mask: [0; 32],
    };
    let result = VirtualMachine::new().run_module(&module);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("out of u16 range"));
}

#[test]
fn test_call_reg_unknown_fn_id_traps() {
    let caller = BytecodeChunk {
        code: vec![
            OpCode::PushConst(r(1), 0),
            OpCode::CallReg(r(2), r(1)),
            OpCode::Ret(r(2)),
        ],
        constants: vec![Value::from_int(5)],
        reg_count: 3,
        param_count: 0, string_constants: Vec::new(),
    };
    let module = Module {
        functions: vec![Chunk::Bytecode(caller)],
        entry: 0, device_mask: [0; 32],
    };
    let result = VirtualMachine::new().run_module(&module);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unknown fn_id"));
}

#[test]
fn test_add_imm_positive() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::AddImm(r(1), r(0), 5),
            OpCode::Ret(r(1)),
        ],
        vec![Value::from_int(10)],
    );
    assert_eq!(result, Ok(Value::from_int(15)));
}

#[test]
fn test_add_imm_negative() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::AddImm(r(1), r(0), -3),
            OpCode::Ret(r(1)),
        ],
        vec![Value::from_int(10)],
    );
    assert_eq!(result, Ok(Value::from_int(7)));
}

#[test]
fn test_sub_imm() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::SubImm(r(1), r(0), 4),
            OpCode::Ret(r(1)),
        ],
        vec![Value::from_int(20)],
    );
    assert_eq!(result, Ok(Value::from_int(16)));
}
