// Linter fuzz — deterministic, zero extra deps.
//
// Properties verified per iteration:
//   P1. No panic or hang on any grammar-generated input
//   P2. unused_variable: warned name has 0 uses in the fn body (AST walk)
//   P3. unused_mut:      warned name has no write in the fn body (AST walk)
//   P4. No false positive for string interpolation ("{var}")
//   P5. No false positive for field assignment (var.field = ...)
//   P6. No false positive for closure capture (|| var)
//   P7. No false positive for nested-block use ({ var })
//   P8. Coverage: ≥35% of structured programs trigger ≥1 warning

use abrase::ast::{BinaryOp, Block, Decl, Expr, FnDecl, Stmt};
use abrase::compiler::liveness::count_uses;
use abrase::lexer::Lexer;
use abrase::parser::Parser;
use abrase::typeck::Checker;
use std::sync::mpsc;
use std::time::Duration;

struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(6364136223846793005).wrapping_add(1))
    }
    fn next(&mut self) -> u64 {
        self.0 = self.0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }
    fn pick(&mut self, n: u64) -> u64 { self.next() % n.max(1) }
}

fn count_uses_of(block: &Block, name: &str) -> usize {
    count_uses(block).get(name).copied().unwrap_or(0)
}

fn is_assign_op(op: &BinaryOp) -> bool {
    matches!(op,
        BinaryOp::Assign | BinaryOp::AddAssign | BinaryOp::SubAssign
        | BinaryOp::MulAssign | BinaryOp::DivAssign | BinaryOp::ModAssign)
}

fn lhs_root(expr: &Expr) -> Option<&str> {
    match expr {
        Expr::Identifier(n)          => Some(n),
        Expr::FieldAccess { base, .. }
        | Expr::Index { base, .. }   => lhs_root(&base.node),
        _                            => None,
    }
}

fn writes_in_block(block: &Block, name: &str) -> bool {
    block.stmts.iter().any(|s| writes_in_stmt(&s.node, name))
    || block.ret.as_ref().map_or(false, |r| writes_in_expr(&r.node, name))
}

fn writes_in_stmt(stmt: &Stmt, name: &str) -> bool {
    match stmt {
        Stmt::Expr(e)           => writes_in_expr(&e.node, name),
        Stmt::Let { value, .. } => writes_in_expr(&value.node, name),
        Stmt::Empty             => false,
    }
}

fn writes_in_expr(expr: &Expr, name: &str) -> bool {
    match expr {
        Expr::Binary { op, left, right } => {
            (is_assign_op(op) && lhs_root(&left.node) == Some(name))
            || writes_in_expr(&left.node, name)
            || writes_in_expr(&right.node, name)
        }
        Expr::Unary { right, .. } => writes_in_expr(&right.node, name),
        Expr::Call { callee, args } =>
            writes_in_expr(&callee.node, name)
            || args.iter().any(|a| writes_in_expr(&a.node, name)),
        Expr::Index { base, index } =>
            writes_in_expr(&base.node, name) || writes_in_expr(&index.node, name),
        Expr::FieldAccess { base, .. } => writes_in_expr(&base.node, name),
        Expr::Block(b) => writes_in_block(b, name),
        Expr::If { condition, consequence, alternative } =>
            writes_in_expr(&condition.node, name)
            || writes_in_expr(&consequence.node, name)
            || alternative.as_ref().map_or(false, |a| writes_in_expr(&a.node, name)),
        Expr::While { condition, body } =>
            writes_in_expr(&condition.node, name) || writes_in_block(body, name),
        Expr::For { iter, body, .. } =>
            writes_in_expr(&iter.node, name) || writes_in_block(body, name),
        Expr::Loop { body } => writes_in_block(body, name),
        Expr::Match { scrutinee, arms } =>
            writes_in_expr(&scrutinee.node, name)
            || arms.iter().any(|arm| writes_in_expr(&arm.body.node, name)),
        Expr::Closure { body, .. } => writes_in_expr(&body.node, name),
        Expr::Return(Some(e)) | Expr::Break(Some(e)) | Expr::Throw(e)
        | Expr::Paren(e) =>
            writes_in_expr(&e.node, name),
        Expr::Tuple(es) | Expr::Array(es) =>
            es.iter().any(|e| writes_in_expr(&e.node, name)),
        Expr::Region { body, .. } => writes_in_block(body, name),
        Expr::Handle { expr, arms } =>
            writes_in_expr(&expr.node, name)
            || arms.iter().any(|a| writes_in_expr(&a.body.node, name)),
        _ => false,
    }
}

