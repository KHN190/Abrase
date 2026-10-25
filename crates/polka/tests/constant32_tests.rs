use polka::Constant;

#[test]
fn wire_constants_keep_all_eight_bytes_without_wide_arithmetic() {
    assert_eq!(core::mem::size_of::<polka::Constant32>(), 8);
    for bytes in [[0; 8], [255; 8], [1,2,3,4,5,6,7,8]] {
        assert_eq!(Constant::from_le_bytes(bytes).to_le_bytes(), bytes);
        assert_eq!(polka::Constant32::from_le_bytes(bytes).to_le_bytes(), bytes);
    }
}

#[test]
fn scalar_conversion_accepts_only_low_word_or_sign_extension() {
    for (lo, hi, valid) in [(0u32,0u32,true), (u32::MAX,0,true),
        (0x80000000,u32::MAX,true), (0x7fffffff,u32::MAX,false), (0,1,false)] {
        let mut bytes = [0;8];
        bytes[..4].copy_from_slice(&lo.to_le_bytes());
        bytes[4..].copy_from_slice(&hi.to_le_bytes());
        assert_eq!(polka::constant_word32(Constant::from_le_bytes(bytes)).ok(), valid.then_some(lo));
    }
}

#[test]
fn bytes_tag_is_wire_bit_63_and_indices_reject_extra_high_bits() {
    let bytes = Constant::from_le_bytes([7,0,0,0,0,0,0,128]);
    assert!(polka::constant_is_bytes(bytes));
    assert_eq!(polka::constant_pool_index(bytes), Ok(7));
    assert!(polka::constant_word32(bytes).is_err());
    let bad = Constant::from_le_bytes([7,0,0,0,1,0,0,128]);
    assert!(polka::constant_pool_index(bad).is_err());
}

fn module() -> polka::Module {
    let mut chunk = polka::BytecodeChunk::default();
    chunk.constants = vec![Constant::from_le_bytes([0;8]);65];
    chunk.const_mask = vec![0;65usize.div_ceil(polka::numeric::WORD_BITS as usize)];
    chunk.string_constants = vec!["text".into()];
    chunk.bytes_constants = vec![vec![1,2]];
    for index in [31,32,63,64] { polka::mask_set(&mut chunk.const_mask,index,true); }
    chunk.constants[0] = Constant::from_le_bytes([255;8]);
    chunk.constants[64] = Constant::from_le_bytes([0,0,0,0,0,0,0,128]);
    polka::Module { functions: vec![polka::Chunk::Bytecode(chunk)], entry:0,
        flags:polka::CART_FLAG_INT32_SAFE,exports:vec![] }
}

#[test]
fn cartridge_preserves_wire_bytes_and_masks_across_32_bit_boundaries() {
    let bytes=polka::cartridge::write_pk(&module()).unwrap();
    let loaded=polka::cartridge::read_pk(&bytes).unwrap();
    assert_eq!(polka::cartridge::write_pk(&loaded).unwrap(),bytes);
    let polka::Chunk::Bytecode(chunk)=&loaded.functions[0] else {panic!()};
    assert_eq!(core::mem::size_of_val(&chunk.const_mask[0]),polka::numeric::WORD_BYTES);
    for index in 0..65 { assert_eq!(chunk.const_is_handle(index),[31,32,63,64].contains(&index)); }
}

#[cfg(all(target_os = "none", target_pointer_width = "32"))]
#[test]
fn cartridge_rejects_wide_profile_and_unrepresentable_constants() {
    let mut bytes=polka::cartridge::write_pk(&module()).unwrap();
    bytes[6]=0;
    assert!(polka::cartridge::read_pk(&bytes).is_err());
    let mut invalid=module();
    let polka::Chunk::Bytecode(chunk)=&mut invalid.functions[0] else {panic!()};
    chunk.constants[0]=Constant::from_le_bytes([0,0,0,0,1,0,0,0]);
    let bytes=polka::cartridge::write_pk(&invalid).unwrap();
    assert!(polka::cartridge::read_pk(&bytes).is_err());
}
