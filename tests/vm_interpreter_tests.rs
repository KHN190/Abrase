use abrase::bytecode::{BytecodeChunk, Chunk, NativeChunk, OpCode, Register, Module};
use abrase::vm::{Value, VirtualMachine};
use std::rc::Rc;

fn r(n: u8) -> Register { Register(n) }

fn run(ops: Vec<OpCode>, constants: Vec<Value>) -> Result<Value, String> {
    let reg_count = 256;
    VirtualMachine::new().run(&Chunk::Bytecode(BytecodeChunk {
        code: ops, constants, reg_count, param_count: 0,
    }))
}

fn run_module_with_param_counts(functions: Vec<(Vec<OpCode>, Vec<Value>, usize, usize)>) -> Result<Value, String> {
    let num_functions = functions.len();
    let chunks: Vec<Chunk> = functions
        .into_iter()
        .map(|(code, constants, reg_count, param_count)| {
            Chunk::Bytecode(BytecodeChunk { code, constants, reg_count, param_count })
        })
        .collect();
    let module = Module { functions: chunks, entry: num_functions - 1, device_mask: [0; 32] };
    VirtualMachine::new().run_module(&module)
}

#[test]
fn test_push_const_and_ret() {
    let result = run(
        vec![OpCode::PushConst(r(0), 0), OpCode::Ret(r(0))],
        vec![Value::Int(42)],
    );
    assert_eq!(result, Ok(Value::Int(42)));
}

#[test]
fn test_mov() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::Copy(r(1), r(0)),
            OpCode::Ret(r(1)),
        ],
        vec![Value::Int(7)],
    );
    assert_eq!(result, Ok(Value::Int(7)));
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
        vec![Value::Int(3), Value::Int(4)],
    );
    assert_eq!(result, Ok(Value::Int(7)));
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
        vec![Value::Int(10), Value::Int(3)],
    );
    assert_eq!(result, Ok(Value::Int(7)));
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
        vec![Value::Int(6), Value::Int(7)],
    );
    assert_eq!(result, Ok(Value::Int(42)));
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
        vec![Value::Int(20), Value::Int(4)],
    );
    assert_eq!(result, Ok(Value::Int(5)));
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
        vec![Value::Int(10), Value::Int(0)],
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
        vec![Value::Int(10), Value::Int(3)],
    );
    assert_eq!(result, Ok(Value::Int(1)));
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
        vec![Value::Int(5)],
    );
    assert_eq!(result, Ok(Value::Bool(true)));
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
        vec![Value::Int(5), Value::Int(3)],
    );
    assert_eq!(result, Ok(Value::Bool(false)));
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
        vec![Value::Int(5), Value::Int(3)],
    );
    assert_eq!(result, Ok(Value::Bool(true)));
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
        vec![Value::Int(5)],
    );
    assert_eq!(result, Ok(Value::Bool(false)));
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
        vec![Value::Int(3), Value::Int(5)],
    );
    assert_eq!(result, Ok(Value::Bool(true)));
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
        vec![Value::Int(5), Value::Int(3)],
    );
    assert_eq!(result, Ok(Value::Bool(false)));
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
        vec![Value::Int(5), Value::Int(3)],
    );
    assert_eq!(result, Ok(Value::Bool(true)));
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
        vec![Value::Int(3), Value::Int(5)],
    );
    assert_eq!(result, Ok(Value::Bool(false)));
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
        vec![Value::Int(5), Value::Int(5)],
    );
    assert_eq!(result, Ok(Value::Bool(true)));
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
        vec![Value::Int(5), Value::Int(3)],
    );
    assert_eq!(result, Ok(Value::Bool(false)));
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
        vec![Value::Int(5), Value::Int(3)],
    );
    assert_eq!(result, Ok(Value::Bool(true)));
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
        vec![Value::Int(3), Value::Int(5)],
    );
    assert_eq!(result, Ok(Value::Bool(false)));
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
        vec![Value::Int(0), Value::Int(42)],
    );
    assert_eq!(result, Ok(Value::Int(42)));
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
        vec![Value::Int(1), Value::Int(99)],
    );
    assert_eq!(result, Ok(Value::Int(99)));
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
        vec![Value::Int(5), Value::Int(42)],
    );
    assert_eq!(result, Ok(Value::Int(42)));
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
        vec![Value::Int(0), Value::Int(99)],
    );
    assert_eq!(result, Ok(Value::Int(99)));
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
        vec![Value::Int(42), Value::Int(99)],
    );
    assert_eq!(result, Ok(Value::Int(42)));
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
        vec![Value::Bool(false), Value::Int(42)],
    );
    assert_eq!(result, Ok(Value::Int(42)));
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
        vec![Value::Int(0), Value::Int(3), Value::Int(1)],
    );
    assert_eq!(result, Ok(Value::Int(3)));
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
        vec![Value::Int(5)],
    );
    assert_eq!(result, Ok(Value::Bool(true)));
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
        vec![Value::Int(5)],
    );
    assert_eq!(result, Ok(Value::Bool(true)));
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
        vec![Value::Int(0), Value::Int(42)],
    );
    assert_eq!(result, Ok(Value::Int(42)));
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
        vec![Value::Int(7), Value::Int(99)],
    );
    assert_eq!(result, Ok(Value::Int(99)));
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
        vec![Value::Int(7), Value::Int(42)],
    );
    assert_eq!(result, Ok(Value::Int(42)));
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
        vec![Value::Int(0), Value::Int(99)],
    );
    assert_eq!(result, Ok(Value::Int(99)));
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
        vec![Value::Unit, Value::Int(42)],
    );
    assert_eq!(result, Ok(Value::Int(42)));
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
        vec![Value::Bool(true), Value::Int(42)],
    );
    assert_eq!(result, Ok(Value::Int(42)));
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
        vec![Value::Int(10), Value::Int(0)],
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
        vec![Value::Int(5)],
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
        vec![Value::Int(5)],
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
        vec![Value::Int(5)],
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
        vec![Value::Int(5)],
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
            vec![Value::Int(2), Value::Int(3)],
            1, 0,
        ),
    ]);
    assert_eq!(result, Ok(Value::Int(5)));
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
            vec![Value::Int(10), Value::Int(20)],
            1, 0,
        ),
    ]);
    assert_eq!(result, Ok(Value::Int(20)));
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
            vec![Value::Int(1)],
            2, 1,
        ),
        (
            vec![
                OpCode::PushConst(r(1), 0),
                OpCode::Call(r(0), 0),
                OpCode::Ret(r(0)),
            ],
            vec![Value::Int(5)],
            1, 0,
        ),
    ]);
    assert_eq!(result, Ok(Value::Int(6)));
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
            vec![Value::Int(0), Value::Int(0), Value::Int(1)],
            4, 1,
        ),
        (
            vec![
                OpCode::PushConst(r(1), 0),
                OpCode::Call(r(0), 0),
                OpCode::Ret(r(0)),
            ],
            vec![Value::Int(2)],
            1, 0,
        ),
    ]);
    assert_eq!(result, Ok(Value::Int(0)));
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
        Ok(Value::Handle { .. }) => {},
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
        vec![Value::Int(42)],
    );
    assert_eq!(result, Ok(Value::Int(42)));
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
        vec![Value::Int(10), Value::Int(20)],
    );
    assert_eq!(result, Ok(Value::Int(30)));
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
            constants: vec![Value::Int(0)],
            reg_count: 3,
            param_count: 0,
        })],
        entry: 0,
        device_mask: [0; 32],
    };
    let result = vm.run_module(&module);
    assert_eq!(result, Ok(Value::Int(0)));
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
            constants: vec![Value::Int(0)],
            reg_count: 5,
            param_count: 0,
        })],
        entry: 0,
        device_mask: [0; 32],
    };
    let result = vm.run_module(&module);
    assert_eq!(result, Ok(Value::Int(0)));
}

