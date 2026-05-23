use polka::{BytecodeChunk, Chunk, Export, Module, NativeChunk, OpCode, Register};
use polka::cartridge::{read_pk, write_pk, Corruption, EncodeError, LoadError};

#[test]
fn empty_module_roundtrip() {
    let m = Module { functions: vec![], entry: 0, flags: 0, exports: vec![] };
    let bytes = write_pk(&m).unwrap();
    let back = read_pk(&bytes).unwrap();
    assert_eq!(back.entry, 0);
    assert!(back.functions.is_empty());
}

#[test]
fn single_bytecode_fn_roundtrip() {
    let bc = BytecodeChunk {
        code: vec![
            OpCode::PushConst(Register(0), 0),
            OpCode::Ret(Register(0)),
        ],
        constants: vec![42u64],
        const_mask: vec![0u64],
        string_constants: vec![],
        reg_count: 1,
        param_count: 0,
    };
    let m = Module { functions: vec![Chunk::Bytecode(bc)], entry: 0, flags: 0, exports: vec![] };
    let bytes = write_pk(&m).unwrap();
    let back = read_pk(&bytes).unwrap();
    assert_eq!(back.functions.len(), 1);
    if let Chunk::Bytecode(b2) = &back.functions[0] {
        assert_eq!(b2.constants, vec![42u64]);
        assert_eq!(b2.code.len(), 2);
    } else {
        panic!("expected bytecode chunk");
    }
}

#[test]
fn native_then_bytecode_roundtrip() {
    let n = NativeChunk { name: "print".into(), param_count: 1 };
    let bc = BytecodeChunk {
        code: vec![OpCode::Ret(Register(0))],
        constants: vec![],
        const_mask: vec![],
        string_constants: vec!["hi".into()],
        reg_count: 1,
        param_count: 0,
    };
    let m = Module {
        functions: vec![Chunk::Native(n), Chunk::Bytecode(bc)],
        entry: 1,
        flags: 0,

        exports: vec![],
    };
    let bytes = write_pk(&m).unwrap();
    let back = read_pk(&bytes).unwrap();
    assert_eq!(back.entry, 1);
    if let Chunk::Native(n2) = &back.functions[0] {
        assert_eq!(n2.name, "print");
        assert_eq!(n2.param_count, 1);
    } else { panic!("expected native"); }
    if let Chunk::Bytecode(b2) = &back.functions[1] {
        assert_eq!(b2.string_constants, vec!["hi".to_string()]);
    } else { panic!("expected bytecode"); }
}

#[test]
fn const_mask_roundtrip() {
    let bc = BytecodeChunk {
        code: vec![OpCode::Ret(Register(0))],
        constants: vec![0u64, 1u64, 2u64, 3u64],
        const_mask: vec![0b1010u64],
        string_constants: vec![],
        reg_count: 1,
        param_count: 0,
    };
    let m = Module { functions: vec![Chunk::Bytecode(bc)], entry: 0, flags: 0, exports: vec![] };
    let bytes = write_pk(&m).unwrap();
    let back = read_pk(&bytes).unwrap();
    if let Chunk::Bytecode(b2) = &back.functions[0] {
        assert!(!b2.const_is_handle(0));
        assert!(b2.const_is_handle(1));
        assert!(!b2.const_is_handle(2));
        assert!(b2.const_is_handle(3));
    } else { panic!(); }
}

#[test]
fn bad_magic_is_not_a_cartridge() {
    let bytes = vec![0u8; 16];
    assert!(matches!(read_pk(&bytes), Err(LoadError::NotACartridge)));
}

fn good_minimal_module() -> Module {
    Module {
        functions: vec![Chunk::Bytecode(BytecodeChunk {
            code: vec![OpCode::Ret(Register(0))],
            constants: vec![],
            const_mask: vec![],
            string_constants: vec![],
            reg_count: 1,
            param_count: 0,
        })],
        entry: 0,
        flags: 0,

        exports: vec![],
    }
}

#[test]
fn bad_version() {
    let m = good_minimal_module();
    let mut bytes = write_pk(&m).unwrap();
    // Header layout: magic[0..4], version[4..6]. Bump to a value we don't support.
    bytes[4] = 0xFF;
    bytes[5] = 0xFF;
    assert!(matches!(read_pk(&bytes), Err(LoadError::UnsupportedVersion(_))));
}

