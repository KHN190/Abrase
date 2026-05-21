use polka::{
    mask_bit_set, mask_set,
    BytecodeChunk, Chunk, Module, NativeChunk, OpCode, Register,
    DISPATCH_ID, DISPATCH_NO_MATCH, DISPATCH_PORT_ENV, DISPATCH_PORT_LOOKUP,
    DISPATCH_PORT_POP_HANDLER, DISPATCH_PORT_RETURN_ENV, DISPATCH_PORT_RETURN_FN,
    FRAME_MASK_WORDS, FRAME_REGS,
    REGION_ID, REGION_PORT_FORGET, REGION_PORT_POP, REGION_PORT_PUSH,
};

// ----- constants -----

#[test]
fn frame_constants() {
    assert_eq!(FRAME_REGS, 64);
    assert_eq!(FRAME_MASK_WORDS, 1);
}

#[test]
fn dispatch_constants() {
    assert_eq!(DISPATCH_ID, 0xE0);
    assert_eq!(DISPATCH_PORT_LOOKUP, 0x00);
    assert_eq!(DISPATCH_PORT_POP_HANDLER, 0x01);
    assert_eq!(DISPATCH_PORT_ENV, 0x02);
    assert_eq!(DISPATCH_PORT_RETURN_FN, 0x03);
    assert_eq!(DISPATCH_PORT_RETURN_ENV, 0x04);
    assert_eq!(DISPATCH_NO_MATCH, 0xFFFF);
}

#[test]
fn region_constants() {
    assert_eq!(REGION_ID, 0xE1);
    assert_eq!(REGION_PORT_PUSH, 0x00);
    assert_eq!(REGION_PORT_POP, 0x01);
    assert_eq!(REGION_PORT_FORGET, 0x02);
}

// ----- Register -----

#[test]
fn register_to_usize() {
    assert_eq!(Register(0).to_usize(), 0);
    assert_eq!(Register(63).to_usize(), 63);
    assert_eq!(Register(255).to_usize(), 255);
}

#[test]
fn register_eq_hash() {
    use std::collections::HashSet;
    let mut s = HashSet::new();
    s.insert(Register(1));
    s.insert(Register(1));
    s.insert(Register(2));
    assert_eq!(s.len(), 2);
    assert_eq!(Register(7), Register(7));
    assert_ne!(Register(7), Register(8));
}

// ----- BytecodeChunk -----

#[test]
fn bytecode_chunk_default_is_empty() {
    let bc = BytecodeChunk::default();
    assert!(bc.code.is_empty());
    assert!(bc.constants.is_empty());
    assert!(bc.const_mask.is_empty());
    assert!(bc.string_constants.is_empty());
    assert_eq!(bc.reg_count, 0);
    assert_eq!(bc.param_count, 0);
}

#[test]
fn const_is_handle_basic() {
    let bc = BytecodeChunk {
        const_mask: vec![0b0000_1010u64], // bits 1 and 3
        ..BytecodeChunk::default()
    };
    assert!(!bc.const_is_handle(0));
    assert!(bc.const_is_handle(1));
    assert!(!bc.const_is_handle(2));
    assert!(bc.const_is_handle(3));
    assert!(!bc.const_is_handle(4));
}

#[test]
fn const_is_handle_crosses_word_boundary() {
    let mut const_mask = vec![0u64; 3];
    const_mask[0] |= 1u64 << 63; // idx 63 in word 0
    const_mask[1] |= 1u64 << 0;  // idx 64 in word 1
    const_mask[2] |= 1u64 << 5;  // idx 64*2 + 5 = 133 in word 2
    let bc = BytecodeChunk { const_mask, ..BytecodeChunk::default() };
    assert!(bc.const_is_handle(63));
    assert!(bc.const_is_handle(64));
    assert!(!bc.const_is_handle(65));
    assert!(bc.const_is_handle(133));
}

#[test]
fn const_is_handle_out_of_bounds_returns_false() {
    let bc = BytecodeChunk::default(); // empty mask
    assert!(!bc.const_is_handle(0));
    assert!(!bc.const_is_handle(1000));
    assert!(!bc.const_is_handle(u16::MAX));
}

// ----- Chunk -----

#[test]
fn chunk_param_count_bytecode() {
    let c = Chunk::Bytecode(BytecodeChunk { param_count: 3, ..BytecodeChunk::default() });
    assert_eq!(c.param_count(), 3);
}

#[test]
fn chunk_param_count_native() {
    let c = Chunk::Native(NativeChunk { name: "x".into(), param_count: 2 });
    assert_eq!(c.param_count(), 2);
}

#[test]
fn chunk_as_bytecode_some_for_bytecode() {
    let c = Chunk::Bytecode(BytecodeChunk { reg_count: 5, ..BytecodeChunk::default() });
    let bc = c.as_bytecode().expect("expected Some");
    assert_eq!(bc.reg_count, 5);
}

#[test]
fn chunk_as_bytecode_none_for_native() {
    let c = Chunk::Native(NativeChunk { name: "n".into(), param_count: 0 });
    assert!(c.as_bytecode().is_none());
}

// ----- Module is a plain struct, just sanity-construct -----

#[test]
fn module_construct() {
    let m = Module { functions: vec![], entry: 0 };
    assert_eq!(m.entry, 0);
    assert!(m.functions.is_empty());
}

// ----- mask helpers -----

#[test]
fn mask_set_and_check_within_word() {
    let mut m = vec![0u64; 1];
    assert!(!mask_bit_set(&m, 5));
    mask_set(&mut m, 5, true);
    assert!(mask_bit_set(&m, 5));
    mask_set(&mut m, 5, false);
    assert!(!mask_bit_set(&m, 5));
}

#[test]
fn mask_set_across_words() {
    let mut m = vec![0u64; 3];
    for &idx in &[0usize, 63, 64, 127, 128, 191] {
        mask_set(&mut m, idx, true);
    }
    for &idx in &[0usize, 63, 64, 127, 128, 191] {
        assert!(mask_bit_set(&m, idx), "bit {} should be set", idx);
    }
    for &idx in &[1usize, 62, 65, 126, 129, 190] {
        assert!(!mask_bit_set(&m, idx), "bit {} should be unset", idx);
    }
}

#[test]
fn mask_bit_set_out_of_bounds_returns_false() {
    let m = vec![0u64; 1];
    assert!(!mask_bit_set(&m, 64));
    assert!(!mask_bit_set(&m, 1_000_000));
}

#[test]
fn mask_set_out_of_bounds_is_noop() {
    let mut m = vec![0u64; 1];
    mask_set(&mut m, 64, true);   // beyond capacity, should not panic
    mask_set(&mut m, 1_000_000, true);
    assert_eq!(m[0], 0);
}

#[test]
fn mask_set_idempotent() {
    let mut m = vec![0u64; 1];
    mask_set(&mut m, 3, true);
    mask_set(&mut m, 3, true);
    mask_set(&mut m, 3, true);
    assert!(mask_bit_set(&m, 3));
    // turning off once clears
    mask_set(&mut m, 3, false);
    assert!(!mask_bit_set(&m, 3));
}

// ----- OpCode equality (we added PartialEq) -----

#[test]
fn opcode_equality() {
    assert_eq!(OpCode::Add(Register(1), Register(2), Register(3)),
               OpCode::Add(Register(1), Register(2), Register(3)));
    assert_ne!(OpCode::Add(Register(1), Register(2), Register(3)),
               OpCode::Sub(Register(1), Register(2), Register(3)));
    assert_ne!(OpCode::Jmp(5), OpCode::Jmp(6));
}
