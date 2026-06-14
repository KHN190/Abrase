use crate::common::*;
use polka::{OpCode, Register};
use myriad::Value;

fn fbinop(op: fn(Register, Register, Register) -> OpCode, a: f64, b: f64) -> (Vec<OpCode>, Vec<u64>) {
    (
        vec![OpCode::PushConst(r(0), 0), OpCode::PushConst(r(1), 1), op(r(2), r(0), r(1)), OpCode::Ret(r(2))],
        vec![Value::from_float(a).raw(), Value::from_float(b).raw()],
    )
}

#[test]
fn float_arith_matches_f64() {
    for op in [OpCode::FAdd as fn(Register, Register, Register) -> OpCode, OpCode::FSub, OpCode::FMul, OpCode::FDiv] {
        let (c, k) = fbinop(op, 3.5, 1.25);
        assert_same_flags(c, k, 4, 0);
    }
}

#[test]
fn float_div_by_zero_is_inf_not_error() {
    let (c, k) = fbinop(OpCode::FDiv, 1.0, 0.0);
    assert_same_flags(c, k, 4, 0);
}

#[test]
fn float_compare_with_nan_is_false() {
    for op in [OpCode::FLt as fn(Register, Register, Register) -> OpCode, OpCode::FEq] {
        let (c, k) = fbinop(op, f64::NAN, 1.0);
        assert_same_flags(c, k, 4, 0);
    }
}

#[test]
fn float_neg_of_zero_keeps_sign_bit() {
    assert_same_flags(
        vec![OpCode::PushConst(r(0), 0), OpCode::FNeg(r(1), r(0)), OpCode::Ret(r(1))],
        vec![Value::from_float(0.0).raw()], 4, 0,
    );
}

#[test]
fn float_ops_narrow_to_f32_under_int32_flag() {
    use polka::CART_FLAG_INT32_SAFE;
    let (c, k) = (
        vec![OpCode::PushConst(r(0), 0), OpCode::PushConst(r(1), 1), OpCode::FAdd(r(2), r(0), r(1)), OpCode::Ret(r(2))],
        vec![Value::from_float_f32(0.1).raw(), Value::from_float_f32(0.2).raw()],
    );
    assert_same_flags(c, k, 4, CART_FLAG_INT32_SAFE);
}

#[test]
fn float_f32_div_inexact_narrows() {
    use polka::CART_FLAG_INT32_SAFE;
    let (c, k) = (
        vec![OpCode::PushConst(r(0), 0), OpCode::PushConst(r(1), 1), OpCode::FDiv(r(2), r(0), r(1)), OpCode::Ret(r(2))],
        vec![Value::from_float_f32(1.0).raw(), Value::from_float_f32(3.0).raw()],
    );
    assert_same_flags(c, k, 4, CART_FLAG_INT32_SAFE);
}

fn random_float_op(rng: &mut Rng, d: Register, a: Register, b: Register, nconst: usize) -> OpCode {
    match rng.below(8) {
        0 => OpCode::FAdd(d, a, b),
        1 => OpCode::FSub(d, a, b),
        2 => OpCode::FMul(d, a, b),
        3 => OpCode::FDiv(d, a, b),
        4 => OpCode::FNeg(d, a),
        5 => OpCode::FLt(d, a, b),
        6 => OpCode::FEq(d, a, b),
        _ => OpCode::PushConst(d, rng.below(nconst) as u16),
    }
}

fn random_float_program(rng: &mut Rng, f32_mode: bool) -> (Vec<OpCode>, Vec<u64>) {
    let edge = [0.0f64, 1.0, -1.0, 0.1, 3.0, -2.5, f64::INFINITY, f64::NAN, 1e30, -0.0];
    let nconst = 2 + rng.below(4);
    let constants: Vec<u64> = (0..nconst)
        .map(|_| {
            let f = edge[rng.below(edge.len())];
            if f32_mode { Value::from_float_f32(f).raw() } else { Value::from_float(f).raw() }
        })
        .collect();
    let nreg = 6;
    let mut code = vec![OpCode::PushConst(r(0), 0), OpCode::PushConst(r(1), (1 % nconst) as u16)];
    let mut defined: Vec<u8> = vec![0, 1];
    let body = 3 + rng.below(7);
    for _ in 0..body {
        let d = r(rng.below(nreg) as u8);
        let a = r(defined[rng.below(defined.len())]);
        let b = r(defined[rng.below(defined.len())]);
        code.push(random_float_op(rng, d, a, b, nconst));
        if !defined.contains(&d.0) { defined.push(d.0); }
    }
    code.push(OpCode::Ret(r(defined[rng.below(defined.len())])));
    (code, constants)
}

#[test]
fn fuzz_float_programs_match_interpreter() {
    use polka::CART_FLAG_INT32_SAFE;
    let n: u64 = std::env::var("FUZZ_N").ok().and_then(|s| s.parse().ok()).unwrap_or(40);
    let mut rng = Rng(0xF10A7C0DE5EED);
    let (mut mods, mut interps) = (Vec::new(), Vec::new());
    for _ in 0..n {
        let f32_mode = rng.below(2) == 0;
        let (code, constants) = random_float_program(&mut rng, f32_mode);
        let flags = if f32_mode { CART_FLAG_INT32_SAFE } else { 0 };
        let m = module_with_flags(chunk(code, constants, 6), flags);
        interps.push((run_module_outcome(&m), 0));
        mods.push(m);
    }
    batch_compare(mods, interps, false);
}
