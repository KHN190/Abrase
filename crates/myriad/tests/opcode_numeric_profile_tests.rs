use myriad::{word32, word64};
use polka::{BytecodeChunk, Chunk, Module, OpCode, Register, CART_FLAG_INT32_SAFE};

fn r(n: u8) -> Register { Register(n) }

fn run(flags: u16, constants: Vec<u64>, code: Vec<OpCode>) -> (u64, usize) {
    let module = Module {
        functions: vec![Chunk::Bytecode(BytecodeChunk {
            constants, code, reg_count: 5, ..BytecodeChunk::default()
        })], entry: 0, flags, exports: vec![],
    };
    if flags & CART_FLAG_INT32_SAFE != 0 {
        let mut vm = word32::VirtualMachine::new();
        let value = vm.run_module(&module).expect("execute narrow profile").raw();
        (value as u64, vm.heap_live_count())
    } else {
        let mut vm = word64::VirtualMachine::new();
        let value = vm.run_module(&module).expect("execute wide profile").raw();
        (value, vm.heap_live_count())
    }
}

fn binary(flags: u16, a: u64, b: u64, op: OpCode) -> u64 {
    run(flags, vec![a, b], vec![
        OpCode::PushConst(r(0), 0), OpCode::PushConst(r(1), 1), op, OpCode::Ret(r(2)),
    ]).0
}

#[test]
fn float32_opcodes_match_native_f32_at_ieee_boundaries() {
    let patterns = [0, 0x80000000, 1, 0x007fffff, 0x00800000, 0x3f800000,
        0xbf800000, 0x7f7fffff, 0xff7fffff, 0x7f800000, 0xff800000, 0x7fc01234];
    for a in patterns {
        for b in patterns {
            let (x, y) = (f32::from_bits(a), f32::from_bits(b));
            for (op, expected) in [
                (OpCode::FAdd(r(2), r(0), r(1)), x + y),
                (OpCode::FSub(r(2), r(0), r(1)), x - y),
                (OpCode::FMul(r(2), r(0), r(1)), x * y),
                (OpCode::FDiv(r(2), r(0), r(1)), x / y),
            ] {
                let got = binary(CART_FLAG_INT32_SAFE, a as u64, b as u64, op);
                if expected.is_nan() { assert!(f32::from_bits(got as u32).is_nan()); }
                else { assert_eq!(got, expected.to_bits() as u64, "a={a:#x}, b={b:#x}"); }
            }
            assert_eq!(binary(CART_FLAG_INT32_SAFE, a as u64, b as u64,
                OpCode::FLt(r(2), r(0), r(1))), (x < y) as u64);
            assert_eq!(binary(CART_FLAG_INT32_SAFE, a as u64, b as u64,
                OpCode::FEq(r(2), r(0), r(1))), (x == y) as u64);
        }
    }
}

#[test]
fn float_neg_preserves_nan_payload_and_signed_zero_in_each_profile() {
    for bits in [0u32, 0x80000000, 1, 0x7f800001, 0x7fc01234, 0xff800001] {
        assert_eq!(binary(CART_FLAG_INT32_SAFE, bits as u64, 0,
            OpCode::FNeg(r(2), r(0))), (bits ^ 0x80000000) as u64);
    }
    for bits in [0u64, 0x8000000000000000, 1, 0x7ff0000000000001, 0x7ff8000000001234] {
        assert_eq!(binary(0, bits, 0, OpCode::FNeg(r(2), r(0))), bits ^ 0x8000000000000000);
    }
}

#[test]
fn float64_arithmetic_keeps_default_precision() {
    assert_eq!(binary(0, 16777216.0f64.to_bits(), 1.0f64.to_bits(),
        OpCode::FAdd(r(2), r(0), r(1))), 16777217.0f64.to_bits());
}

#[test]
fn integer32_comparisons_use_signed_low_word() {
    for (op, expected) in [
        (OpCode::Lt(r(2), r(0), r(1)), 1), (OpCode::Gt(r(2), r(0), r(1)), 0),
        (OpCode::Lte(r(2), r(0), r(1)), 1), (OpCode::Gte(r(2), r(0), r(1)), 0),
    ] {
        assert_eq!(binary(CART_FLAG_INT32_SAFE, 0x80000000, 0, op), expected);
    }
}

#[test]
fn integer32_bitwise_results_use_native_word_bits() {
    for (op, expected) in [
        (OpCode::And(r(2), r(0), r(1)), i32::MIN as u32 as u64),
        (OpCode::Or(r(2), r(0), r(1)), u32::MAX as u64),
        (OpCode::Xor(r(2), r(0), r(1)), i32::MAX as u64),
    ] {
        assert_eq!(binary(CART_FLAG_INT32_SAFE, 0x80000000, 0xffffffff, op), expected);
    }
}

#[test]
fn integer32_arithmetic_matches_canonical_negative_constant_for_raw_equality() {
    for (eq, expected) in [(true, 1), (false, 0)] {
        let cmp = if eq { OpCode::Eq(r(4), r(2), r(3)) } else { OpCode::Neq(r(4), r(2), r(3)) };
        let result = run(CART_FLAG_INT32_SAFE, vec![i32::MAX as u64, 1, i32::MIN as i64 as u64], vec![
            OpCode::PushConst(r(0), 0), OpCode::PushConst(r(1), 1),
            OpCode::Add(r(2), r(0), r(1)), OpCode::PushConst(r(3), 2), cmp, OpCode::Ret(r(4)),
        ]);
        assert_eq!(result, (expected, 0));
    }
}

#[test]
fn raw_equality_preserves_handle_bits_above_32() {
    for (eq, expected) in [(true, 0), (false, 1)] {
        let mut code = vec![OpCode::Alloc(r(0), 257)];
        for slot in 0..257 {
            code.extend([OpCode::Alloc(r(1), 1), OpCode::St(r(1), r(0), slot)]);
        }
        code.extend([OpCode::Ld(r(1), r(0), 0), OpCode::Ld(r(2), r(0), 256)]);
        code.push(if eq { OpCode::Eq(r(3), r(1), r(2)) } else { OpCode::Neq(r(3), r(1), r(2)) });
        code.extend([OpCode::Drop(r(1)), OpCode::Drop(r(2)), OpCode::Drop(r(0)), OpCode::Ret(r(3))]);
        assert_eq!(run(0, vec![], code), (expected, 0));
    }
}
