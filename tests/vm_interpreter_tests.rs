use ect::bytecode::{Chunk, OpCode, Register, Module};
use ect::vm::{Value, VirtualMachine};

fn r(n: u8) -> Register { Register(n) }

fn run(ops: Vec<OpCode>, constants: Vec<Value>) -> Result<Value, String> {
    let reg_count = 256;
    VirtualMachine::new().run(&Chunk { code: ops, constants, reg_count, param_count: 0 })
}

fn run_module_with_param_counts(functions: Vec<(Vec<OpCode>, Vec<Value>, usize, usize)>) -> Result<Value, String> {
    let num_functions = functions.len();
    let chunks: Vec<Chunk> = functions
        .into_iter()
        .map(|(code, constants, reg_count, param_count)| {
            Chunk { code, constants, reg_count, param_count }
        })
        .collect();
    let module = Module { functions: chunks, entry: num_functions - 1 };
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
    // Layout:
    //   pc 0: PushConst r0
    //   pc 1: Jz r0, +1 (skip pc 2, land on pc 3)
    //   pc 2: PushConst r1 (skipped)
    //   pc 3: PushConst r1
    //   pc 4: Ret r1
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
    // pc 0: PushConst, pc 1: Jz (offset 2 to past Ret, not taken), 
    // pc 2: PushConst, pc 3: Ret
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
    // Layout:
    //   pc 0: PushConst r0, 0  ; i = 0
    //   pc 1: PushConst r1, 1  ; limit = 3
    //   pc 2: Lt r2, r0, r1
    //   pc 3: Jz r2, +3        ; if !(i<limit), jump past loop body to pc 7
    //   pc 4: PushConst r3, 2  ; one
    //   pc 5: Add r0, r0, r3
    //   pc 6: Jmp -5           ; back to pc 2
    //   pc 7: Ret r0
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
    // Callee adds r0 and r1.
    // Main has reg_count=1 (just dest r0); 
    // args land at r1, r2 = caller_reg_count + 0, +1.
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
    // countdown(n): if n<=0 return 0 else countdown(n-1)
    // Locals: r0=n, r1=tmp, r2=cmp, r3=call_dest. reg_count=4, param_count=1.
    // Recursive arg lives at r4 (caller_reg_count + 0).
    //
    // pc 0: PushConst r1, 0   ; r1 = 0
    // pc 1: Lte r2, r0, r1
    // pc 2: Jz r2, +3         ; if !(n<=0), jump to pc 6
    // pc 3: PushConst r0, 1   ; r0 = 0 (return value)
    // pc 4: Ret r0
    // pc 5: (unreachable)
    // pc 6: PushConst r1, 2   ; r1 = 1
    // pc 7: Sub r0, r0, r1    ; r0 = n - 1
    // pc 8: Copy r4, r0       ; outbound arg
    // pc 9: Call r3, 0
    // pc 10: Ret r3
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
        Ok(Value::Int(_)) => {},
        _ => panic!("Expected Int pointer from Alloc"),
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

