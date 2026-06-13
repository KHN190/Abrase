mod common;
use common::*;
use polka::{OpCode, Register, Chunk};
use myriad::{Value, VirtualMachine};

#[test]
fn add_then_return() {
    assert_same(
        vec![
            OpCode::PushConst(r(0), 0), OpCode::PushConst(r(1), 1),
            OpCode::Add(r(2), r(0), r(1)), OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(7).raw(), Value::from_int(5).raw()], 4,
    );
}

#[test]
fn wrapping_overflow_at_i64_max() {
    assert_same(
        vec![
            OpCode::PushConst(r(0), 0), OpCode::PushConst(r(1), 1),
            OpCode::Add(r(2), r(0), r(1)), OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(i64::MAX).raw(), Value::from_int(1).raw()], 4,
    );
}

#[test]
fn neg_of_i64_min_wraps() {
    assert_same(
        vec![OpCode::PushConst(r(0), 0), OpCode::Neg(r(1), r(0)), OpCode::Ret(r(1))],
        vec![Value::from_int(i64::MIN).raw()], 4,
    );
}

#[test]
fn shift_amount_masked_to_six_bits() {
    assert_same(
        vec![
            OpCode::PushConst(r(0), 0), OpCode::PushConst(r(1), 1),
            OpCode::Shl(r(2), r(0), r(1)), OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(1).raw(), Value::from_int(65).raw()], 4,
    );
}

#[test]
fn div_by_zero_matches_error() {
    assert_same(
        vec![
            OpCode::PushConst(r(0), 0), OpCode::PushConst(r(1), 1),
            OpCode::Div(r(2), r(0), r(1)), OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(10).raw(), Value::from_int(0).raw()], 4,
    );
}

#[test]
fn div_min_by_neg_one_overflows_to_error() {
    assert_same(
        vec![
            OpCode::PushConst(r(0), 0), OpCode::PushConst(r(1), 1),
            OpCode::Div(r(2), r(0), r(1)), OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(i64::MIN).raw(), Value::from_int(-1).raw()], 4,
    );
}

#[test]
fn mod_by_zero_matches_error() {
    assert_same(
        vec![
            OpCode::PushConst(r(0), 0), OpCode::PushConst(r(1), 1),
            OpCode::Mod(r(2), r(0), r(1)), OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(10).raw(), Value::from_int(0).raw()], 4,
    );
}

// Signed-semantics class: ops whose result differs under signed vs unsigned
// interpretation must follow the interpreter's i64 view (the Shr bug's family).
fn binop_neg(op: fn(Register, Register, Register) -> OpCode, a: i64, b: i64) -> (Vec<OpCode>, Vec<u64>) {
    (
        vec![OpCode::PushConst(r(0), 0), OpCode::PushConst(r(1), 1), op(r(2), r(0), r(1)), OpCode::Ret(r(2))],
        vec![Value::from_int(a).raw(), Value::from_int(b).raw()],
    )
}

#[test]
fn signed_lt_with_negative() {
    let (c, k) = binop_neg(OpCode::Lt, -1, 1);
    assert_same(c, k, 4);
}

#[test]
fn signed_div_negative_truncates_toward_zero() {
    let (c, k) = binop_neg(OpCode::Div, -7, 2);
    assert_same(c, k, 4);
}

#[test]
fn signed_mod_negative_keeps_sign_of_dividend() {
    let (c, k) = binop_neg(OpCode::Mod, -7, 2);
    assert_same(c, k, 4);
}

#[test]
fn arithmetic_shr_of_negative_sign_extends() {
    let (c, k) = binop_neg(OpCode::Shr, -8, 1);
    assert_same(c, k, 4);
}

#[test]
fn signed_gte_with_negatives() {
    let (c, k) = binop_neg(OpCode::Gte, -5, -10);
    assert_same(c, k, 4);
}

#[test]
fn backward_loop_sums_to_ten() {
    assert_same(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::PushConst(r(1), 1),
            OpCode::Add(r(0), r(0), r(1)),
            OpCode::SubImm(r(1), r(1), 1),
            OpCode::Jnz(r(1), -3),
            OpCode::Ret(r(0)),
        ],
        vec![Value::from_int(0).raw(), Value::from_int(4).raw()], 4,
    );
}

#[test]
fn forward_conditional_skip() {
    assert_same(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::PushConst(r(1), 1),
            OpCode::Jz(r(0), 1),
            OpCode::PushConst(r(1), 2),
            OpCode::Ret(r(1)),
        ],
        vec![Value::from_int(0).raw(), Value::from_int(1).raw(), Value::from_int(99).raw()], 4,
    );
}

#[test]
fn unconditional_jmp_skips_code() {
    assert_same(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::Jmp(1),
            OpCode::PushConst(r(0), 1),
            OpCode::Ret(r(0)),
        ],
        vec![Value::from_int(5).raw(), Value::from_int(99).raw()], 4,
    );
}

