// VM dispatch loop, register operations, arithmetic.
use ect::bytecode::{Chunk, OpCode, Register};
use ect::vm::{Value, VirtualMachine};

fn r(n: u8) -> Register { Register(n) }

fn run(ops: Vec<OpCode>, constants: Vec<Value>) -> Result<Value, String> {
    VirtualMachine::new().run(&Chunk { code: ops, constants })
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
            OpCode::Mov(r(1), r(0)),
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
fn test_div_by_zero_returns_unit() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::PushConst(r(1), 1),
            OpCode::Div(r(2), r(0), r(1)),
            OpCode::Ret(r(2)),
        ],
        vec![Value::Int(10), Value::Int(0)],
    );
    assert_eq!(result, Ok(Value::Unit));
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
    assert_eq!(run(vec![], vec![]), Ok(Value::Unit));
}

#[test]
fn test_ret_empty_register_errors() {
    let result = run(vec![OpCode::Ret(r(0))], vec![]);
    assert!(result.is_err());
}

// Comparison Operators
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

// Jump Instructions
#[test]
fn test_jz_takes_jump_when_zero() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),       // r0 = 0 (false)
            OpCode::Jz(r(0), 3),              // if r0==0, jump to instruction 3
            OpCode::PushConst(r(1), 1),       // (skipped) r1 = 99
            OpCode::PushConst(r(1), 1),       // r1 = 42
            OpCode::Ret(r(1)),
        ],
        vec![Value::Int(0), Value::Int(42)],
    );
    assert_eq!(result, Ok(Value::Int(42)));
}

#[test]
fn test_jz_skips_jump_when_nonzero() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),       // r0 = 1 (true)
            OpCode::Jz(r(0), 4),              // if r0==0, jump (won't happen)
            OpCode::PushConst(r(1), 1),       // r1 = 99
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
            OpCode::PushConst(r(0), 0),       // r0 = 5 (true)
            OpCode::Jnz(r(0), 3),             // if r0!=0, jump to instruction 3
            OpCode::PushConst(r(1), 1),       // (skipped) r1 = 99
            OpCode::PushConst(r(1), 1),       // r1 = 42
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
            OpCode::PushConst(r(0), 0),       // r0 = 0 (false)
            OpCode::Jnz(r(0), 4),             // if r0!=0, jump (won't happen)
            OpCode::PushConst(r(1), 1),       // r1 = 99
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
            OpCode::Jmp(3),
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
            OpCode::PushConst(r(0), 0),       // r0 = false
            OpCode::Jz(r(0), 3),              // if r0==false (falsy), jump to instruction 3
            OpCode::PushConst(r(1), 1),       // (skipped) r1 = 99
            OpCode::PushConst(r(1), 1),       // r1 = 42
            OpCode::Ret(r(1)),
        ],
        vec![Value::Bool(false), Value::Int(42)],
    );
    assert_eq!(result, Ok(Value::Int(42)));
}

#[test]
fn test_loop_counter() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0), // i = 0
            OpCode::PushConst(r(1), 1), // limit = 3
            // Loop: check i < limit
            OpCode::Lt(r(2), r(0), r(1)),  // r2 = i < limit
            OpCode::Jz(r(2), 7),           // if !r2, jump to end
            // Increment: i = i + 1
            OpCode::PushConst(r(3), 2),    // r3 = 1
            OpCode::Add(r(0), r(0), r(3)), // i = i + 1
            OpCode::Jmp(2),                // jump back to loop check
            // End: return i
            OpCode::Ret(r(0)),
        ],
        vec![Value::Int(0), Value::Int(3), Value::Int(1)],
    );
    assert_eq!(result, Ok(Value::Int(3)));
}

// Boundary conditions for comparisons
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

// is_falsy edge cases
#[test]
fn test_jz_falsy_int_zero() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),      // r0 = 0 (falsy)
            OpCode::Jz(r(0), 3),              // if r0 is falsy, jump
            OpCode::PushConst(r(1), 1),       // (skipped) r1 = 99
            OpCode::PushConst(r(1), 1),       // r1 = 42
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
            OpCode::PushConst(r(0), 0),      // r0 = 7 (truthy)
            OpCode::Jz(r(0), 3),              // if r0 is falsy, jump (won't happen)
            OpCode::PushConst(r(1), 1),       // r1 = 99
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
            OpCode::PushConst(r(0), 0),      // r0 = 7 (truthy)
            OpCode::Jnz(r(0), 3),             // if r0 is truthy, jump
            OpCode::PushConst(r(1), 1),       // (skipped) r1 = 99
            OpCode::PushConst(r(1), 1),       // r1 = 42
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
            OpCode::PushConst(r(0), 0),      // r0 = 0 (falsy)
            OpCode::Jnz(r(0), 3),             // if r0 is truthy, jump (won't happen)
            OpCode::PushConst(r(1), 1),       // r1 = 99
            OpCode::Ret(r(1)),
        ],
        vec![Value::Int(0), Value::Int(99)],
    );
    assert_eq!(result, Ok(Value::Int(99)));
}

// Error cases
#[test]
fn test_mov_empty_source_register_errors() {
    let result = run(
        vec![
            OpCode::Mov(r(0), r(1)),  // r1 is uninitialized
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
            OpCode::Jz(r(0), 1),  // r0 is uninitialized
            OpCode::Ret(r(1)),
        ],
        vec![],
    );
    assert!(result.is_err());
}