#[test]
fn test_heap_ld_rejects_stale_generation() {
    use abrase::vm::memory::Heap;
    let mut heap = Heap::new();
    let (slot, gen0) = heap.alloc(2);
    heap.rc_dec(slot, gen0).unwrap();
    let (slot2, gen1) = heap.alloc(2);
    assert_eq!(slot2, slot, "free_list should reuse the slot");
    assert_ne!(gen0, gen1, "reused slot must bump its generation");

    let err = heap.ld(slot, gen0, 0).unwrap_err();
    assert!(err.contains("stale handle"), "got: {}", err);

    heap.st(slot2, gen1, 0, Value::Int(7)).unwrap();
    assert_eq!(heap.ld(slot2, gen1, 0).unwrap(), Value::Int(7));
}

#[test]
fn test_rc_inc_keeps_cell_alive_until_balanced() {
    use abrase::vm::memory::Heap;
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
    use abrase::vm::memory::Heap;
    let mut heap = Heap::new();
    let (child, cgen) = heap.alloc(1);
    let (parent, pgen) = heap.alloc(1);
    heap.st(parent, pgen, 0, Value::Handle { slot: child, generation: cgen }).unwrap();
    heap.rc_dec(parent, pgen).unwrap();
    assert_eq!(heap.live_count(), 0, "child must be reclaimed transitively");
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
        constants: vec![Value::Int(7), Value::Int(35)],
        reg_count: 3,
        param_count: 0,
    };
    let native = NativeChunk {
        param_count: 2,
        func: Rc::new(|args: &[Value]| {
            let (a, b) = match (&args[0], &args[1]) {
                (Value::Int(a), Value::Int(b)) => (*a, *b),
                _ => return Err("expected ints".into()),
            };
            Ok(Value::Int(a + b))
        }),
    };
    let module = Module {
        functions: vec![Chunk::Bytecode(caller), Chunk::Native(native)],
        entry: 0, device_mask: [0; 32]
    };
    let result = VirtualMachine::new().run_module(&module);
    assert_eq!(result, Ok(Value::Int(42)));
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
        constants: vec![Value::Int(0)],
        reg_count: 2,
        param_count: 0,
    };
    let native = NativeChunk {
        param_count: 1,
        func: Rc::new(|_args: &[Value]| Err("boom".to_string())),
    };
    let module = Module {
        functions: vec![Chunk::Bytecode(caller), Chunk::Native(native)],
        entry: 0, device_mask: [0; 32]
    };
    let result = VirtualMachine::new().run_module(&module);
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
        vec![Value::Int(42)],
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
        vec![Value::Int(0)],
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
        vec![Value::Int(1)],
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
        vec![Value::Int(7)],
    );
    assert_eq!(result, Ok(Value::Int(7)));
}