fn extract_name(msg: &str) -> Option<String> {
    let start = msg.find('`')? + 1;
    let end = start + msg[start..].find('`')?;
    Some(msg[start..end].to_string())
}

struct LintOutcome {
    panicked: bool,
    warnings: Vec<abrase::lint::Lint>,
    ast: Vec<Decl>,
    parse_ok: bool,
}

fn run_linter(src: &str) -> LintOutcome {
    let src = src.to_string();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut p = Parser::new(Lexer::new(&src)).with_source(src.clone());
        let ast = p.parse_program();
        let ok = p.errors.is_empty();
        let mut checker = Checker::new();
        checker.check_program(&ast);
        (ast, checker.warnings, ok)
    }));
    match result {
        Ok((ast, warnings, ok)) => LintOutcome { panicked: false, warnings, ast, parse_ok: ok },
        Err(_) => LintOutcome { panicked: true, warnings: vec![], ast: vec![], parse_ok: false },
    }
}

fn run_linter_timeout(src: &str, ms: u64) -> Option<LintOutcome> {
    let src = src.to_string();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || { let _ = tx.send(run_linter(&src)); });
    rx.recv_timeout(Duration::from_millis(ms)).ok()
}

// Returns true if `name` is bound via `let` (not param) anywhere in `block` (recursive).
fn is_let_bound(block: &Block, name: &str) -> bool {
    block.stmts.iter().any(|s| is_let_bound_stmt(&s.node, name))
    || block.ret.as_ref().map_or(false, |r| is_let_bound_expr(&r.node, name))
}

fn is_let_bound_stmt(stmt: &Stmt, name: &str) -> bool {
    match stmt {
        Stmt::Let { pattern, value, .. } => {
            (if let abrase::ast::Pattern::Bind(n) = &pattern.node { n == name } else { false })
            || is_let_bound_expr(&value.node, name)
        }
        Stmt::Expr(e) => is_let_bound_expr(&e.node, name),
        Stmt::Empty => false,
    }
}

fn is_let_bound_expr(expr: &Expr, name: &str) -> bool {
    match expr {
        Expr::Block(b) => is_let_bound(b, name),
        Expr::If { consequence, alternative, .. } =>
            is_let_bound_expr(&consequence.node, name)
            || alternative.as_ref().map_or(false, |a| is_let_bound_expr(&a.node, name)),
        Expr::While { body, .. } | Expr::Loop { body } => is_let_bound(body, name),
        Expr::For { body, .. } => is_let_bound(body, name),
        Expr::Match { arms, .. } => arms.iter().any(|arm| is_let_bound_expr(&arm.body.node, name)),
        Expr::Closure { body, .. } => is_let_bound_expr(&body.node, name),
        Expr::Region { body, .. } => is_let_bound(body, name),
        Expr::Handle { arms, .. } => arms.iter().any(|arm| is_let_bound_expr(&arm.body.node, name)),
        _ => false,
    }
}

// Returns true if `name` is a parameter of `f`.
fn is_param(f: &FnDecl, name: &str) -> bool {
    f.params.iter().any(|p| {
        if let abrase::ast::Param::Named { pattern, .. } = p {
            if let abrase::ast::Pattern::Bind(n) = &pattern.node { n == name } else { false }
        } else { false }
    })
}

