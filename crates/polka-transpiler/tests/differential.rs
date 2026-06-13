use polka::{BytecodeChunk, Chunk, Module, OpCode, Register};
use polka_transpiler::{transpile_module, transpile_program};
use myriad::{Value, VirtualMachine};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

fn r(n: u8) -> Register { Register(n) }

// Resolve a myriad rlib that links as `--extern myriad`. Several build variants
// sit in target/debug/deps; probe-compile each until one links, then cache it.
fn myriad_rlib() -> &'static str {
    static RLIB: OnceLock<String> = OnceLock::new();
    RLIB.get_or_init(|| {
        let deps = format!("{}/target/debug/deps", env!("CARGO_MANIFEST_DIR").trim_end_matches("/crates/polka-transpiler"));
        let dir = std::path::Path::new(&deps);
        let probe = std::env::temp_dir().join(format!("polka_probe_{}.rs", std::process::id()));
        std::fs::write(&probe, "fn main() { let _ = myriad::Heap::new(); }").unwrap();
        for entry in std::fs::read_dir(dir).expect("deps dir") {
            let p = entry.unwrap().path();
            let name = p.file_name().unwrap().to_string_lossy().to_string();
            if !(name.starts_with("libmyriad-") && name.ends_with(".rlib")) { continue; }
            let bin = std::env::temp_dir().join(format!("polka_probe_{}.bin", std::process::id()));
            let ok = Command::new("rustc")
                .args(["--edition", "2021"])
                .arg("--extern").arg(format!("myriad={}", p.display()))
                .arg("-L").arg(&deps)
                .arg(&probe).arg("-o").arg(&bin)
                .stderr(std::process::Stdio::null())
                .status().map(|s| s.success()).unwrap_or(false);
            if ok { return p.display().to_string(); }
        }
        panic!("no linkable myriad rlib in {}", deps);
    })
}

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