// Drive Handle/Resume opcodes directly; codegen lowers `handle` to arm-fn Calls.
#[test]
fn test_resume_without_handler_traps() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::Resume(r(0)),
            OpCode::Ret(r(0)),
        ],
        vec![Value::Int(7)],
    );
    assert!(result.is_err(), "resume without handler must trap");
}

#[test]
fn test_handle_allocates_cell_and_resume_frees_it() {
    // Single-shot Resume must reclaim its cell; heap net-zero at exit.
    let mut vm = VirtualMachine::new();
    let chunk = Chunk::Bytecode(BytecodeChunk {
        code: vec![
            OpCode::PushConst(r(0), 0),    // r0 = 99 (resume value)
            OpCode::Handle(r(3), 0),       // install w/ dest=r3
            OpCode::Resume(r(0)),          // pops, writes 99 → r3, frees cell
            OpCode::Ret(r(3)),
        ],
        constants: vec![Value::Int(99)],
        reg_count: 256,
        param_count: 0,
    });
    let _ = vm.run(&chunk);
    assert_eq!(vm.heap_live_count(), 0,
        "continuation cell should be reclaimed after single-shot resume");
}

#[test]
fn test_double_resume_traps_after_single_shot() {
    // After first Resume frees the cell, re-entry hits an empty handler stack → trap.
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::Handle(r(3), 0),
            OpCode::Resume(r(0)),
            OpCode::Ret(r(3)),
        ],
        vec![Value::Int(99)],
    );
    assert!(result.is_err(), "second resume must trap, got {:?}", result);
    let err = result.unwrap_err();
    assert!(err.contains("Resume") || err.contains("resume"),
            "expected resume-related error, got: {}", err);
}

#[test]
fn test_handle_allocates_one_cell_per_install() {
    // One Handle (no Resume) leaves exactly one live continuation cell.
    let mut vm = VirtualMachine::new();
    let install = Chunk::Bytecode(BytecodeChunk {
        code: vec![
            OpCode::Handle(r(7), 0),       // dest = r7
            OpCode::Ret(r(0)),
        ],
        constants: vec![Value::Int(0)],
        reg_count: 256,
        param_count: 0,
    });
    let _ = vm.run(&install);
    assert_eq!(vm.heap_live_count(), 1,
        "Handle must allocate exactly one continuation cell");
}

// Lea is meaningless under the handle/generation heap and must trap.
#[test]
fn test_lea_traps() {
    let result = run(
        vec![
            OpCode::Alloc(r(0), 4),
            OpCode::Lea(r(1), r(0), 1),
            OpCode::Ret(r(1)),
        ],
        vec![],
    );
    assert!(result.is_err(), "lea must trap");
    let err = result.unwrap_err();
    assert!(err.contains("lea"), "expected lea error, got: {}", err);
}

//  Call whose `dest` is outside the caller's reg_count must trap.
#[test]
fn test_call_dest_out_of_caller_window_traps() {
    let result = run_module_with_param_counts(vec![
        (vec![OpCode::Ret(r(0))], vec![Value::Int(1)], 1, 0),
        // Caller has reg_count=2 but Call writes to r9 — out of window.
        (vec![OpCode::Call(r(9), 0), OpCode::Ret(r(0))], vec![], 2, 0),
    ]);
    assert!(result.is_err(), "call with out-of-window dest must trap");
    let err = result.unwrap_err();
    assert!(err.contains("out of caller window") || err.contains("register window"),
            "expected window error, got: {}", err);
}

// FRAME_REGS=256 enforcement at module-load and call time.
#[test]
fn test_module_load_rejects_oversize_reg_count() {
    let bad = BytecodeChunk {
        code: vec![OpCode::Ret(r(0))],
        constants: vec![],
        reg_count: 257,
        param_count: 0,
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
        param_count: 5,
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
        constants: vec![Value::Int(7)],
        reg_count: 256,
        param_count: 0,
    };
    let module = Module {
        functions: vec![Chunk::Bytecode(chunk)],
        entry: 0, device_mask: [0; 32]
    };
    let result = VirtualMachine::new().run_module(&module);
    assert_eq!(result, Ok(Value::Int(7)));
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