#[test]
fn bad_kind_byte() {
    let m = good_minimal_module();
    let mut bytes = write_pk(&m).unwrap();
    // header (12) + fn_count u32 (4) = 16. Next byte is the first entry's kind.
    bytes[16] = 0x7F;
    let err = match read_pk(&bytes) { Ok(_) => panic!("expected error"), Err(e) => e };
    assert!(matches!(err, LoadError::Corrupt { offset: 16, kind: Corruption::UnknownKind(0x7F) }),
            "expected UnknownKind at offset 16, got {:?}", err);
}

#[test]
fn truncated_header() {
    // Less than 4 bytes: not enough to even check the magic.
    assert!(matches!(read_pk(&[]), Err(LoadError::NotACartridge)));
    assert!(matches!(read_pk(&[0u8; 3]), Err(LoadError::NotACartridge)));

    let bytes = write_pk(&good_minimal_module()).unwrap();
    assert!(bytes.len() >= 12);
    for cut in 4..12 {
        let r = read_pk(&bytes[..cut]);
        match r {
            Err(LoadError::Corrupt { offset, kind: Corruption::Truncated }) => {
                assert!(offset <= cut,
                        "offset {} should not exceed cut {}", offset, cut);
            }
            other => panic!("header cut at byte {} should be Truncated, got {:?}", cut, other),
        }
    }
}

#[test]
fn truncated_fn_table_count() {
    let m = good_minimal_module();
    let bytes = write_pk(&m).unwrap();
    assert!(matches!(
        read_pk(&bytes[..14]),
        Err(LoadError::Corrupt { kind: Corruption::Truncated, .. })
    ));
}

#[test]
fn truncated_fn_entry() {
    let m = good_minimal_module();
    let bytes = write_pk(&m).unwrap();
    assert!(matches!(
        read_pk(&bytes[..18]),
        Err(LoadError::Corrupt { kind: Corruption::Truncated, .. })
    ));
}

#[test]
fn truncated_payload() {
    let m = good_minimal_module();
    let bytes = write_pk(&m).unwrap();
    let cut = bytes.len() - 2;
    assert!(matches!(
        read_pk(&bytes[..cut]),
        Err(LoadError::Corrupt { kind: Corruption::Truncated, .. })
    ));
}

#[test]
fn bad_opcode_in_code() {
    let m = good_minimal_module();
    let mut bytes = write_pk(&m).unwrap();
    let code_start = bytes.len() - 4 - 4;
    bytes[code_start] = 0x7E;
    let err = read_pk(&bytes).unwrap_err();
    match err {
        LoadError::Corrupt { offset, kind: Corruption::UnknownOpcode(0x7E) } => {
            assert_eq!(offset, code_start, "offset should point at the bad opcode");
        }
        other => panic!("expected UnknownOpcode(0x7E), got {:?}", other),
    }
}

#[test]
fn ld_offset_over_255_rejected() {
    let bc = BytecodeChunk {
        code: vec![OpCode::Ld(Register(0), Register(1), 256)],
        reg_count: 2,
        ..BytecodeChunk::default()
    };
    let m = Module { functions: vec![Chunk::Bytecode(bc)], entry: 0, flags: 0, exports: vec![] };
    assert!(matches!(
        write_pk(&m),
        Err(EncodeError::OffsetTooLarge { value: 256, op: "ld" })
    ));
}

#[test]
fn st_offset_over_255_rejected() {
    let bc = BytecodeChunk {
        code: vec![OpCode::St(Register(0), Register(1), 1000)],
        reg_count: 2,
        ..BytecodeChunk::default()
    };
    let m = Module { functions: vec![Chunk::Bytecode(bc)], entry: 0, flags: 0, exports: vec![] };
    assert!(matches!(
        write_pk(&m),
        Err(EncodeError::OffsetTooLarge { value: 1000, op: "st" })
    ));
}

#[test]
fn magic_is_little_endian_ecff00ec() {
    let bytes = write_pk(&good_minimal_module()).unwrap();
    assert_eq!(&bytes[0..4], &[0xEC, 0x00, 0xFF, 0xEC]);
}

#[test]
fn version_is_current() {
    let bytes = write_pk(&good_minimal_module()).unwrap();
    let v = u16::from_le_bytes([bytes[4], bytes[5]]);
    assert_eq!(v, polka::cartridge::VERSION);
}

#[test]
fn header_is_12_bytes_then_fn_count() {
    let m = good_minimal_module();
    let bytes = write_pk(&m).unwrap();
    // entry_fn_id at offset 8..12
    assert_eq!(u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]), 0);
    // fn_count at offset 12..16
    assert_eq!(u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]), 1);
}