fn verify_correctness(outcome: &LintOutcome, src: &str) -> Vec<String> {
    let mut failures = vec![];
    if !outcome.parse_ok { return failures; }

    for w in &outcome.warnings {
        let name = match extract_name(&w.message) {
            Some(n) => n,
            None => continue,
        };
        if name.starts_with('_') { continue; }

        let fns: Vec<&FnDecl> = outcome.ast.iter().filter_map(|d| {
            if let Decl::Fn(f) = d { Some(f) } else { None }
        }).collect();

        match w.code {
            "unused_variable" => {
                // False positive = there is NO fn where `name` is bound AND has 0 uses.
                let any_legitimately_unused = fns.iter().any(|f| {
                    (is_let_bound(&f.body, &name) || is_param(f, &name))
                    && count_uses_of(&f.body, &name) == 0
                });
                if !any_legitimately_unused {
                    failures.push(format!(
                        "P2 false-positive: unused_variable `{}` but every binding has uses > 0\nsrc:\n{}",
                        name, truncate(src, 300)
                    ));
                }
            }
            "unused_mut" => {
                for f in &fns {
                    if !is_let_bound(&f.body, &name) { continue; }
                    if writes_in_block(&f.body, &name) {
                        failures.push(format!(
                            "P3 false-positive: unused_mut `{}` in fn `{}` but has a write\nsrc:\n{}",
                            name, f.name, truncate(src, 300)
                        ));
                    }
                }
            }
            _ => {}
        }
    }
    failures
}

fn truncate(s: &str, n: usize) -> String {
    if s.len() <= n { s.into() } else { format!("{}…", &s[..n]) }
}

const VARS: &[&str] = &["a", "b", "c", "d", "e", "f0", "g0"];

fn gen_structured(seed: u64) -> String {
    let mut r = Rng::new(seed);
    let nv = 2 + r.pick(4) as usize;
    let mut stmts = String::new();
    let mut tail = "0".to_string();

    for i in 0..nv {
        let v = VARS[i % VARS.len()];
        match r.pick(10) {
            0 => {
                // genuinely unused — should warn
                stmts.push_str(&format!("let {v} = 1;\n"));
            }
            1 => {
                // used as return value — no warn
                stmts.push_str(&format!("let {v} = 1;\n"));
                tail = v.to_string();
            }
            2 => {
                // used in string interpolation — no warn (P4)
                stmts.push_str(&format!("let {v} = 42;\nlet _s{i} = \"{{{v}}}\";\n"));
            }
            3 => {
                // used in binary expr — no warn
                stmts.push_str(&format!("let {v} = 3;\nlet _q{i} = {v} + 1;\n"));
            }
            4 => {
                // used in closure body — no warn (P6)
                stmts.push_str(&format!("let {v} = 1;\nlet _cl{i} = || {v};\n"));
            }
            5 => {
                // used in if condition — no warn
                stmts.push_str(&format!("let {v} = 0;\nlet _r{i} = if {v} == 0 {{ 1 }} else {{ 0 }};\n"));
            }
            6 => {
                // mut + write + use — no unused_mut warn, no unused_variable warn
                stmts.push_str(&format!("let mut {v} = 0;\n{v} = 1;\n"));
                tail = v.to_string();
            }
            7 => {
                // mut but never written — should warn unused_mut
                stmts.push_str(&format!("let mut {v} = 0;\n"));
                tail = v.to_string();
            }
            8 => {
                // used inside nested block — no warn (P7)
                stmts.push_str(&format!("let {v} = 5;\nlet _nb{i} = {{ {v} }};\n"));
            }
            _ => {
                // used in while condition — no warn
                stmts.push_str(&format!("let {v} = 0;\nwhile {v} > 10 {{ {v} = {v} - 1; }};\n"));
            }
        }
    }
    format!("fn main() -> Int {{\n{stmts}{tail}\n}}\n")
}

fn fp_cases() -> Vec<(&'static str, String)> {
    vec![
        ("string_interp",
            "fn main() -> Int { let x = 42; let _s = \"{x}\"; 0 }".into()),
        ("string_interp_field",
            "fn main() -> Int { let x = 42; let _s = \"val={x}\"; 0 }".into()),
        ("field_assign_mut",
            "type Pt = { x: Int, y: Int }\nfn main() -> Int { let mut p = Pt { x: 1, y: 2 }; p.x = 10; p.x }".into()),
        ("index_assign_mut",
            "fn main() -> Int { let mut a = [1, 2, 3]; a[0] = 99; a[0] }".into()),
        ("closure_capture",
            "fn main() -> Int { let x = 1; let f = || x; f() }".into()),
        ("nested_block",
            "fn main() -> Int { let x = 7; let _r = { x }; 0 }".into()),
        ("used_in_match_scrutinee",
            "fn main() -> Int { let x = 1; match x { 1 => 10, _ => 0 } }".into()),
        ("used_in_while_cond",
            "fn main() -> Int { let mut x = 3; while x > 0 { x = x - 1; }; 0 }".into()),
        ("used_in_for_iter",
            "fn main() -> Int { let xs = [1,2,3]; for _k in xs { }; 0 }".into()),
        ("used_via_field_access",
            "type T = { n: Int }\nfn main() -> Int { let t = T { n: 5 }; t.n }".into()),
        ("param_used_in_interp",
            "fn greet(name: String) -> String { \"hello {name}\" }\nfn main() -> Int { 0 }".into()),
        ("param_used_via_field",
            "type T = { n: Int }\nfn f(t: T) -> Int { t.n }\nfn main() -> Int { 0 }".into()),
    ]
}

