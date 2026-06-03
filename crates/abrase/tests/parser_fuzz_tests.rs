// Parser fuzz. Two strategies, both deterministic per seed, 0 deps.
//   byte-fuzz: random printable-ASCII source → Lexer → Parser.
//   token-fuzz: random concat of grammar-vocabulary tokens.

use abrase::lexer::Lexer;
use abrase::parser::Parser;
use std::sync::mpsc;
use std::time::Duration;

struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self { Self(seed.wrapping_mul(6364136223846793005).wrapping_add(1)) }
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0
    }
    fn pick(&mut self, n: u64) -> u64 { self.next() % n }
}

fn gen_bytes(seed: u64, len: usize) -> String {
    let mut r = Rng::new(seed);
    let mut s = String::with_capacity(len);
    for _ in 0..len {
        let c = (r.pick(95) as u8) + 32;
        s.push(c as char);
        if r.pick(20) == 0 { s.push('\n'); }
    }
    s
}

const TOKENS: &[&str] = &[
    "fn", "let", "mut", "const", "static", "if", "else", "match", "loop", "while", "for", "in",
    "region", "handle", "resume", "break", "continue", "return", "type", "pub", "import",
    "true", "false",
    "Int", "Float", "Bool", "Char", "String", "Unit", "Shared",
    "(", ")", "{", "}", "[", "]", "<", ">",
    ",", ";", ":", "::", "->", "=>", "=", "==", "!=", "<=", ">=", "&&", "||",
    "+", "-", "*", "/", "%", "!", "&", "&mut", ".", "..", "..=", "|", "?",
    "x", "y", "z", "a", "b", "n", "v0", "v1", "f", "g", "R", "T",
    "0", "1", "42", "-1", "100", "3.14", "0.0",
    "\"hi\"", "\"\"", "'a'", "'\\n'",
];

fn gen_tokens(seed: u64, count: usize) -> String {
    let mut r = Rng::new(seed);
    let mut s = String::new();
    for i in 0..count {
        if i > 0 { s.push(' '); }
        s.push_str(TOKENS[r.pick(TOKENS.len() as u64) as usize]);
        if r.pick(8) == 0 { s.push('\n'); }
    }
    s
}

struct ParseRes { decls: usize, errors: usize }

enum Outcome { Ok(ParseRes), Panic(String), Hang }

fn parse_one(src: String) -> Outcome {
    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut p = Parser::new(Lexer::new(&src)).with_source(src.clone());
        let decls = p.parse_program();
        ParseRes { decls: decls.len(), errors: p.errors.len() }
    }));
    match res {
        Ok(r) => Outcome::Ok(r),
        Err(e) => {
            let msg = if let Some(s) = e.downcast_ref::<&str>() { s.to_string() }
                      else if let Some(s) = e.downcast_ref::<String>() { s.clone() }
                      else { "<non-string panic>".into() };
            Outcome::Panic(msg)
        }
    }
}

fn run_with_timeout(src: String, timeout: Duration) -> Outcome {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || { let _ = tx.send(parse_one(src)); });
    match rx.recv_timeout(timeout) {
        Ok(o) => o,
        Err(_) => Outcome::Hang,
    }
}

#[derive(Default)]
struct Bucket { count: u64, examples: Vec<(u64, String, String)> }
impl Bucket {
    fn record(&mut self, seed: u64, detail: String, src: String, cap: usize) {
        self.count += 1;
        if self.examples.len() < cap { self.examples.push((seed, detail, src)); }
    }
}

#[derive(Default)]
struct Cov {
    total: u64,
    engaged: u64,        // parser produced >= 1 decl OR reported >= 1 error
    nonempty_decls: u64, // parser produced >= 1 decl (rare for random input)
    error_free: u64,     // parser produced >= 1 decl AND reported 0 errors
    total_decls: u64,
    total_errors: u64,
}

fn truncate(src: &str, n: usize) -> String {
    if src.len() <= n { src.to_string() } else { format!("{}...<+{} bytes>", &src[..n], src.len() - n) }
}