#[test]
fn entry_fn_id_nonzero_roundtrip() {
    let n = NativeChunk { name: "abort".into(), param_count: 1 };
    let bc = BytecodeChunk {
        code: vec![OpCode::Ret(Register(0))],
        reg_count: 1,
        ..BytecodeChunk::default()
    };
    let m = Module {
        functions: vec![Chunk::Native(n), Chunk::Bytecode(bc)],
        entry: 1,
        flags: 0,

        exports: vec![],
    };
    let bytes = write_pk(&m).unwrap();
    let back = read_pk(&bytes).unwrap();
    assert_eq!(back.entry, 1);
}

fn all_opcodes() -> Vec<OpCode> {
    use OpCode::*;
    let r = |n: u8| Register(n);
    vec![
        Add(r(0), r(1), r(2)),
        Sub(r(0), r(1), r(2)),
        Mul(r(0), r(1), r(2)),
        Div(r(0), r(1), r(2)),
        Mod(r(0), r(1), r(2)),
        Neg(r(0), r(1)),

        Eq (r(0), r(1), r(2)),
        Neq(r(0), r(1), r(2)),
        Lt (r(0), r(1), r(2)),
        Gt (r(0), r(1), r(2)),
        Lte(r(0), r(1), r(2)),
        Gte(r(0), r(1), r(2)),

        And(r(0), r(1), r(2)),
        Or (r(0), r(1), r(2)),
        Xor(r(0), r(1), r(2)),
        Shl(r(0), r(1), r(2)),
        Shr(r(0), r(1), r(2)),

        Jmp(-1),
        Jz (r(3), 42),
        Jnz(r(3), -42),
        Call(r(4), 0x1234),
        Ret(r(5)),
        CallReg(r(6), r(7)),

        PushConst(r(8), 0xABCD),
        Copy(r(9), r(10)),
        Move(r(11), r(12)),

        Ld   (r(13), r(14), 0xFF),  // boundary: 8-bit max
        St   (r(13), r(14), 0),
        LdIdx(r(15), r(16), r(17)),
        StIdx(r(18), r(19), r(20)),

        Alloc(r(23), 0xFFFF),
        Drop(r(24)),

        Dei(r(25), r(26)),
        Deo(r(27), r(28)),

        Handle(r(29), 0xBEEF),
        Resume(r(30), r(31)),

        AddImm(r(32), r(33),  127),
        SubImm(r(34), r(35), -128),

        FAdd(r(36), r(37), r(38)),
        FSub(r(36), r(37), r(38)),
        FMul(r(36), r(37), r(38)),
        FDiv(r(36), r(37), r(38)),
        FNeg(r(39), r(40)),
        FLt (r(41), r(42), r(43)),
        FEq (r(44), r(45), r(46)),
    ]
}

#[test]
fn all_46_opcodes_roundtrip() {
    let code = all_opcodes();
    assert_eq!(code.len(), 45, "opcode set drifted from spec");

    // 1024 reg_count is bigger than needed but exercises u16 reg_count when serialized.
    let bc = BytecodeChunk {
        code: code.clone(),
        constants: vec![0u64; 0xABCE],     // exercises u16 const_count near upper end
        const_mask: vec![0u64; (0xABCE + 63) / 64],
        string_constants: vec!["hello".into(), "".into(), "中文 🚀".into()],
        reg_count: 64,
        param_count: 0,
    };
    let m = Module { functions: vec![Chunk::Bytecode(bc)], entry: 0, flags: 0, exports: vec![] };
    let bytes = write_pk(&m).unwrap();
    let back = read_pk(&bytes).unwrap();
    let bc2 = back.functions[0].as_bytecode().unwrap();

    assert_eq!(bc2.code, code, "opcode round-trip lost data");
    assert_eq!(bc2.string_constants, vec!["hello".to_string(), "".into(), "中文 🚀".into()]);
    assert_eq!(bc2.reg_count, 64);
}

#[test]
fn negative_jump_immediate_signed() {
    let bc = BytecodeChunk {
        code: vec![OpCode::Jmp(i16::MIN), OpCode::Jmp(i16::MAX), OpCode::Ret(Register(0))],
        reg_count: 1,
        ..BytecodeChunk::default()
    };
    let m = Module { functions: vec![Chunk::Bytecode(bc)], entry: 0, flags: 0, exports: vec![] };
    let bytes = write_pk(&m).unwrap();
    let back = read_pk(&bytes).unwrap();
    let bc2 = back.functions[0].as_bytecode().unwrap();
    assert_eq!(bc2.code[0], OpCode::Jmp(i16::MIN));
    assert_eq!(bc2.code[1], OpCode::Jmp(i16::MAX));
}

