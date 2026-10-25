// Under --int32 a Float register holds f32 bits. Every float builtin must read
// its arguments and write its result with that encoding, so a program's result
// under int32 equals the f64-mode result rounded to f32.
// Oracle: the same source compiled without int32, result cast to f32.

use abrase::compiler::Compiler;
use abrase::lexer::Lexer;
use abrase::parser::Parser;
use myriad::{word32, word64};

fn run_raw(src: &str, int32: bool) -> Result<(u64, Option<String>), String> {
    let mut p = Parser::new(Lexer::new(src)).with_source(src.to_string());
    let ast = p.parse_program();
    if !p.errors.is_empty() { return Err(format!("parse:\n{}", p.pretty_print_errors())); }
    let mut c = Compiler::new().with_source(src.to_string()).with_int32_mode(int32);
    let module = c.compile_module(&ast).map_err(|_| c.pretty_print_errors())?;
    macro_rules! execute {
        ($profile:ident) => {{
            let mut vm = $profile::VirtualMachine::new().with_step_cap(1_000_000);
            let value = vm.run_module(&module)?;
            let text = $profile::read_string(vm.heap_ref(), value);
            Ok((value.raw() as u64, text))
        }};
    }
    if int32 { execute!(word32) } else { execute!(word64) }
}

fn float_result(src: &str, int32: bool) -> f64 {
    let (raw, _) = run_raw(src, int32).unwrap_or_else(|e| panic!("int32={}: {}\n{}", int32, e, src));
    if int32 { f32::from_bits(raw as u32) as f64 } else { f64::from_bits(raw) }
}

fn int_result(src: &str, int32: bool) -> i64 {
    let (raw, _) = run_raw(src, int32).unwrap_or_else(|e| panic!("int32={}: {}\n{}", int32, e, src));
    if int32 { raw as u32 as i32 as i64 } else { raw as i64 }
}

fn string_result(src: &str, int32: bool) -> String {
    let (_, s) = run_raw(src, int32).unwrap_or_else(|e| panic!("int32={}: {}\n{}", int32, e, src));
    s.unwrap_or_else(|| panic!("int32={}: not a string\n{}", int32, src))
}

fn assert_float_matches_oracle(body: &str) {
    let src = format!("fn main() -> Float {{ {} }}", body);
    let oracle = float_result(&src, false) as f32 as f64;
    let got = float_result(&src, true);
    let tol = (oracle.abs() * 1e-6).max(1e-6);
    assert!((got - oracle).abs() <= tol, "int32 {} != oracle {}\n{}", got, oracle, src);
}

fn assert_int_matches_oracle(body: &str) {
    let src = format!("fn main() -> Int {{ {} }}", body);
    assert_eq!(int_result(&src, true), int_result(&src, false), "{}", src);
}

#[test]
fn float_to_i_truncates_under_int32() {
    assert_int_matches_oracle("let x = 2.5; x.to_i()");
}

#[test]
fn float_abs_under_int32() {
    assert_float_matches_oracle("(0.0 - 1.5).abs()");
}

#[test]
fn float_max_under_int32() {
    assert_float_matches_oracle("let x = 2.5; x.max(3.5)");
}

#[test]
fn float_min_under_int32() {
    assert_float_matches_oracle("let x = 2.5; x.min(3.5)");
}

#[test]
fn float_sqrt_under_int32() {
    assert_float_matches_oracle("sqrt(4.0)");
}

#[test]
fn float_sin_cos_under_int32() {
    assert_float_matches_oracle("sin(0.5) + cos(0.5)");
}

#[test]
fn float_ceil_floor_under_int32() {
    assert_float_matches_oracle("ceil(2.5) + flr(2.5)");
}

#[test]
fn int_to_f_result_usable_in_vm_arithmetic_under_int32() {
    assert_float_matches_oracle("3.to_f() + 1.0");
}

#[test]
fn builtin_result_feeds_vm_arithmetic_under_int32() {
    assert_float_matches_oracle("let x = 2.5; x.max(3.5) * 2.0");
}

