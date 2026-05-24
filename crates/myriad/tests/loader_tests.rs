use polka::{BytecodeChunk, Chunk, Module, OpCode, Register};
use myriad::loader::load;

fn bc(reg_count: usize, param_count: usize) -> Chunk {
    Chunk::Bytecode(BytecodeChunk {
        code: vec![OpCode::Ret(Register(0))],
        reg_count, param_count,
        ..BytecodeChunk::default()
    })
}

#[test]
fn load_accepts_well_formed_module() {
    let m = Module {
        functions: vec![bc(1, 0)],
        entry: 0,
        flags: 0,
        exports: vec![],
    };
    assert!(load(m).is_ok());
}

#[test]
fn load_rejects_reg_count_over_frame_budget() {
    let over = polka::FRAME_REGS + 1;
    let m = Module {
        functions: vec![bc(over, 0)],
        entry: 0,
        flags: 0,
        exports: vec![],
    };
    let err = match load(m) { Ok(_) => panic!("expected error"), Err(e) => e };
    assert!(err.contains(&format!("reg_count {}", over)), "got: {}", err);
    assert!(err.contains("frame budget"));
}

#[test]
fn load_rejects_param_count_over_reg_count() {
    let m = Module {
        functions: vec![bc(2, 5)],
        entry: 0,
        flags: 0,
        exports: vec![],
    };
    let err = match load(m) { Ok(_) => panic!("expected error"), Err(e) => e };
    assert!(err.contains("param_count 5"));
    assert!(err.contains("reg_count 2"));
}

#[test]
fn load_rejects_entry_out_of_range() {
    let m = Module {
        functions: vec![bc(1, 0)],
        entry: 99,
        flags: 0,
        exports: vec![],
    };
    let err = match load(m) { Ok(_) => panic!("expected error"), Err(e) => e };
    assert!(err.contains("entry 99"));
}

#[test]
fn load_ignores_native_chunk_reg_count() {
    use polka::NativeChunk;
    let m = Module {
        functions: vec![Chunk::Native(NativeChunk { name: "host".into(), param_count: 200 })],
        entry: 0,
        flags: 0,
        exports: vec![],
    };
    assert!(load(m).is_ok());
}
