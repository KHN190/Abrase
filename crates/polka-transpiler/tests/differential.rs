use polka::{BytecodeChunk, Chunk, OpCode, Register};
use polka_transpiler::transpile_program;
use myriad::{Value, VirtualMachine};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

fn r(n: u8) -> Register { Register(n) }

fn chunk(code: Vec<OpCode>, constants: Vec<u64>, reg_count: usize) -> BytecodeChunk {
    BytecodeChunk {
        code, constants,
        const_mask: Vec::new(),
        string_constants: Vec::new(),
        reg_count, param_count: 0,
        lines: Vec::new(),
        src_file: String::new(),
    }
}

#[derive(Debug, PartialEq)]
enum Outcome { Ok(u64), Err(String) }

fn interp(bc: &BytecodeChunk) -> Outcome {
    match VirtualMachine::new().run(&Chunk::Bytecode(bc.clone())) {
        Ok(v) => Outcome::Ok(v.raw()),
        Err(e) => Outcome::Err(e),
    }
}

static SEQ: AtomicU64 = AtomicU64::new(0);

fn transpiled(bc: &BytecodeChunk) -> Outcome {
    let src = transpile_program(bc).expect("transpile");
    let id = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("polka_tp_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let src_path = dir.join(format!("prog_{}.rs", id));
    let bin_path = dir.join(format!("prog_{}.bin", id));
    std::fs::write(&src_path, &src).unwrap();
    let status = Command::new("rustc")
        .args(["-O", "--edition", "2021"])
        .arg(&src_path).arg("-o").arg(&bin_path)
        .status().expect("rustc");
    assert!(status.success(), "rustc failed on:\n{}", src);
    let out = Command::new(&bin_path).output().expect("run binary");
    let s = String::from_utf8(out.stdout).unwrap();
    let s = s.trim();
    if let Some(rest) = s.strip_prefix("OK ") {
        Outcome::Ok(rest.parse().expect("parse u64"))
    } else if let Some(rest) = s.strip_prefix("ERR ") {
        Outcome::Err(rest.to_string())
    } else {
        panic!("unexpected program output: {:?}", s);
    }
}

fn assert_same(code: Vec<OpCode>, constants: Vec<u64>, reg_count: usize) {
    let bc = chunk(code, constants, reg_count);
    let i = interp(&bc);
    let t = transpiled(&bc);
    match (&i, &t) {
        (Outcome::Ok(a), Outcome::Ok(b)) => assert_eq!(a, b, "value mismatch"),
        (Outcome::Err(e), Outcome::Err(k)) => {
            let key = k.as_str();
            assert!(e.contains(key), "error category mismatch: interp={:?} transpiled={:?}", e, k);
        }
        _ => panic!("outcome kind mismatch: interp={:?} transpiled={:?}", i, t),
    }
}

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
    // -1 < 1 is true signed, false unsigned
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
    // -8 >> 1 == -4 arithmetic, huge value if logical
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

// ---- Differential fuzzer ------------------------------------------------

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12; x ^= x << 25; x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    fn below(&mut self, n: usize) -> usize { (self.next() % n as u64) as usize }
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
    let mut def = |reg: u8, defined: &mut Vec<u8>| {
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
        let op = match rng.below(14) {
            0 => OpCode::Add(d, a, b),
            1 => OpCode::Sub(d, a, b),
            2 => OpCode::Mul(d, a, b),
            3 => OpCode::Div(d, a, b),
            4 => OpCode::Mod(d, a, b),
            5 => OpCode::Neg(d, a),
            6 => OpCode::Lt(d, a, b),
            7 => OpCode::Eq(d, a, b),
            8 => OpCode::And(d, a, b),
            9 => OpCode::Xor(d, a, b),
            10 => OpCode::Shl(d, a, b),
            11 => OpCode::Shr(d, a, b),
            12 => OpCode::AddImm(d, a, (rng.next() % 7) as i8 - 3),
            _  => OpCode::PushConst(d, rng.below(nconst) as u16),
        };
        def(d.0, &mut defined);
        code.push(op);
    }
    let here = code.len();
    let back = 1 + rng.below(here.min(3).max(1));
    code.push(OpCode::Jz(r(pick_def(rng, &defined)), -(back as i16)));
    code.push(OpCode::Ret(r(pick_def(rng, &defined))));
    let _ = nreg;
    (code, constants)
}

#[test]
fn fuzz_integer_programs_match_interpreter() {
    let n: u64 = std::env::var("FUZZ_N").ok().and_then(|s| s.parse().ok()).unwrap_or(60);
    let mut rng = Rng(0x9E3779B97F4A7C15);
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
        let t = transpiled(&bc);
        match (&i, &t) {
            (Outcome::Ok(a), Outcome::Ok(b)) =>
                assert_eq!(a, b, "value mismatch on:\n{}", transpile_program(&bc).unwrap()),
            (Outcome::Err(e), Outcome::Err(k)) =>
                assert!(e.contains(k.as_str()),
                    "error mismatch: interp={:?} transpiled={:?}", e, k),
            _ => panic!("outcome kind mismatch: interp={:?} transpiled={:?}\n{}",
                i, t, transpile_program(&bc).unwrap()),
        }
    }
}
