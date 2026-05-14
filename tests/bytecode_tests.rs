use ect::bytecode::{Chunk, OpCode, Register};
use ect::vm::Value;

#[test]
fn test_register_roundtrip() {
    let reg = Register(42);
    assert_eq!(reg.to_usize(), 42);
}

#[test]
fn test_register_zero() {
    let reg = Register(0);
    assert_eq!(reg.to_usize(), 0);
}

#[test]
fn test_register_max() {
    let reg = Register(255);
    assert_eq!(reg.to_usize(), 255);
}

#[test]
fn test_chunk_construction() {
    let chunk = Chunk {
        code: vec![
            OpCode::PushConst(Register(0), 0),
            OpCode::Ret(Register(0)),
        ],
        constants: vec![Value::Int(42)],
    };
    assert_eq!(chunk.code.len(), 2);
    assert_eq!(chunk.constants.len(), 1);
}

#[test]
fn test_chunk_empty() {
    let chunk = Chunk {
        code: vec![],
        constants: vec![],
    };
    assert!(chunk.code.is_empty());
    assert!(chunk.constants.is_empty());
}

#[test]
fn test_opcode_variants() {
    let ops = vec![
        OpCode::PushConst(Register(0), 0),
        OpCode::Mov(Register(0), Register(1)),
        OpCode::Add(Register(2), Register(0), Register(1)),
        OpCode::Sub(Register(2), Register(0), Register(1)),
        OpCode::Mul(Register(2), Register(0), Register(1)),
        OpCode::Div(Register(2), Register(0), Register(1)),
        OpCode::Mod(Register(2), Register(0), Register(1)),
        OpCode::Eq(Register(2), Register(0), Register(1)),
        OpCode::Neq(Register(2), Register(0), Register(1)),
        OpCode::Lt(Register(2), Register(0), Register(1)),
        OpCode::Gt(Register(2), Register(0), Register(1)),
        OpCode::Lte(Register(2), Register(0), Register(1)),
        OpCode::Gte(Register(2), Register(0), Register(1)),
        OpCode::Jz(Register(0), 5),
        OpCode::Jnz(Register(0), 5),
        OpCode::Jmp(5),
        OpCode::Ret(Register(0)),
    ];
    assert_eq!(ops.len(), 17);
}
