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

fn gen_local_alias(r: &mut Rng) -> String {
    let v = r.range(1, 99);
    format!("fn main() -> Int {{ let mut a = {v}; let r1 = &a; let r2 = &mut a; let _ = r1; let _ = r2; 0 }}")
}

fn gen_ref_payload_effect(r: &mut Rng) -> String {
    let v = r.range(1, 99);
    format!("effect E {{ op send(r: &Int) -> Int }}\nfn body() -> <E> Int {{ region {{ let a = {v}; E.send(&a) }} }}\nfn main() -> Int {{ handle body() {{ return v => v, E.send q => resume(*q) }} }}")
}

fn gen_record_double_move(r: &mut Rng) -> String {
    let v = r.range(1, 99);
    format!(
        "type P = {{ v: Int }}\n\
         fn consume(p: P) -> Int {{ p.v }}\n\
         fn main() -> Int {{ let p = P {{ v: {v} }}; consume(p) + consume(p) }}"
    )
}

fn gen_field_double_move(r: &mut Rng) -> String {
    let v = r.range(1, 9);
    format!(
        "type W = {{ s: String }}\n\
         fn take(w: W) -> Int {{ {v} }}\n\
         fn main() -> Int {{ let b = W {{ s: \"hi\" }}; take(b) + take(b) }}"
    )
}

fn gen_ref_escape_nested_region_effect(r: &mut Rng) -> String {
    let v = r.range(1, 99);
    format!(
        "effect E {{ op send(r: &Int) -> Int }}\n\
         fn body() -> <E> Int {{ region {{ region {{ let a = {v}; E.send(&a) }} }} }}\n\
         fn main() -> Int {{ handle body() {{ return v => v, E.send q => resume(*q) }} }}"
    )
}

fn gen_cart_non_native_effect(r: &mut Rng) -> String {
    let v = r.range(0, 9);
    format!(
        "effect E {{ op tick() -> Int }}\n\
         @cart\n\
         fn main() -> <E> Unit {{ let _ = E.tick(); let _ = {v}; () }}"
    )
}

fn gen_cart_graphics_without_host(r: &mut Rng) -> String {
    // The compute core declares the `Graphics` effect but provides no native
    // that discharges it, so a @cart main may not declare it here.
    let _ = r.range(0, 9);
    "@cart\nfn main() -> <frame, Graphics> Unit { loop { frame.present() } }".to_string()
}

fn gen_missing_effect_arm_in_handler(r: &mut Rng) -> String {
    let v = r.range(0, 9);
    format!(
        "effect E {{ op tick() -> Int }}\n\
         fn body() -> <E> Int {{ E.tick() }}\n\
         fn main() -> Int {{ handle body() {{ return _ => {v} }} }}"
    )
}

fn gen_fallible_call_without_question(r: &mut Rng) -> String {
    let v = r.range(1, 99);
    format!(
        "fn inner() -> <exn<Int>> Int {{ {v} }}\n\
         fn mid() -> <exn<Int>> Int {{ let x = inner(); x + 1 }}\n\
         fn main() -> Int {{ handle mid() {{ return v => v, exn _ => 0 }} }}"
    )
}

fn gen_mixed_fallible_plain_if_tail(r: &mut Rng) -> String {
    let v = r.range(1, 99);
    format!(
        "fn inner() -> <exn<Int>> Int {{ {v} }}\n\
         fn f(c: Bool) -> <exn<Int>> Int {{ if c {{ inner() }} else {{ {v} }} }}\n\
         fn main() -> Int {{ handle f(true) {{ return v => v, exn _ => 0 }} }}"
    )
}

fn gen_mut_borrow_scalar_var(r: &mut Rng) -> String {
    let v = r.range(1, 99);
    format!(
        "fn bump(p: &mut Int) -> Unit {{ }}\n\
         fn main() -> Int {{ let mut x = {v}; bump(&mut x); x }}"
    )
}

