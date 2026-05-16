use abrase::bytecode::{BytecodeChunk, Chunk, OpCode, Register};
use abrase::vm::Value;

fn bc(b: BytecodeChunk) -> Chunk { Chunk::Bytecode(b) }
fn as_bc(c: &Chunk) -> &BytecodeChunk { c.as_bytecode().expect("expected bytecode chunk") }

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
    let chunk = bc(BytecodeChunk {
        code: vec![
            OpCode::PushConst(Register(0), 0),
            OpCode::Ret(Register(0)),
        ],
        constants: vec![Value::from_int(42)],
        reg_count: 1,
        param_count: 0,
        string_constants: Vec::new(),
    });
    let b = as_bc(&chunk);
    assert_eq!(b.code.len(), 2);
    assert_eq!(b.constants.len(), 1);
}

#[test]
fn test_chunk_empty() {
    let chunk = bc(BytecodeChunk {
        code: vec![],
        constants: vec![],
        reg_count: 0,
        param_count: 0,
        string_constants: Vec::new(),
    });
    let b = as_bc(&chunk);
    assert!(b.code.is_empty());
    assert!(b.constants.is_empty());
}

#[test]
fn test_opcode_variants() {
    let ops = vec![
        OpCode::PushConst(Register(0), 0),
        OpCode::Copy(Register(0), Register(1)),
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
        OpCode::Call(Register(0), 1),
    ];
    assert_eq!(ops.len(), 18);
}

#[test]
fn test_call_opcode() {
    let call_op = OpCode::Call(Register(0), 42);
    match call_op {
        OpCode::Call(dest, func_id) => {
            assert_eq!(dest.to_usize(), 0);
            assert_eq!(func_id, 42);
        }
        _ => panic!("Not a Call opcode"),
    }
}

#[test]
fn test_chunk_reg_count() {
    let chunk = bc(BytecodeChunk {
        code: vec![OpCode::PushConst(Register(0), 0)],
        constants: vec![Value::from_int(10)],
        reg_count: 42,
        param_count: 0,
        string_constants: Vec::new(),
    });
    assert_eq!(as_bc(&chunk).reg_count, 42);
}

#[test]
fn test_frame_dest_reg() {
    let frame = abrase::vm::frame::Frame {
        func_id: 1,
        ip: 10,
        base_reg: 64,
        dest_reg: 5,
    };
    assert_eq!(frame.func_id, 1);
    assert_eq!(frame.ip, 10);
    assert_eq!(frame.base_reg, 64);
    assert_eq!(frame.dest_reg, 5);
}

#[test]
fn test_alloc_opcode() {
    let alloc_op = OpCode::Alloc(Register(0), 16);
    match alloc_op {
        OpCode::Alloc(dest, size) => {
            assert_eq!(dest.to_usize(), 0);
            assert_eq!(size, 16);
        }
        _ => panic!("Not an Alloc opcode"),
    }
}

#[test]
fn test_ld_opcode() {
    let ld_op = OpCode::Ld(Register(0), Register(1), 8);
    match ld_op {
        OpCode::Ld(dest, base, offset) => {
            assert_eq!(dest.to_usize(), 0);
            assert_eq!(base.to_usize(), 1);
            assert_eq!(offset, 8);
        }
        _ => panic!("Not an Ld opcode"),
    }
}

#[test]
fn test_st_opcode() {
    let st_op = OpCode::St(Register(2), Register(1), 4);
    match st_op {
        OpCode::St(src, base, offset) => {
            assert_eq!(src.to_usize(), 2);
            assert_eq!(base.to_usize(), 1);
            assert_eq!(offset, 4);
        }
        _ => panic!("Not an St opcode"),
    }
}

#[test]
fn test_ldidx_opcode() {
    let ldidx_op = OpCode::LdIdx(Register(0), Register(1), Register(2));
    match ldidx_op {
        OpCode::LdIdx(dest, base, idx) => {
            assert_eq!(dest.to_usize(), 0);
            assert_eq!(base.to_usize(), 1);
            assert_eq!(idx.to_usize(), 2);
        }
        _ => panic!("Not an LdIdx opcode"),
    }
}

#[test]
fn test_stidx_opcode() {
    let stidx_op = OpCode::StIdx(Register(3), Register(1), Register(2));
    match stidx_op {
        OpCode::StIdx(src, base, idx) => {
            assert_eq!(src.to_usize(), 3);
            assert_eq!(base.to_usize(), 1);
            assert_eq!(idx.to_usize(), 2);
        }
        _ => panic!("Not an StIdx opcode"),
    }
}

#[test]
fn test_memory_opcodes_in_chunk() {
    let chunk = bc(BytecodeChunk {
        code: vec![
            OpCode::PushConst(Register(0), 0),
            OpCode::Alloc(Register(1), 16),
            OpCode::PushConst(Register(2), 1),
            OpCode::St(Register(2), Register(1), 0),
            OpCode::Ld(Register(3), Register(1), 0),
            OpCode::Ret(Register(3)),
        ],
        constants: vec![Value::from_int(42), Value::from_int(100)],
        reg_count: 4,
        param_count: 0,
        string_constants: Vec::new(),
    });
    let b = as_bc(&chunk);
    assert_eq!(b.code.len(), 6);

    match &b.code[1] {
        OpCode::Alloc(_, size) => assert_eq!(*size, 16),
        _ => panic!("Expected Alloc opcode"),
    }
    match &b.code[3] {
        OpCode::St(_, _, offset) => assert_eq!(*offset, 0),
        _ => panic!("Expected St opcode"),
    }
}

#[test]
fn test_handle_opcode() {
    let h = OpCode::Handle(Register(0), 5);
    match h {
        OpCode::Handle(dest, fn_id) => {
            assert_eq!(dest.to_usize(), 0);
            assert_eq!(fn_id, 5);
        }
        _ => panic!("Not a Handle opcode"),
    }
}

#[test]
fn test_resume_opcode() {
    let r = OpCode::Resume(Register(1));
    match r {
        OpCode::Resume(reg) => assert_eq!(reg.to_usize(), 1),
        _ => panic!("Not a Resume opcode"),
    }
}