fn run_strategy(
    name: &str, iter: u64, timeout_ms: u64,
    min_engaged_pct: u64,   // assert this % of seeds caused parser to walk (decls or errors > 0)
    gen_fn: impl Fn(u64) -> String,
) {
    let mut panics = Bucket::default();
    let mut hangs = Bucket::default();
    let mut cov = Cov::default();
    let timeout = Duration::from_millis(timeout_ms);
    for seed in 0..iter {
        cov.total += 1;
        let src = gen_fn(seed);
        match run_with_timeout(src.clone(), timeout) {
            Outcome::Ok(r) => {
                if r.decls > 0 { cov.nonempty_decls += 1; }
                if r.decls > 0 && r.errors == 0 { cov.error_free += 1; }
                if r.decls > 0 || r.errors > 0 { cov.engaged += 1; }
                cov.total_decls += r.decls as u64;
                cov.total_errors += r.errors as u64;
            }
            Outcome::Panic(msg) => panics.record(seed, msg, truncate(&src, 200), 3),
            Outcome::Hang => hangs.record(seed, format!(">{}ms", timeout_ms), truncate(&src, 200), 3),
        }
    }
    let engaged_pct = cov.engaged * 100 / cov.total.max(1);
    let nonempty_pct = cov.nonempty_decls * 100 / cov.total.max(1);
    let avg_decls = cov.total_decls as f64 / cov.total.max(1) as f64;
    let avg_errors = cov.total_errors as f64 / cov.total.max(1) as f64;
    eprintln!(
        "\n[{}] iter={} panics={} hangs={} | engaged={}% nonempty_decls={}% avg_decls={:.2} avg_errors={:.1}",
        name, iter, panics.count, hangs.count,
        engaged_pct, nonempty_pct, avg_decls, avg_errors,
    );
    let report = |label: &str, b: &Bucket| {
        if b.count == 0 { return String::new(); }
        let mut s = format!("\n=== {} {} ({} total) ===\n", name, label, b.count);
        for (seed, detail, src) in &b.examples {
            s.push_str(&format!("--- seed={} {} ---\n{}\n", seed, detail, src));
        }
        s
    };
    let body = format!("{}{}", report("PANIC", &panics), report("HANG", &hangs));
    if !body.is_empty() { eprintln!("{}", body); }
    assert!(
        engaged_pct >= min_engaged_pct,
        "[{}] coverage too low: only {}% of seeds engaged parser (want >={}%)",
        name, engaged_pct, min_engaged_pct,
    );
    assert!(
        panics.count == 0 && hangs.count == 0,
        "[{}] panics={} hangs={} (see stderr report)",
        name, panics.count, hangs.count
    );
}

#[test]
fn parser_fuzz_random_bytes() {
    // Engaged = parser got into top-level recovery at least once.
    run_strategy("byte", 2_000, 100, 80, |seed| {
        let len = ((Rng::new(seed ^ 0xBEEF).pick(400) + 20) as usize).min(500);
        gen_bytes(seed, len)
    });
}

#[test]
fn parser_fuzz_random_tokens() {
    run_strategy("token", 2_000, 100, 95, |seed| {
        let count = ((Rng::new(seed ^ 0xCAFE).pick(80) + 5) as usize).min(120);
        gen_tokens(seed, count)
    });
}

fn gen_closure_position(seed: u64) -> String {
    let mut r = Rng::new(seed ^ 0xC105);
    let mv = if r.pick(2) == 0 { "move " } else { "" };
    let ann = if r.pick(2) == 0 { ": Int" } else { "" };
    let body = match r.pick(3) {
        0 => "x + base",
        1 => "x * 2",
        _ => "x",
    };
    let clo = format!("{mv}|x{ann}| {body}");
    match r.pick(4) {
        0 => format!("fn main() -> Int {{ let base = 1; let f = {clo}; f(2) }}"),
        1 => format!("fn main() -> Int {{ let base = 1; {{ let _t = 0; {clo} }}; 0 }}"),
        2 => format!("fn mk(base: Int) -> (Int) -> Int {{ let _t = base; {clo} }}\nfn main() -> Int {{ 0 }}"),
        _ => format!("fn main() -> Int {{ let g = region {{ let base = 1; {clo} }}; 0 }}"),
    }
}

#[test]
fn parser_fuzz_closure_tail_positions() {
    let mut bad: Vec<(u64, usize, String)> = Vec::new();
    for seed in 0..500u64 {
        let src = gen_closure_position(seed);
        if let Outcome::Ok(r) = parse_one(src.clone()) {
            if r.errors != 0 { bad.push((seed, r.errors, src)); }
        } else {
            bad.push((seed, 0, src));
        }
    }
    if !bad.is_empty() {
        for (seed, errs, src) in bad.iter().take(8) {
            eprintln!("--- seed={} errors={} ---\n{}", seed, errs, src);
        }
        panic!("parser_fuzz_closure_tail_positions: {} program(s) failed to parse", bad.len());
    }
}

struct GRng(u64);
impl GRng {
    fn new(seed: u64) -> Self { Self(seed.wrapping_mul(6364136223846793005).wrapping_add(1)) }
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0
    }
    fn pick(&mut self, n: u64) -> u64 { self.next() % n.max(1) }
}

fn g_leaf(r: &mut GRng) -> String {
    match r.pick(6) {
        0 => "0".into(), 1 => "1".into(), 2 => "42".into(),
        3 => "x".into(), 4 => "n".into(), _ => "i".into(),
    }
}

fn g_expr(r: &mut GRng, d: u32) -> String {
    if d == 0 { return g_leaf(r); }
    match r.pick(8) {
        0 => g_leaf(r),
        1 => format!("({} {} {})", g_expr(r, d-1),
                ["+","-","*","%"][r.pick(4) as usize], g_expr(r, d-1)),
        2 => g_block(r, d-1),
        3 => format!("if {} {{ {} }} else {{ {} }}",
                g_expr(r, d-1), g_expr(r, d-1), g_expr(r, d-1)),
        4 => format!("match {} {{ 0..2 => {}, _ if {} => {}, _ => {} }}",
                g_expr(r, d-1), g_expr(r, d-1), g_expr(r, d-1), g_expr(r, d-1), g_expr(r, d-1)),
        5 => format!("loop {{ {} break {} }}", g_stmts(r, d-1), g_expr(r, d-1)),
        6 => format!("region {{ {} }}", g_block_inner(r, d-1)),
        _ => format!("f({})", g_expr(r, d-1)),
    }
}