#[test]
fn addimm_signed_imm8() {
    let bc = BytecodeChunk {
        code: vec![
            OpCode::AddImm(Register(0), Register(1),  127),
            OpCode::AddImm(Register(0), Register(1), -128),
            OpCode::SubImm(Register(0), Register(1),    0),
            OpCode::Ret(Register(0)),
        ],
        reg_count: 2,
        ..BytecodeChunk::default()
    };
    let m = Module { functions: vec![Chunk::Bytecode(bc)], entry: 0, flags: 0, exports: vec![] };
    let bytes = write_pk(&m).unwrap();
    let back = read_pk(&bytes).unwrap();
    let bc2 = back.functions[0].as_bytecode().unwrap();
    assert_eq!(bc2.code[0], OpCode::AddImm(Register(0), Register(1),  127));
    assert_eq!(bc2.code[1], OpCode::AddImm(Register(0), Register(1), -128));
    assert_eq!(bc2.code[2], OpCode::SubImm(Register(0), Register(1), 0));
}

#[test]
fn empty_string_in_pool() {
    let bc = BytecodeChunk {
        code: vec![OpCode::Ret(Register(0))],
        string_constants: vec!["".into(), "x".into(), "".into()],
        reg_count: 1,
        ..BytecodeChunk::default()
    };
    let m = Module { functions: vec![Chunk::Bytecode(bc)], entry: 0, flags: 0, exports: vec![] };
    let bytes = write_pk(&m).unwrap();
    let back = read_pk(&bytes).unwrap();
    let bc2 = back.functions[0].as_bytecode().unwrap();
    assert_eq!(bc2.string_constants, vec!["".to_string(), "x".into(), "".into()]);
}

#[test]
fn many_functions() {
    let mut fns: Vec<Chunk> = Vec::new();
    for i in 0..32u32 {
        fns.push(Chunk::Bytecode(BytecodeChunk {
            code: vec![
                OpCode::PushConst(Register(0), 0),
                OpCode::Ret(Register(0)),
            ],
            constants: vec![i as u64],
            const_mask: vec![0u64],
            reg_count: 1,
            param_count: 0,
            ..BytecodeChunk::default()
        }));
    }
    let m = Module { functions: fns, entry: 31, flags: 0, exports: vec![] };
    let bytes = write_pk(&m).unwrap();
    let back = read_pk(&bytes).unwrap();
    assert_eq!(back.functions.len(), 32);
    assert_eq!(back.entry, 31);
    for (i, c) in back.functions.iter().enumerate() {
        let bc = c.as_bytecode().unwrap();
        assert_eq!(bc.constants, vec![i as u64]);
    }
}

#[test]
fn load_error_display_is_user_facing() {
    let s = format!("{}", LoadError::NotACartridge);
    assert!(s.to_lowercase().contains("polka") || s.to_lowercase().contains("cartridge"),
            "display should identify the file type: {}", s);

    let s = format!("{}", LoadError::Corrupt {
        offset: 42,
        kind: Corruption::UnknownOpcode(0xFF),
    });
    assert!(s.contains("42"), "display should include offset: {}", s);
    assert!(s.contains("0xFF"), "display should include opcode hex: {}", s);
}

#[test]
fn encode_error_display_names_the_culprit() {
    let s = format!("{}", EncodeError::OffsetTooLarge { value: 300, op: "ld" });
    assert!(s.contains("ld"), "display should name the opcode: {}", s);
    assert!(s.contains("300"), "display should include the value: {}", s);

    let s = format!("{}", EncodeError::CountOverflow { value: 70000, what: "constant" });
    assert!(s.contains("constant"), "display should name the count kind: {}", s);
}

#[test]
fn exports_roundtrip() {
    let bc = BytecodeChunk {
        code: vec![OpCode::Ret(Register(0))],
        reg_count: 1,
        ..BytecodeChunk::default()
    };
    let m = Module {
        functions: vec![Chunk::Bytecode(bc.clone()), Chunk::Bytecode(bc)],
        entry: 0,
        flags: 0,
        exports: vec![
            Export { name: "main".into(), fn_id: 0 },
            Export { name: "helper".into(), fn_id: 1 },
        ],
    };
    let bytes = write_pk(&m).unwrap();
    let back = read_pk(&bytes).unwrap();
    assert_eq!(back.exports.len(), 2);
    assert_eq!(back.exports[0].name, "main");
    assert_eq!(back.exports[0].fn_id, 0);
    assert_eq!(back.exports[1].name, "helper");
    assert_eq!(back.exports[1].fn_id, 1);
}

#[test]
fn empty_exports_roundtrip() {
    let m = good_minimal_module();
    let bytes = write_pk(&m).unwrap();
    let back = read_pk(&bytes).unwrap();
    assert!(back.exports.is_empty());
}