#[test]
fn vm_arithmetic_feeds_builtin_under_int32() {
    assert_float_matches_oracle("let x = 2.5; (x * 2.0).abs()");
}

#[test]
fn float_to_s_under_int32() {
    let src = "fn main() -> String { let x = 2.5; x.to_s() }";
    assert_eq!(string_result(src, true), string_result(src, false));
}

#[test]
fn float_interpolation_under_int32() {
    let src = "fn main() -> String { let x = 2.5; \"{x}\" }";
    assert_eq!(string_result(src, true), string_result(src, false));
}

#[test]
fn float_to_s_after_builtin_under_int32() {
    let src = "fn main() -> String { let x = 2.5; let m = x.max(3.5); m.to_s() }";
    assert_eq!(string_result(src, true), string_result(src, false));
}

#[test]
fn plain_float_arithmetic_under_int32_is_baseline() {
    assert_float_matches_oracle("let x = 2.5; x * 2.0 + 0.5");
}

struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self { Self(seed.wrapping_mul(6364136223846793005).wrapping_add(1)) }
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0
    }
    fn pick(&mut self, n: u64) -> u64 { self.next() % n }
}

fn lit(rng: &mut Rng) -> String {
    let n = rng.pick(64) as i64 - 32;
    format!("{}.{}", n, ["0", "5", "25", "75"][rng.pick(4) as usize])
}

fn gen_float_expr(rng: &mut Rng, depth: u32) -> String {
    if depth == 0 { return lit(rng); }
    match rng.pick(10) {
        0 => format!("({}).abs()", gen_float_expr(rng, depth - 1)),
        1 => format!("({}).max({})", gen_float_expr(rng, depth - 1), gen_float_expr(rng, depth - 1)),
        2 => format!("({}).min({})", gen_float_expr(rng, depth - 1), gen_float_expr(rng, depth - 1)),
        3 => format!("sqrt(({}).abs())", gen_float_expr(rng, depth - 1)),
        4 => format!("ceil({})", gen_float_expr(rng, depth - 1)),
        5 => format!("flr({})", gen_float_expr(rng, depth - 1)),
        6 => format!("({}).to_i().to_f()", gen_float_expr(rng, depth - 1)),
        7 => format!("({} + {})", gen_float_expr(rng, depth - 1), gen_float_expr(rng, depth - 1)),
        8 => format!("({} * {})", gen_float_expr(rng, depth - 1), gen_float_expr(rng, depth - 1)),
        _ => format!("({} - {})", gen_float_expr(rng, depth - 1), gen_float_expr(rng, depth - 1)),
    }
}

#[test]
fn fuzz_float_builtins_match_f64_oracle_under_int32() {
    let mut failures = String::new();
    let mut n_fail = 0u32;
    for seed in 0..300u64 {
        let mut rng = Rng::new(seed);
        let body = gen_float_expr(&mut rng, 3);
        let src = format!("fn main() -> Float {{ {} }}", body);
        let oracle = match run_raw(&src, false) {
            Ok((raw, _)) => f64::from_bits(raw) as f32 as f64,
            Err(_) => continue,
        };
        let got = match run_raw(&src, true) {
            Ok((raw, _)) => f32::from_bits(raw as u32) as f64,
            Err(e) => { n_fail += 1; failures.push_str(&format!("seed={} int32 run failed: {}\n{}\n", seed, e, src)); continue; }
        };
        let tol = (oracle.abs() * 1e-5).max(1e-5);
        if !(got - oracle).abs().le(&tol) && !(got.is_nan() && oracle.is_nan()) {
            n_fail += 1;
            if failures.len() < 4000 {
                failures.push_str(&format!("seed={} int32={} oracle={}\n{}\n", seed, got, oracle, src));
            }
        }
    }
    assert_eq!(n_fail, 0, "{} divergent program(s):\n{}", n_fail, failures);
}