fn g_range(r: &mut GRng) -> String {
    let hi = r.pick(6) + 1;
    if r.pick(2) == 0 { format!("0..{}", hi) } else { format!("0..={}", hi) }
}

fn g_stmt(r: &mut GRng, d: u32) -> String {
    match r.pick(6) {
        0 => format!("let y{} = {};", r.pick(1000), g_expr(r, d)),
        1 => format!("x = {};", g_expr(r, d)),
        2 => format!("while {} {{ {} }};", g_expr(r, d), g_stmts(r, d.saturating_sub(1))),
        3 => format!("for k in {} {{ {} }};", g_range(r), g_stmts(r, d.saturating_sub(1))),
        4 => format!("if {} {{ {} }};", g_expr(r, d), g_stmts(r, d.saturating_sub(1))),
        _ => format!("{};", g_expr(r, d)),
    }
}

fn g_stmts(r: &mut GRng, d: u32) -> String {
    let n = r.pick(3);
    (0..n).map(|_| g_stmt(r, d)).collect::<Vec<_>>().join(" ") + " "
}

fn g_block_inner(r: &mut GRng, d: u32) -> String {
    format!("{}{}", g_stmts(r, d), g_expr(r, d))
}

fn g_block(r: &mut GRng, d: u32) -> String {
    format!("{{ {} }}", g_block_inner(r, d))
}

fn gen_grammar(seed: u64) -> String {
    let mut r = GRng::new(seed);
    let depth = 2 + (seed % 3) as u32;
    let helper = "fn f(n: Int) -> Int { if n <= 0 { 0 } else { n + f(n - 1) } }\n";
    // Occasionally wrap a helper call in an effect handler.
    if r.pick(4) == 0 {
        format!(
            "{helper}effect E {{ op ask(q: Int) -> Int }}\n\
             fn g() -> <E> Int {{ E.ask({}) }}\n\
             fn main() -> Int {{ let x = 1; let i = 2; handle g() {{ return v => v, E.ask q => resume(({} + q)) }} }}\n",
            g_expr(&mut r, 1), g_leaf(&mut r))
    } else {
        format!("{helper}fn main() -> Int {{ let x = 1; let n = 2; let i = 3; {} }}\n",
            g_block_inner(&mut r, depth))
    }
}

#[test]
fn parser_fuzz_grammar_nested() {
    let mut bad: Vec<(u64, usize, String)> = Vec::new();
    for seed in 0..3_000u64 {
        let src = gen_grammar(seed);
        match run_with_timeout(src.clone(), Duration::from_secs(2)) {
            Outcome::Ok(r) if r.errors == 0 => {}
            Outcome::Ok(r) => bad.push((seed, r.errors, src)),
            Outcome::Panic(m) => bad.push((seed, 0, format!("PANIC {}\n{}", m, src))),
            Outcome::Hang => bad.push((seed, 0, format!("HANG\n{}", src))),
        }
    }
    if !bad.is_empty() {
        for (seed, errs, src) in bad.iter().take(8) {
            eprintln!("--- seed={} errors={} ---\n{}", seed, errs, truncate(src, 400));
        }
        panic!("parser_fuzz_grammar_nested: {}/3000 programs failed to parse cleanly", bad.len());
    }
}

fn gen_resume_index_block(seed: u64) -> String {
    let mut r = Rng::new(seed ^ 0x9111);
    let block = match r.pick(5) {
        0 => "if 1 { 1 } else { 2 }",
        1 => "match 0 { _ => 1 }",
        2 => "{ 1 }",
        3 => "loop { break 1 }",
        _ => "region { 1 }",
    };
    if r.pick(2) == 0 {
        format!(
            "effect E {{ op e() -> Int }}\nfn g() -> <E> Int {{ E.e() }}\n\
             fn main() -> Int {{ handle g() {{ return v => v, E.e => resume({}) }} }}\n", block)
    } else {
        format!("fn main() -> Int {{ let a = [1, 2, 3]; a[{}] }}\n", block)
    }
}

#[test]
fn parser_fuzz_resume_index_block_args() {
    let mut bad: Vec<(u64, usize, String)> = Vec::new();
    for seed in 0..400u64 {
        let src = gen_resume_index_block(seed);
        match parse_one(src.clone()) {
            Outcome::Ok(r) if r.errors == 0 => {}
            Outcome::Ok(r) => bad.push((seed, r.errors, src)),
            other => bad.push((seed, 0, format!("{:?}\n{}",
                matches!(other, Outcome::Panic(_)), src))),
        }
    }
    if !bad.is_empty() {
        for (seed, errs, src) in bad.iter().take(6) {
            eprintln!("--- seed={} errors={} ---\n{}", seed, errs, src);
        }
        panic!("parser_fuzz_resume_index_block_args: {}/400 failed to parse", bad.len());
    }
}