// Returns (outcome, live_cells). Programs print `OK <value> <live>` or `ERR <msg>`.
fn compile_run_full(src: &str) -> (Outcome, usize) {
    let id = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("polka_tp_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let src_path = dir.join(format!("prog_{}.rs", id));
    let bin_path = dir.join(format!("prog_{}.bin", id));
    std::fs::write(&src_path, src).unwrap();
    let deps = format!("{}/target/debug/deps", env!("CARGO_MANIFEST_DIR").trim_end_matches("/crates/polka-transpiler"));
    let status = Command::new("rustc")
        .args(["-O", "--edition", "2021"])
        .arg("--extern").arg(format!("myriad={}", myriad_rlib()))
        .arg("-L").arg(&deps)
        .arg(&src_path).arg("-o").arg(&bin_path)
        .status().expect("rustc");
    assert!(status.success(), "rustc failed on:\n{}", src);
    let out = Command::new(&bin_path).output().expect("run binary");
    let s = String::from_utf8(out.stdout).unwrap();
    let s = s.trim();
    if let Some(rest) = s.strip_prefix("OK ") {
        let mut it = rest.split_whitespace();
        let v: u64 = it.next().expect("value").parse().expect("parse u64");
        let live: usize = it.next().expect("live").parse().expect("parse live");
        (Outcome::Ok(v), live)
    } else if let Some(rest) = s.strip_prefix("ERR ") {
        (Outcome::Err(rest.to_string()), 0)
    } else {
        panic!("unexpected program output: {:?}", s);
    }
}

fn compile_run(src: &str) -> Outcome { compile_run_full(src).0 }

fn transpiled(bc: &BytecodeChunk) -> Outcome {
    compile_run(&transpile_program(bc).expect("transpile"))
}

fn compare(i: &Outcome, t: &Outcome) {
    match (i, t) {
        (Outcome::Ok(a), Outcome::Ok(b)) => assert_eq!(a, b, "value mismatch"),
        (Outcome::Err(e), Outcome::Err(k)) =>
            assert!(e.contains(k.as_str()), "error category mismatch: interp={:?} transpiled={:?}", e, k),
        _ => panic!("outcome kind mismatch: interp={:?} transpiled={:?}", i, t),
    }
}

fn assert_same(code: Vec<OpCode>, constants: Vec<u64>, reg_count: usize) {
    let bc = chunk(code, constants, reg_count);
    compare(&interp(&bc), &transpiled(&bc));
}

fn fn_chunk(code: Vec<OpCode>, constants: Vec<u64>, reg_count: usize, param_count: usize) -> Chunk {
    Chunk::Bytecode(BytecodeChunk {
        code, constants,
        const_mask: Vec::new(),
        string_constants: Vec::new(),
        reg_count, param_count,
        lines: Vec::new(),
        src_file: String::new(),
    })
}

fn assert_same_module(functions: Vec<Chunk>, entry: usize) {
    let module = Module { functions, entry, flags: 0, exports: vec![] };
    let i = match VirtualMachine::new().with_step_cap(1_000_000).run_module(&module) {
        Ok(v) => Outcome::Ok(v.raw()),
        Err(e) => Outcome::Err(e),
    };
    let t = compile_run(&transpile_module(&module).expect("transpile module"));
    compare(&i, &t);
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
    code.push(OpCode::Ret(r(pick_def(rng, &defined))));
    let _ = nreg;
    (code, constants)
}

fn random_op(rng: &mut Rng, d: Register, a: Register, b: Register, nconst: usize) -> OpCode {
    match rng.below(21) {
        0 => OpCode::Add(d, a, b),
        1 => OpCode::Sub(d, a, b),
        2 => OpCode::Mul(d, a, b),
        3 => OpCode::Div(d, a, b),
        4 => OpCode::Mod(d, a, b),
        5 => OpCode::Neg(d, a),
        6 => OpCode::Lt(d, a, b),
        7 => OpCode::Gt(d, a, b),
        8 => OpCode::Lte(d, a, b),
        9 => OpCode::Gte(d, a, b),
        10 => OpCode::Eq(d, a, b),
        11 => OpCode::Neq(d, a, b),
        12 => OpCode::And(d, a, b),
        13 => OpCode::Or(d, a, b),
        14 => OpCode::Xor(d, a, b),
        15 => OpCode::Shl(d, a, b),
        16 => OpCode::Shr(d, a, b),
        17 => OpCode::AddImm(d, a, (rng.next() % 7) as i8 - 3),
        18 => OpCode::SubImm(d, a, (rng.next() % 7) as i8 - 3),
        19 => OpCode::Move(d, a),
        _  => OpCode::PushConst(d, rng.below(nconst) as u16),
    }
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

#[test]
fn call_square_function() {
    let main = fn_chunk(
        vec![OpCode::PushConst(r(4), 0), OpCode::Call(r(0), 1), OpCode::Ret(r(0))],
        vec![Value::from_int(9).raw()], 4, 0,
    );
    let square = fn_chunk(
        vec![OpCode::Mul(r(1), r(0), r(0)), OpCode::Ret(r(1))],
        vec![], 2, 1,
    );
    assert_same_module(vec![main, square], 0);
}

#[test]
fn recursive_factorial() {
    let main = fn_chunk(
        vec![OpCode::PushConst(r(4), 0), OpCode::Call(r(0), 1), OpCode::Ret(r(0))],
        vec![Value::from_int(5).raw()], 4, 0,
    );
    let fact = fn_chunk(
        vec![
            OpCode::PushConst(r(1), 0),
            OpCode::Lte(r(2), r(0), r(1)),
            OpCode::Jz(r(2), 1),
            OpCode::Ret(r(1)),
            OpCode::SubImm(r(3), r(0), 1),
            OpCode::Copy(r(5), r(3)),
            OpCode::Call(r(4), 1),
            OpCode::Mul(r(4), r(0), r(4)),
            OpCode::Ret(r(4)),
        ],
        vec![Value::from_int(1).raw()], 5, 1,
    );
    assert_same_module(vec![main, fact], 0);
}

fn run_module_outcome(module: &Module) -> Outcome {
    match VirtualMachine::new().with_step_cap(1_000_000).run_module(module) {
        Ok(v) => Outcome::Ok(v.raw()),
        Err(e) => Outcome::Err(e),
    }
}

// Leaf helper: param_count params (def from the start), random straight-line
// body, ends Ret. No nested call, so no use-before-def across frames.
fn random_leaf(rng: &mut Rng, nparams: usize) -> Chunk {
    let edge = [0i64, 1, -1, 2, i64::MAX, i64::MIN, 7];
    let nconst = 1 + rng.below(3);
    let constants: Vec<u64> = (0..nconst)
        .map(|_| Value::from_int(edge[rng.below(edge.len())]).raw())
        .collect();
    let nreg = (nparams + 4).max(4);
    let mut defined: Vec<u8> = (0..nparams as u8).collect();
    if defined.is_empty() { defined.push(0); }
    let mut code = Vec::new();
    code.push(OpCode::PushConst(r(nparams as u8), 0));
    if !defined.contains(&(nparams as u8)) { defined.push(nparams as u8); }
    let body = 2 + rng.below(6);
    for _ in 0..body {
        let d = r(rng.below(nreg) as u8);
        let a = r(defined[rng.below(defined.len())]);
        let b = r(defined[rng.below(defined.len())]);
        code.push(random_op(rng, d, a, b, nconst));
        if !defined.contains(&d.0) { defined.push(d.0); }
    }
    code.push(OpCode::Ret(r(defined[rng.below(defined.len())])));
    fn_chunk(code, constants, nreg, nparams)
}

#[test]
fn fuzz_call_programs_match_interpreter() {
    let n: u64 = std::env::var("FUZZ_N").ok().and_then(|s| s.parse().ok()).unwrap_or(40);
    let mut rng = Rng(0xD1B54A32D192ED03);
    for _ in 0..n {
        let nparams = 1 + rng.below(3);
        let leaf = random_leaf(&mut rng, nparams);
        let mreg = 4;
        let nconst = nparams;
        let constants: Vec<u64> = (0..nconst)
            .map(|_| Value::from_int([0i64, 1, -1, 2, 7, -7][rng.below(6)]).raw())
            .collect();
        let mut code = Vec::new();
        for i in 0..nparams {
            code.push(OpCode::PushConst(r((mreg + i) as u8), i as u16));
        }
        code.push(OpCode::Call(r(0), 1));
        code.push(OpCode::Ret(r(0)));
        let main = fn_chunk(code, constants, mreg, 0);
        let module = Module { functions: vec![main, leaf], entry: 0, flags: 0, exports: vec![] };
        let i = run_module_outcome(&module);
        if let Outcome::Err(e) = &i { if e.contains("step cap") { continue; } }
        let src = transpile_module(&module).expect("transpile module");
        let t = compile_run(&src);
        match (&i, &t) {
            (Outcome::Ok(a), Outcome::Ok(b)) =>
                assert_eq!(a, b, "value mismatch interp={} transpiled={} on:\n{}", a, b, src),
            (Outcome::Err(e), Outcome::Err(k)) =>
                assert!(e.contains(k.as_str()), "error mismatch: interp={:?} transpiled={:?}\n{}", e, k, src),
            _ => panic!("outcome kind mismatch: interp={:?} transpiled={:?}\n{}", i, t, src),
        }
    }
}

#[test]
fn two_param_call() {
    let main = fn_chunk(
        vec![
            OpCode::PushConst(r(4), 0),
            OpCode::PushConst(r(5), 1),
            OpCode::Call(r(0), 1),
            OpCode::Ret(r(0)),
        ],
        vec![Value::from_int(20).raw(), Value::from_int(22).raw()], 4, 0,
    );
    let add2 = fn_chunk(
        vec![OpCode::Add(r(2), r(0), r(1)), OpCode::Ret(r(2))],
        vec![], 3, 2,
    );
    assert_same_module(vec![main, add2], 0);
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
fn native_fn_in_module_unsupported() {
    use polka::{Chunk as PChunk, NativeChunk};
    let main = fn_chunk(vec![OpCode::PushConst(r(0), 0), OpCode::Ret(r(0))], vec![Value::from_int(1).raw()], 4, 0);
    let native = PChunk::Native(NativeChunk { name: "print".into(), param_count: 1 });
    let module = Module { functions: vec![main, native], entry: 0, flags: 0, exports: vec![] };
    assert!(transpile_module(&module).is_err(), "native fn should be unsupported");
}

#[test]
fn error_propagates_across_call_frames() {
    let main = fn_chunk(
        vec![
            OpCode::PushConst(r(4), 0),
            OpCode::PushConst(r(5), 1),
            OpCode::Call(r(0), 1),
            OpCode::Ret(r(0)),
        ],
        vec![Value::from_int(10).raw(), Value::from_int(0).raw()], 4, 0,
    );
    let divide = fn_chunk(
        vec![OpCode::Div(r(2), r(0), r(1)), OpCode::Ret(r(2))],
        vec![], 3, 2,
    );
    assert_same_module(vec![main, divide], 0);
}

#[test]
fn nested_call_chain_two_levels() {
    let main = fn_chunk(
        vec![OpCode::PushConst(r(4), 0), OpCode::Call(r(0), 1), OpCode::Ret(r(0))],
        vec![Value::from_int(6).raw()], 4, 0,
    );
    let mid = fn_chunk(
        vec![OpCode::Copy(r(2), r(0)), OpCode::Call(r(1), 2), OpCode::AddImm(r(1), r(1), 1), OpCode::Ret(r(1))],
        vec![], 2, 1,
    );
    let leaf = fn_chunk(
        vec![OpCode::Mul(r(1), r(0), r(0)), OpCode::Ret(r(1))],
        vec![], 2, 1,
    );
    assert_same_module(vec![main, mid, leaf], 0);
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

#[test]
fn deep_recursion_chain_300_levels() {
    let main = fn_chunk(
        vec![OpCode::PushConst(r(4), 0), OpCode::Call(r(0), 1), OpCode::Ret(r(0))],
        vec![Value::from_int(300).raw()], 4, 0,
    );
    let sum = fn_chunk(
        vec![
            OpCode::PushConst(r(1), 0),
            OpCode::Eq(r(2), r(0), r(1)),
            OpCode::Jz(r(2), 1),
            OpCode::Ret(r(1)),
            OpCode::SubImm(r(3), r(0), 1),
            OpCode::Copy(r(6), r(3)),
            OpCode::Call(r(4), 1),
            OpCode::Add(r(4), r(0), r(4)),
            OpCode::Ret(r(4)),
        ],
        vec![Value::from_int(0).raw()], 6, 1,
    );
    assert_same_module(vec![main, sum], 0);
}

#[test]
fn effect_and_device_ops_unsupported() {
    for op in [
        OpCode::Dei(r(1), r(0)),
        OpCode::Deo(r(1), r(0)),
        OpCode::Handle(r(0), 0),
        OpCode::Resume(r(0), r(1)),
        OpCode::Raise(r(0), r(1), r(2)),
    ] {
        let bc = chunk(vec![op, OpCode::Ret(r(0))], vec![], 4);
        assert!(transpile_program(&bc).is_err(), "effect/device op should be unsupported");
    }
}

#[test]
fn module_with_exports_transpiles_via_entry() {
    use polka::Export;
    let main = fn_chunk(vec![OpCode::PushConst(r(0), 0), OpCode::Ret(r(0))], vec![Value::from_int(9).raw()], 4, 0);
    let helper = fn_chunk(vec![OpCode::Ret(r(0))], vec![], 2, 1);
    let module = Module {
        functions: vec![main, helper],
        entry: 0, flags: 0,
        exports: vec![Export { name: "helper".into(), fn_id: 1 }],
    };
    let i = run_module_outcome(&module);
    let t = compile_run(&transpile_module(&module).expect("transpile"));
    compare(&i, &t);
}

// Heap differential: compares the return value AND the final live-cell count, so
// an RC imbalance (leak or premature free) on either side is caught.
fn assert_same_heap(functions: Vec<Chunk>, entry: usize) {
    let module = Module { functions, entry, flags: 0, exports: vec![] };
    let mut vm = VirtualMachine::new().with_step_cap(1_000_000);
    let i = match vm.run_module(&module) {
        Ok(v) => Outcome::Ok(v.raw()),
        Err(e) => Outcome::Err(e),
    };
    let i_live = vm.heap_live_count();
    let (t, t_live) = compile_run_full(&transpile_module(&module).expect("transpile heap"));
    compare(&i, &t);
    if let Outcome::Ok(_) = i {
        assert_eq!(i_live, t_live, "heap live-count mismatch: interp={} transpiled={}", i_live, t_live);
    }
}

#[test]
fn heap_alloc_store_load_drop_no_leak() {
    let main = fn_chunk(
        vec![
            OpCode::Alloc(r(0), 2),
            OpCode::PushConst(r(1), 0),
            OpCode::St(r(1), r(0), 0),
            OpCode::Ld(r(2), r(0), 0),
            OpCode::Drop(r(0)),
            OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(42).raw()], 4, 0,
    );
    assert_same_heap(vec![main], 0);
}

#[test]
fn heap_leak_detected_when_not_dropped() {
    let main = fn_chunk(
        vec![
            OpCode::Alloc(r(0), 2),
            OpCode::PushConst(r(1), 0),
            OpCode::St(r(1), r(0), 0),
            OpCode::Ld(r(2), r(0), 0),
            OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(42).raw()], 4, 0,
    );
    assert_same_heap(vec![main], 0);
}

#[test]
fn nested_handle_drop_cascades_to_zero() {
    let main = fn_chunk(
        vec![
            OpCode::Alloc(r(0), 1),
            OpCode::Alloc(r(1), 1),
            OpCode::St(r(1), r(0), 0),
            OpCode::Drop(r(0)),
            OpCode::PushConst(r(2), 0),
            OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(0).raw()], 4, 0,
    );
    assert_same_heap(vec![main], 0);
}

#[test]
fn copy_handle_increments_refcount() {
    let main = fn_chunk(
        vec![
            OpCode::Alloc(r(0), 1),
            OpCode::Copy(r(1), r(0)),
            OpCode::Drop(r(0)),
            OpCode::Drop(r(1)),
            OpCode::PushConst(r(2), 0),
            OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(0).raw()], 4, 0,
    );
    assert_same_heap(vec![main], 0);
}

#[test]
fn handle_passed_through_call_no_leak() {
    let main = fn_chunk(
        vec![
            OpCode::Alloc(r(0), 1),
            OpCode::PushConst(r(5), 0),
            OpCode::St(r(5), r(0), 0),
            OpCode::Copy(r(4), r(0)),
            OpCode::Call(r(1), 1),
            OpCode::Drop(r(0)),
            OpCode::Ret(r(1)),
        ],
        vec![Value::from_int(99).raw()], 4, 0,
    );
    let reader = fn_chunk(
        vec![
            OpCode::Ld(r(1), r(0), 0),
            OpCode::Drop(r(0)),
            OpCode::Ret(r(1)),
        ],
        vec![], 2, 1,
    );
    assert_same_heap(vec![main, reader], 0);
}
