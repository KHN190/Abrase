// Rejection fuzz: the dual of the value/leak fuzzers. Every generator emits a
// program that is ILLEGAL per wiki (escape-barrier, ownership, loop/handler
// scoping, types, or grammar) and asserts the compiler REJECTS it (parse error
// or compile error). A generated program that COMPILES is a soundness hole —
// exactly where loopholes hide. 0 deps, deterministic per seed.

use abrase::compiler::Compiler;
use abrase::lexer::Lexer;
use abrase::parser::Parser;

struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self { Self(seed.wrapping_mul(6364136223846793005).wrapping_add(1)) }
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0
    }
    fn range(&mut self, lo: i64, hi: i64) -> i64 { lo + (self.next() % (hi - lo) as u64) as i64 }
}

// True iff the source is rejected (parse or compile error).
fn is_rejected(src: &str) -> bool {
    let mut p = Parser::new(Lexer::new(src)).with_source(src.to_string());
    let ast = p.parse_program();
    if !p.errors.is_empty() { return true; }
    let mut c = Compiler::new().with_source(src.to_string());
    c.compile_module(&ast).is_err()
}

// ── illegal-program generators (each MUST be rejected) ────────────────────────

fn gen_shared_escape(r: &mut Rng) -> String {
    format!("fn main() -> Int {{ let s = region {{ Shared({}) }}; 0 }}", r.range(1, 99))
}

fn gen_closure_captures_shared(r: &mut Rng) -> String {
    format!("fn main() -> Int {{ region {{ let s = Shared({}); \
             let c = move |x: Int| -> Int {{ let _u = s; x }}; c(0) }} }}", r.range(1, 99))
}

fn gen_break_outside_loop(r: &mut Rng) -> String {
    format!("fn main() -> Int {{ break {} }}", r.range(0, 99))
}

fn gen_continue_outside_loop(_r: &mut Rng) -> String {
    "fn main() -> Int { continue }".to_string()
}

fn gen_resume_outside_arm(r: &mut Rng) -> String {
    format!("fn main() -> Int {{ resume({}) }}", r.range(0, 99))
}

fn gen_use_after_move(_r: &mut Rng) -> String {
    "fn main() -> String { let s = \"x\"; let a = s; let b = s; b }".to_string()
}

fn gen_unknown_var(r: &mut Rng) -> String {
    format!("fn main() -> Int {{ undefined_{} }}", r.range(0, 9999))
}

fn gen_type_mismatch(r: &mut Rng) -> String {
    format!("fn main() -> Int {{ true + {} }}", r.range(0, 99))
}

fn gen_arg_count(r: &mut Rng) -> String {
    format!("fn f(a: Int, b: Int) -> Int {{ a }} fn main() -> Int {{ f({}) }}", r.range(0, 99))
}

fn gen_ref_escape(r: &mut Rng) -> String {
    format!("fn main() -> Int {{ let r = region {{ let a = {}; &a }}; 0 }}", r.range(1, 99))
}

fn gen_bad_syntax(r: &mut Rng) -> String {
    match r.range(0, 4) {
        0 => "fn main() -> Int { let = 5; 0 }".to_string(),
        1 => format!("fn main() -> Int {{ {} + }}", r.range(0, 99)),
        2 => "fn main() -> Int { if { 1 } else { 2 } }".to_string(),
        _ => "fn main() -> Int { match { _ => 0 } }".to_string(),
    }
}

const ILLEGAL_GENS: &[(&str, fn(&mut Rng) -> String)] = &[
    ("shared_escape",            gen_shared_escape),
    ("closure_captures_shared",  gen_closure_captures_shared),
    ("break_outside_loop",       gen_break_outside_loop),
    ("continue_outside_loop",    gen_continue_outside_loop),
    ("resume_outside_arm",       gen_resume_outside_arm),
    ("use_after_move",           gen_use_after_move),
    ("unknown_var",              gen_unknown_var),
    ("type_mismatch",            gen_type_mismatch),
    ("arg_count",                gen_arg_count),
    ("ref_escape",               gen_ref_escape),
    ("bad_syntax",               gen_bad_syntax),
];

#[test]
fn rejection_fuzz_illegal_programs_rejected() {
    let mut holes: Vec<(u64, &str, String)> = Vec::new();
    for seed in 0..500u64 {
        for (name, g) in ILLEGAL_GENS {
            let mut rng = Rng::new(seed * 101 + name.len() as u64);
            let src = g(&mut rng);
            if !is_rejected(&src) {
                holes.push((seed, name, src));
            }
        }
    }
    if !holes.is_empty() {
        for (seed, name, src) in holes.iter().take(8) {
            eprintln!("--- SOUNDNESS HOLE seed={} gen={} (compiled, should reject) ---\n{}", seed, name, src);
        }
        panic!("rejection_fuzz: {} illegal program(s) were accepted", holes.len());
    }
}

#[test]
fn rejection_fuzz_ref_escape_rejected() {
    let mut rng = Rng::new(1);
    let mut holes = 0;
    for _ in 0..50 {
        let n = rng.range(1, 99);
        let src = format!("fn main() -> Int {{ let r = region {{ let a = {}; &a }}; 0 }}", n);
        if !is_rejected(&src) { holes += 1; }
    }
    assert_eq!(holes, 0, "&T escaping region accepted in {}/50 cases", holes);
}