fn gen_mut_borrow_scalar_field(r: &mut Rng) -> String {
    let v = r.range(1, 99);
    format!(
        "type W = {{ n: Int }}\n\
         fn bump(p: &mut Int) -> Unit {{ }}\n\
         fn main() -> Int {{ let mut w = W {{ n: {v} }}; bump(&mut w.n); w.n }}"
    )
}

const ILLEGAL_GENS: &[(&str, fn(&mut Rng) -> String)] = &[
    ("fallible_no_question",      gen_fallible_call_without_question),
    ("mixed_fallible_if_tail",    gen_mixed_fallible_plain_if_tail),
    ("mut_borrow_scalar_var",     gen_mut_borrow_scalar_var),
    ("mut_borrow_scalar_field",   gen_mut_borrow_scalar_field),
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
    ("local_alias",              gen_local_alias),
    ("ref_payload_effect",       gen_ref_payload_effect),
    ("record_double_move",       gen_record_double_move),
    ("missing_effect_arm",       gen_missing_effect_arm_in_handler),
    ("cart_non_native_effect",   gen_cart_non_native_effect),
    ("cart_graphics_no_host",    gen_cart_graphics_without_host),
    ("field_double_move",        gen_field_double_move),
    ("ref_escape_nested_effect", gen_ref_escape_nested_region_effect),
    ("bad_syntax",               gen_bad_syntax),
];

// Moving a value invalidates an outstanding named `&` borrow of it: using the
// borrow afterwards is rejected. Only named bindings (`let r = &x`) are tracked,
// so a discarded `&x` temporary never blocks a later move.
#[test]
fn rejection_borrow_after_base_moved() {
    let src = r#"
fn consume(s: String) -> Int { 1 }
fn rlen(s: &String) -> Int { 2 }
fn main() -> Int { let x = "m"; let r = &x; let _ = consume(x); rlen(r) }
"#;
    assert!(is_rejected(src), "borrow-after-move must be rejected");
}

#[test]
fn rejection_ref_payload_escapes_via_effect() {
    let src = r#"
effect E { op send(r: &Int) -> Int }
fn body() -> <E> Int { region { let a = 5; E.send(&a) } }
fn main() -> Int { handle body() { return v => v, E.send q => resume(*q) } }
"#;
    assert!(is_rejected(src), "&T escaping region via effect payload must be rejected");
}

#[test]
fn rejection_fallible_call_value_without_question() {
    let src = r#"
fn inner() -> <exn<Int>> Int { 5 }
fn mid() -> <exn<Int>> Int { let x = inner(); x + 1 }
fn main() -> Int { handle mid() { return v => v, exn _ => 0 } }
"#;
    assert!(is_rejected(src), "fallible call used as a value without `?` must be rejected");
}

#[test]
fn rejection_mixed_fallible_plain_if_tail() {
    let src = r#"
fn inner() -> <exn<Int>> Int { 5 }
fn f(c: Bool) -> <exn<Int>> Int { if c { inner() } else { 7 } }
fn main() -> Int { handle f(true) { return v => v, exn _ => 0 } }
"#;
    assert!(is_rejected(src), "mixed fallible/plain `if` branches in a fallible tail must be rejected");
}

#[test]
fn rejection_mut_borrow_of_scalar() {
    let var = r#"
fn bump(p: &mut Int) -> Unit { }
fn main() -> Int { let mut x = 1; bump(&mut x); x }
"#;
    let field = r#"
type W = { n: Int }
fn bump(p: &mut Int) -> Unit { }
fn main() -> Int { let mut w = W { n: 1 }; bump(&mut w.n); w.n }
"#;
    assert!(is_rejected(var), "`&mut` of a scalar variable must be rejected (no stable address)");
    assert!(is_rejected(field), "`&mut` of a scalar field must be rejected (no stable address)");
}

#[test]
fn rejection_mut_borrow_field_through_immutable_binding() {
    let src = r#"
type S = { n: Int }
type W = { s: S }
fn bump(s: &mut S) -> Unit { s.n = s.n + 1 }
fn main() -> Int { let w = W { s: S { n: 0 } }; bump(&mut w.s); w.s.n }
"#;
    assert!(is_rejected(src), "`&mut` projection through an immutable binding must be rejected");
}

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