fn gen_grammar_src(seed: u64) -> String {
    let mut r = Rng::new(seed);
    let depth = 2 + (seed % 3) as u32;
    let helper = "fn f(n: Int) -> Int { if n <= 0 { 0 } else { n + f(n - 1) } }\n";
    format!(
        "{helper}fn main() -> Int {{ let x = 1; let n = 2; let i = 3; {} }}\n",
        g_block_inner(&mut r, depth)
    )
}

fn g_leaf(r: &mut Rng) -> &'static str {
    ["0", "1", "x", "n", "i"][r.pick(5) as usize]
}

fn g_expr(r: &mut Rng, d: u32) -> String {
    if d == 0 { return g_leaf(r).into(); }
    match r.pick(7) {
        0 => g_leaf(r).into(),
        1 => format!("({} {} {})",
                g_expr(r, d-1), ["+","-","*","%"][r.pick(4) as usize], g_expr(r, d-1)),
        2 => format!("{{ {} }}", g_block_inner(r, d-1)),
        3 => format!("if {} {{ {} }} else {{ {} }}",
                g_expr(r, d-1), g_expr(r, d-1), g_expr(r, d-1)),
        4 => format!("match {} {{ 0 => {}, _ => {} }}",
                g_expr(r, d-1), g_expr(r, d-1), g_expr(r, d-1)),
        5 => format!("loop {{ {} break {} }}", g_stmts(r, d-1), g_expr(r, d-1)),
        _ => format!("f({})", g_expr(r, d-1)),
    }
}

fn g_stmt(r: &mut Rng, d: u32) -> String {
    match r.pick(5) {
        0 => format!("let y{} = {};", r.pick(100), g_expr(r, d)),
        1 => format!("x = {};", g_expr(r, d)),
        2 => format!("while {} {{ {} }};", g_expr(r, d), g_stmts(r, d.saturating_sub(1))),
        3 => format!("if {} {{ {} }};", g_expr(r, d), g_stmts(r, d.saturating_sub(1))),
        _ => format!("{};", g_expr(r, d)),
    }
}

fn g_stmts(r: &mut Rng, d: u32) -> String {
    let n = r.pick(3);
    (0..n).map(|_| g_stmt(r, d)).collect::<Vec<_>>().join(" ")
}

fn g_block_inner(r: &mut Rng, d: u32) -> String {
    format!("{} {}", g_stmts(r, d), g_expr(r, d))
}

#[test]
fn linter_fuzz_no_panic_grammar() {
    let mut panics: Vec<(u64, String)> = vec![];
    let mut hangs:  Vec<u64>           = vec![];

    for seed in 0..2_000u64 {
        let src = gen_grammar_src(seed);
        match run_linter_timeout(&src, 500) {
            None => hangs.push(seed),
            Some(o) if o.panicked => panics.push((seed, truncate(&src, 200))),
            _ => {}
        }
    }

    if !panics.is_empty() || !hangs.is_empty() {
        for (s, src) in panics.iter().take(5) {
            eprintln!("PANIC seed={s}\n{src}");
        }
        for s in hangs.iter().take(5) {
            eprintln!("HANG seed={s}");
        }
        panic!("linter_fuzz_no_panic_grammar: {} panics, {} hangs", panics.len(), hangs.len());
    }
}

