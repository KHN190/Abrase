// Acceptance fuzz for region escape on stores: random sequences of "allocate a
// fresh heap value inside some region and store it into an outer container",
// modelled in Rust. Every program must return the modelled value with
// heap_live_count == 0, with inlining on and off.

use abrase::compiler::Compiler;
use abrase::lexer::Lexer;
use abrase::parser::Parser;
use myriad::VirtualMachine;

const ITER: u64 = 400;

struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self { Self(seed.wrapping_mul(6364136223846793005).wrapping_add(1)) }
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0
    }
    fn pick(&mut self, n: u64) -> u64 { self.next() % n }
}

#[derive(Default)]
struct Model {
    wa: (i64, i64),
    wp: i64,
    ws_len: i64,
    x: (i64, i64),
    xs0: (i64, i64),
}

impl Model {
    fn result(&self) -> i64 {
        self.wa.1 + self.wp + self.ws_len + self.x.0 + self.xs0.1
    }
}

struct Gen {
    rng: Rng,
    helpers: String,
    body: String,
    model: Model,
    next_fn: u32,
}

impl Gen {
    fn new(seed: u64) -> Self {
        Self { rng: Rng::new(seed), helpers: String::new(), body: String::new(), model: Model::default(), next_fn: 0 }
    }

    fn val(&mut self) -> i64 { self.rng.pick(50) as i64 + 1 }

    fn field_store(&mut self, target: &str) -> String {
        match self.rng.pick(3) {
            0 => { let a = self.val(); let b = self.val(); self.model.wa = (a, b); format!("{}.a = [{}, {}]", target, a, b) }
            1 => { let n = self.val(); self.model.wp = n; format!("{}.p = B({})", target, n) }
            _ => { let n = (self.rng.pick(4) + 1) as usize; self.model.ws_len = n as i64; format!("{}.s = [\"{}\"]", target, "z".repeat(n)) }
        }
    }

    fn local_store(&mut self) -> String {
        match self.rng.pick(2) {
            0 => { let a = self.val(); let b = self.val(); self.model.x = (a, b); format!("x = [{}, {}]", a, b) }
            _ => { let a = self.val(); let b = self.val(); self.model.xs0 = (a, b); format!("xs[0] = [{}, {}]", a, b) }
        }
    }

    fn store(&mut self) -> String {
        if self.rng.pick(2) == 0 { self.field_store("w") } else { self.local_store() }
    }

    fn mut_ref_call(&mut self) -> String {
        let name = format!("f{}", self.next_fn);
        self.next_fn += 1;
        let st = self.field_store("w");
        let inlinable = self.rng.pick(2) == 0;
        if inlinable {
            self.helpers.push_str(&format!("fn {}(w: &mut W) -> Unit {{ {} }}\n", name, st));
        } else {
            self.helpers.push_str(&format!("fn {}(w: &mut W) -> Unit {{ let k = 0; {} }}\n", name, st));
        }
        format!("{}(&mut w)", name)
    }

    fn action(&mut self) -> String {
        if self.rng.pick(3) == 0 { self.mut_ref_call() } else { self.store() }
    }

    fn step(&mut self) {
        let kind = self.rng.pick(6);
        let n = self.rng.pick(3) + 1;
        let line = match kind {
            0 => { let a = self.action(); format!("{};", a) }
            1 => { let a = self.action(); format!("for i in 0..{} {{ {} }};", n, a) }
            2 => { let a = self.action(); let b = self.action(); format!("for i in 0..{} {{ {}; {} }};", n, a, b) }
            3 => { let a = self.action(); format!("{{ {}; }};", a) }
            4 => { let a = self.action(); format!("let mut c = 0; while c < {} {{ {}; c = c + 1 }};", n, a) }
            _ => { let a = self.field_store("r"); format!("for i in 0..{} {{ let r = &mut w; {} }};", n, a) }
        };
        self.body.push_str("    ");
        self.body.push_str(&line);
        self.body.push('\n');
    }

    fn program(mut self) -> (String, i64) {
        let steps = self.rng.pick(5) + 1;
        for _ in 0..steps { self.step(); }
        let src = format!(
            "type P = | A | B(Int)\ntype W = {{ a: Array<Int>, p: P, s: Array<String> }}\n{}\
fn main() -> Int {{\n    let mut w = W {{ a: [0, 0], p: A, s: [\"\"] }};\n    let mut x = [0, 0];\n    let mut xs = [[0, 0]];\n{}\
    let q = w.p;\n    let pn = match q {{ B(n) => n, _ => 0 }};\n    w.a[1] + pn + w.s[0].len() + x[0] + xs[0][1]\n}}\n",
            self.helpers, self.body
        );
        (src, self.model.result())
    }
}

fn run(src: &str, inline: bool) -> Result<(i64, usize), String> {
    let mut p = Parser::new(Lexer::new(src)).with_source(src.to_string());
    let ast = p.parse_program();
    if !p.errors.is_empty() { return Err(format!("parse:\n{}", p.pretty_print_errors())); }
    let mut c = Compiler::new().with_source(src.to_string()).with_inline(inline);
    let module = c.compile_module(&ast).map_err(|_| c.pretty_print_errors())?;
    let mut vm = VirtualMachine::new().with_step_cap(2_000_000);
    let v = vm.run_module(&module)?;
    Ok((v.as_int(), vm.heap_live_count()))
}

#[test]
fn fuzz_region_stores_return_model_value_with_zero_live_heap() {
    let mut n_fail = 0u32;
    let mut report = String::new();
    for seed in 0..ITER {
        let (src, expected) = Gen::new(seed).program();
        for inline in [false, true] {
            match run(&src, inline) {
                Ok((v, 0)) if v == expected => {}
                other => {
                    n_fail += 1;
                    if report.len() < 6000 {
                        report.push_str(&format!("--- seed={} inline={} expected={} got={:?} ---\n{}\n", seed, inline, expected, other, src));
                    }
                }
            }
        }
    }
    assert_eq!(n_fail, 0, "{} failing run(s):\n{}", n_fail, report);
}

#[test]
fn fuzz_generator_programs_typecheck() {
    let mut bad = 0u32;
    let mut report = String::new();
    for seed in 0..ITER {
        let (src, _) = Gen::new(seed).program();
        let mut p = Parser::new(Lexer::new(&src)).with_source(src.clone());
        let ast = p.parse_program();
        let mut c = Compiler::new().with_source(src.clone());
        if !p.errors.is_empty() || c.compile_module(&ast).is_err() {
            bad += 1;
            if report.len() < 3000 {
                report.push_str(&format!("--- seed={} ---\n{}\n{}\n", seed, p.pretty_print_errors(), c.pretty_print_errors()));
            }
        }
    }
    assert_eq!(bad, 0, "{} generated program(s) rejected by the frontend:\n{}", bad, report);
}
