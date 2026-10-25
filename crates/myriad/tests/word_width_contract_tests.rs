use myriad::VirtualMachine;
use polka::{BytecodeChunk, Chunk, Module, OpCode, Register, CART_FLAG_INT32_SAFE};

fn run_result(flags: u16, constants: &[i64], op: OpCode) -> Result<polka::Value, String> {
    let module = Module {
        functions: vec![Chunk::Bytecode(BytecodeChunk {
            code: vec![
                OpCode::PushConst(Register(0), 0),
                OpCode::PushConst(Register(1), 1),
                op,
                OpCode::Ret(Register(2)),
            ],
            constants: constants.iter().map(|&n| n as u64).collect(),
            reg_count: 3,
            ..BytecodeChunk::default()
        })],
        entry: 0,
        flags,
        exports: vec![],
    };
    VirtualMachine::new().run_module(&module)
}

fn run(flags: u16, constants: &[i64], op: OpCode) -> i64 {
    run_result(flags, constants, op).expect("execute opcode").as_int()
}

macro_rules! word_contract {
    ($narrow:ident, $wide:ident, $constants:expr, $op:expr, $expected32:expr, $expected64:expr) => {
        #[test]
        fn $narrow() {
            assert_eq!(run(CART_FLAG_INT32_SAFE, &$constants, $op), $expected32);
        }
        #[test]
        fn $wide() {
            assert_eq!(run(0, &$constants, $op), $expected64);
        }
    };
}

word_contract!(int32_add_wraps_at_signed_boundary, int64_add_preserves_32_bit_overflow,
    [2147483647, 1], OpCode::Add(Register(2), Register(0), Register(1)), -2147483648, 2147483648);
word_contract!(int32_sub_wraps_at_signed_boundary, int64_sub_preserves_32_bit_overflow,
    [-2147483648, 1], OpCode::Sub(Register(2), Register(0), Register(1)), 2147483647, -2147483649);
word_contract!(int32_mul_wraps_at_word_boundary, int64_mul_preserves_32_bit_overflow,
    [65536, 65536], OpCode::Mul(Register(2), Register(0), Register(1)), 0, 4294967296);
word_contract!(int32_neg_wraps_minimum, int64_neg_preserves_32_bit_overflow,
    [-2147483648, 0], OpCode::Neg(Register(2), Register(0)), -2147483648, 2147483648);
word_contract!(int32_add_imm_wraps_at_signed_boundary, int64_add_imm_preserves_32_bit_overflow,
    [2147483647, 0], OpCode::AddImm(Register(2), Register(0), 1), -2147483648, 2147483648);
word_contract!(int32_sub_imm_wraps_at_signed_boundary, int64_sub_imm_preserves_32_bit_overflow,
    [-2147483648, 0], OpCode::SubImm(Register(2), Register(0), 1), 2147483647, -2147483649);
word_contract!(int32_shl_wraps_at_signed_boundary, int64_shl_preserves_32_bit_overflow,
    [1, 31], OpCode::Shl(Register(2), Register(0), Register(1)), -2147483648, 2147483648);
word_contract!(int32_shl_masks_count_to_five_bits, int64_shl_keeps_six_bit_count,
    [1, 32], OpCode::Shl(Register(2), Register(0), Register(1)), 1, 4294967296);
word_contract!(int32_shr_masks_count_to_five_bits, int64_shr_keeps_six_bit_count,
    [-2147483648, 32], OpCode::Shr(Register(2), Register(0), Register(1)), -2147483648, -1);
word_contract!(int32_shr_sign_extends, int64_shr_sign_extends,
    [-2147483648, 1], OpCode::Shr(Register(2), Register(0), Register(1)), -1073741824, -1073741824);

#[test]
fn int32_div_traps_signed_overflow() {
    assert!(run_result(CART_FLAG_INT32_SAFE, &[i32::MIN as i64, -1],
        OpCode::Div(Register(2), Register(0), Register(1))).is_err());
}

#[test]
fn int64_div_traps_signed_overflow() {
    assert!(run_result(0, &[i64::MIN, -1],
        OpCode::Div(Register(2), Register(0), Register(1))).is_err());
}

#[test]
fn int32_mod_traps_signed_overflow() {
    assert!(run_result(CART_FLAG_INT32_SAFE, &[i32::MIN as i64, -1],
        OpCode::Mod(Register(2), Register(0), Register(1))).is_err());
}

#[test]
fn int64_mod_traps_signed_overflow() {
    assert!(run_result(0, &[i64::MIN, -1],
        OpCode::Mod(Register(2), Register(0), Register(1))).is_err());
}
