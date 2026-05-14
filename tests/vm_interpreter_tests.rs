// Tests for vm/interpreter.rs: VM dispatch loop, register operations, arithmetic.
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