#[test]
fn move_copies_register() {
    assert_same(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::Move(r(1), r(0)),
            OpCode::Ret(r(1)),
        ],
        vec![Value::from_int(77).raw()], 4,
    );
}

#[test]
fn gt_lte_neq_or_match() {
    for op in [OpCode::Gt as fn(Register, Register, Register) -> OpCode,
               OpCode::Lte, OpCode::Neq, OpCode::Or] {
        assert_same(
            vec![OpCode::PushConst(r(0), 0), OpCode::PushConst(r(1), 1), op(r(2), r(0), r(1)), OpCode::Ret(r(2))],
            vec![Value::from_int(-3).raw(), Value::from_int(5).raw()], 4,
        );
    }
}

#[test]
fn ret_less_halt_yields_r0() {
    assert_same(
        vec![OpCode::PushConst(r(0), 0)],
        vec![Value::from_int(123).raw()], 4,
    );
}

#[test]
fn out_of_range_branch_errors_both_sides() {
    let bc = chunk(
        vec![OpCode::PushConst(r(0), 0), OpCode::Jmp(100), OpCode::Ret(r(0))],
        vec![Value::from_int(1).raw()], 4,
    );
    match (interp(&bc), transpiled(&bc)) {
        (Outcome::Err(e), Outcome::Err(k)) =>
            assert!(e.contains("out of range") && k.contains("out of range"),
                "interp={:?} transpiled={:?}", e, k),
        (i, t) => panic!("expected both Err: interp={:?} transpiled={:?}", i, t),
    }
}

#[test]
fn read_after_move_sees_consumed_sentinel() {
    assert_same(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::Move(r(1), r(0)),
            OpCode::Add(r(2), r(0), r(1)),
            OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(5).raw()], 4,
    );
}

#[test]
fn self_move_is_identity() {
    assert_same(
        vec![OpCode::PushConst(r(0), 0), OpCode::Move(r(0), r(0)), OpCode::Ret(r(0))],
        vec![Value::from_int(88).raw()], 4,
    );
}

fn random_program(rng: &mut Rng) -> (Vec<OpCode>, Vec<u64>) {
    let edge = [0i64, 1, -1, 2, i64::MAX, i64::MIN, 7, -7, 1000];
    let nconst = 2 + rng.below(4);
    let constants: Vec<u64> = (0..nconst)
        .map(|_| Value::from_int(edge[rng.below(edge.len())]).raw())
        .collect();
    let nreg = 6;
    let body_len = 3 + rng.below(8);
    let mut code = Vec::new();
    // def-before-use: reading an uninit register is malformed/UB, out of contract.
    let mut defined: Vec<u8> = Vec::new();
    let def = |reg: u8, defined: &mut Vec<u8>| {
        if !defined.contains(&reg) { defined.push(reg); }
    };
    code.push(OpCode::PushConst(r(0), 0));
    def(0, &mut defined);
    code.push(OpCode::PushConst(r(1), (1 % nconst) as u16));
    def(1, &mut defined);
    let pick_def = |rng: &mut Rng, defined: &[u8]| defined[rng.below(defined.len())];
    for _ in 0..body_len {
        let d = r(rng.below(nreg) as u8);
        let a = r(pick_def(rng, &defined));
        let b = r(pick_def(rng, &defined));
        let op = random_op(rng, d, a, b, nconst);
        def(d.0, &mut defined);
        code.push(op);
    }
    let here = code.len();
    let back = 1 + rng.below(here.min(3).max(1));
    let cond = r(pick_def(rng, &defined));
    code.push(if rng.below(2) == 0 { OpCode::Jz(cond, -(back as i16)) } else { OpCode::Jnz(cond, -(back as i16)) });
    if rng.below(4) != 0 {
        code.push(OpCode::Ret(r(pick_def(rng, &defined))));
    }
    let _ = nreg;
    (code, constants)
}

#[test]
fn fuzz_integer_programs_match_interpreter() {
    let n: u64 = std::env::var("FUZZ_N").ok().and_then(|s| s.parse().ok()).unwrap_or(60);
    let mut rng = Rng(0x9E3779B97F4A7C15);
    let (mut mods, mut interps) = (Vec::new(), Vec::new());
    for _ in 0..n {
        let (code, constants) = random_program(&mut rng);
        let bc = chunk(code, constants, 6);
        let i = match VirtualMachine::new().with_step_cap(100_000).run(&Chunk::Bytecode(bc.clone())) {
            Ok(v) => Outcome::Ok(v.raw()),
            Err(e) => Outcome::Err(e),
        };
        // step-cap = non-terminating shape; transpiled binary would hang. Skip.
        if let Outcome::Err(e) = &i {
            if e.contains("step cap") { continue; }
        }
        interps.push((i, 0));
        mods.push(module_of(bc));
    }
    batch_compare(mods, interps, false);
}