#[test]
fn linter_fuzz_correctness_structured() {
    let mut failures: Vec<String> = vec![];
    let mut warned = 0u64;

    for seed in 0..3_000u64 {
        let src = gen_structured(seed);
        let outcome = run_linter(&src);
        assert!(!outcome.panicked, "panic on seed={seed}\n{src}");
        if !outcome.warnings.is_empty() { warned += 1; }
        let errs = verify_correctness(&outcome, &src);
        failures.extend(errs);
    }

    let pct = warned * 100 / 3_000;
    eprintln!("[structured] warned={}% (3000 seeds)", pct);

    if !failures.is_empty() {
        for f in failures.iter().take(5) { eprintln!("{f}"); }
        panic!("linter_fuzz_correctness_structured: {} correctness failures", failures.len());
    }
    assert!(pct >= 35, "coverage too low: only {}% triggered a warning (want ≥35%)", pct);
}

#[test]
fn linter_fuzz_no_false_positives() {
    for (label, src) in fp_cases() {
        let outcome = run_linter(&src);
        assert!(!outcome.panicked, "[{label}] panic\nsrc: {src}");
        assert!(outcome.parse_ok,  "[{label}] parse error\nsrc: {src}");

        let false_positives: Vec<_> = outcome.warnings.iter().filter(|w| {
            let name = match extract_name(&w.message) { Some(n) => n, None => return false };
            if name.starts_with('_') { return false; }
            w.code == "unused_variable" || w.code == "unused_mut"
        }).collect();

        assert!(
            false_positives.is_empty(),
            "[{label}] false positive warnings:\n{}\nsrc:\n{src}",
            false_positives.iter().map(|w| format!("  {}: {}", w.code, w.message)).collect::<Vec<_>>().join("\n"),
        );
    }
}

#[test]
fn linter_fuzz_correctness_grammar() {
    let mut failures: Vec<String> = vec![];

    for seed in 0..1_000u64 {
        let src = gen_grammar_src(seed);
        let outcome = run_linter(&src);
        assert!(!outcome.panicked, "panic seed={seed}");
        let errs = verify_correctness(&outcome, &src);
        failures.extend(errs);
    }

    if !failures.is_empty() {
        for f in failures.iter().take(5) { eprintln!("{f}"); }
        panic!("linter_fuzz_correctness_grammar: {} correctness failures", failures.len());
    }
}

#[test]
fn linter_fuzz_mutation_stability() {
    const BASES: &[&str] = &[
        "fn main() -> Int { let x = 1; let y = x + 2; y }",
        "fn main() -> Int { let mut n = 0; n = 5; n }",
        "fn main() -> Int { let s = \"hello\"; let _r = \"{s}\"; 0 }",
        "fn f(a: Int, b: Int) -> Int { a + b }\nfn main() -> Int { f(1, 2) }",
    ];

    let mut panics = 0u32;
    for (bi, base) in BASES.iter().enumerate() {
        for seed in 0..200u64 {
            let src = mutate(base, seed);
            let o = run_linter(&src);
            if o.panicked {
                panics += 1;
                eprintln!("PANIC base={bi} seed={seed}\n{}", truncate(&src, 200));
            }
        }
    }
    assert!(panics == 0, "linter_fuzz_mutation_stability: {panics} panics");
}

fn mutate(src: &str, seed: u64) -> String {
    let mut r = Rng::new(seed ^ 0xDEAD);
    let bytes: Vec<u8> = src.bytes().collect();
    let len = bytes.len();
    let mut out = bytes.clone();

    match r.pick(5) {
        0 => {
            // insert random printable char
            let pos = r.pick(len as u64 + 1) as usize;
            let ch = (32 + r.pick(95)) as u8;
            out.insert(pos, ch);
        }
        1 => {
            // delete a char
            if !out.is_empty() {
                let pos = r.pick(len as u64) as usize;
                out.remove(pos);
            }
        }
        2 => {
            // replace a char
            if !out.is_empty() {
                let pos = r.pick(len as u64) as usize;
                out[pos] = (32 + r.pick(95)) as u8;
            }
        }
        3 => {
            // duplicate a word
            let keywords = [" let ", " mut ", " fn ", " if ", " else ", " return "];
            let kw = keywords[r.pick(keywords.len() as u64) as usize];
            if let Some(pos) = src.find(kw) {
                out.splice(pos..pos, kw.bytes());
            }
        }
        _ => {
            // append a statement fragment
            let frags = ["; let _z = 0", "; 0", "; let mut _m = 1"];
            out.extend_from_slice(frags[r.pick(frags.len() as u64) as usize].as_bytes());
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}
