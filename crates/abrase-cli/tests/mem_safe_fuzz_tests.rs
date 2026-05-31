// memory-safety fuzz. Generate random small carts from a legal wiki-05 subset,
// compile, run, assert heap_live_count == 0 and main returns expected Int.
// 0 deps, no fixtures. Seeded LCG, deterministic per-seed.

use abrase::compiler::Compiler;
use abrase::lexer::Lexer;
use abrase::parser::Parser;
use myriad::VirtualMachine;

struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self { Self(seed.wrapping_mul(6364136223846793005).wrapping_add(1)) }
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0
    }
    fn pick(&mut self, n: u64) -> u64 { self.next() % n }
}

struct Gen {
    rng: Rng,
    next_var: u32,
    next_fn: u32,
    out: String,
    helpers: String,
    fuel: u32,
    has_record: bool,
    has_static: bool,
}

impl Gen {
    fn new(seed: u64) -> Self {
        Self {
            rng: Rng::new(seed), next_var: 0, next_fn: 0,
            out: String::new(), helpers: String::new(), fuel: 250,
            has_record: false, has_static: false,
        }
    }
    fn fresh(&mut self) -> String { let n = self.next_var; self.next_var += 1; format!("v{}", n) }
    fn fresh_fn(&mut self) -> String { let n = self.next_fn; self.next_fn += 1; format!("f{}", n) }
    fn push(&mut self, s: &str) { self.out.push_str(s); }

    fn gen_program(&mut self) -> String {
        // Optional borrow-taking helper.
        if self.rng.pick(2) == 0 {
            let name = self.fresh_fn();
            self.helpers.push_str(&format!("fn {}(x: &Int) -> Int {{ *x }}\n", name));
        }
        // Optional record type.
        if self.rng.pick(2) == 0 {
            self.helpers.push_str("type R = { a: Int, b: Int }\n");
            self.has_record = true;
        }
        // Optional static int.
        if self.rng.pick(2) == 0 {
            let v = self.rng.pick(100) as i64;
            self.helpers.push_str(&format!("static S: Int = {};\n", v));
            self.has_static = true;
        }
        self.push("fn main() -> Int {\n");
        self.gen_block(0, &mut Vec::new(), &mut Vec::new(), &mut Vec::new(), 0);
        self.push("  0\n}\n");
        format!("{}{}", std::mem::take(&mut self.helpers), std::mem::take(&mut self.out))
    }

    fn gen_block(
        &mut self,
        region_depth: u32,
        shareds: &mut Vec<String>,      // Shared<Int>
        ints: &mut Vec<String>,
        shared_recs: &mut Vec<String>,  // Shared<R>
        loop_depth: u32,
    ) {
        let stmts = (self.rng.pick(6) + 1) as usize;
        let s_snap = shareds.len();
        let i_snap = ints.len();
        let sr_snap = shared_recs.len();
        for _ in 0..stmts {
            if self.fuel == 0 { break; }
            self.fuel -= 1;
            let choices = if region_depth > 0 { 15 } else { 8 };
            match self.rng.pick(choices) {
                0 => {
                    let name = self.fresh();
                    let v = self.rng.pick(1000) as i64 - 500;
                    self.push(&format!("  let {}: Int = {};\n", name, v));
                    ints.push(name);
                }
                1 if !ints.is_empty() => {
                    let name = self.fresh();
                    let src = ints[self.rng.pick(ints.len() as u64) as usize].clone();
                    self.push(&format!("  let {}: Int = {};\n", name, src));
                    ints.push(name);
                }
                2 => {
                    self.push("  region {\n");
                    self.gen_block(region_depth + 1, shareds, ints, shared_recs, loop_depth);
                    self.push("  }\n");
                }
                3 if !ints.is_empty() && self.next_fn > 0 => {
                    let name = self.fresh();
                    let src = ints[self.rng.pick(ints.len() as u64) as usize].clone();
                    let fname = format!("f{}", self.rng.pick(self.next_fn as u64));
                    self.push(&format!("  let {}: Int = {}(&{});\n", name, fname, src));
                    ints.push(name);
                }
                4 if !ints.is_empty() => {
                    let name = self.fresh();
                    let a = ints[self.rng.pick(ints.len() as u64) as usize].clone();
                    let b = ints[self.rng.pick(ints.len() as u64) as usize].clone();
                    let op = ["+", "-", "*"][self.rng.pick(3) as usize];
                    self.push(&format!("  let {}: Int = {} {} {};\n", name, a, op, b));
                    ints.push(name);
                }
                5 if self.has_static => {
                    let name = self.fresh();
                    self.push(&format!("  let {}: Int = S;\n", name));
                    ints.push(name);
                }
                6 if region_depth > 0 => {
                    let name = self.fresh();
                    let v = self.rng.pick(1000) as i64;
                    self.push(&format!("  let {}: Shared<Int> = Shared({});\n", name, v));
                    shareds.push(name);
                }
                7 if region_depth > 0 && !ints.is_empty() => {
                    // Shared(int_var) — exercises St clobber of source register.
                    let name = self.fresh();
                    let src = ints[self.rng.pick(ints.len() as u64) as usize].clone();
                    self.push(&format!("  let {}: Shared<Int> = Shared({});\n", name, src));
                    shareds.push(name);
                }
                8 if region_depth > 0 && !shareds.is_empty() => {
                    let name = self.fresh();
                    let src = shareds[self.rng.pick(shareds.len() as u64) as usize].clone();
                    self.push(&format!("  let {}: Shared<Int> = {}.clone();\n", name, src));
                    shareds.push(name);
                }
                9 if region_depth > 0 && !shareds.is_empty() => {
                    let name = self.fresh();
                    let src = shareds[self.rng.pick(shareds.len() as u64) as usize].clone();
                    self.push(&format!("  let {}: Int = *{};\n", name, src));
                    ints.push(name);
                }
                10 if region_depth > 0 && self.has_record => {
                    // Shared<R>: record handle wrapped in Shared.
                    let name = self.fresh();
                    let a = self.rng.pick(100) as i64;
                    let b = self.rng.pick(100) as i64;
                    self.push(&format!(
                        "  let {}: Shared<R> = Shared(R {{ a: {}, b: {} }});\n",
                        name, a, b
                    ));
                    shared_recs.push(name);
                }
                11 if region_depth > 0 && !shared_recs.is_empty() => {
                    let name = self.fresh();
                    let src = shared_recs[self.rng.pick(shared_recs.len() as u64) as usize].clone();
                    self.push(&format!("  let {}: Shared<R> = {}.clone();\n", name, src));
                    shared_recs.push(name);
                }
                12 if loop_depth < 1 => {
                    // Loop body is an implicit region. Break immediately to avoid hangs.
                    self.push("  loop {\n");
                    self.gen_block(region_depth, shareds, ints, shared_recs, loop_depth + 1);
                    self.push("    break;\n");
                    self.push("  }\n");
                }
                13 if self.has_static => {
                    // if/else: static only in else branch (non-first).
                    let name = self.fresh();
                    self.push(&format!("  let {}: Int = if false {{ 0 }} else {{ S }};\n", name));
                    ints.push(name);
                }
                14 if self.has_static => {
                    // match wildcard: static only in non-first arm.
                    let name = self.fresh();
                    self.push(&format!("  let {}: Int = match 1 {{ 0 => 0, _ => S }};\n", name));
                    ints.push(name);
                }
                _ => {
                    let name = self.fresh();
                    self.push(&format!("  let {}: Int = 0;\n", name));
                    ints.push(name);
                }
            }
        }
        shareds.truncate(s_snap);
        ints.truncate(i_snap);
        shared_recs.truncate(sr_snap);
    }
}

#[derive(Default)]
struct Stats {
    total: u64, parsed: u64, compiled: u64, ran: u64,
    leaked: u64, wrong_val: u64, run_err: u64, hung: u64,
}

#[derive(Default)]
struct Bucket { count: u64, examples: Vec<(u64, String, String)> } // (seed, detail, src)
impl Bucket {
    fn record(&mut self, seed: u64, detail: String, src: String, cap: usize) {
        self.count += 1;
        if self.examples.len() < cap { self.examples.push((seed, detail, src)); }
    }
}

enum Outcome { Ok, ParseFail, CompileFail, RunErr(String), Leak(usize), WrongRet(i64), Hang }

fn try_run_with_timeout(src: String, step_cap: u64) -> Outcome {
    try_run(&src, step_cap)
}

fn try_run(src: &str, step_cap: u64) -> Outcome {
    let mut p = Parser::new(Lexer::new(src)).with_source(src.to_string());
    let ast = p.parse_program();
    if !p.errors.is_empty() { return Outcome::ParseFail; }
    let mut compiler = Compiler::new();
    let module = match compiler.compile_module(&ast) {
        Ok(m) => m,
        Err(_) => return Outcome::CompileFail,
    };
    let mut vm = VirtualMachine::new().with_step_cap(step_cap);
    let v = match vm.run_module(&module) {
        Ok(v) => v,
        Err(e) => {
            if e.starts_with("step cap exceeded") { return Outcome::Hang; }
            return Outcome::RunErr(e);
        }
    };
    let live = vm.heap_live_count();
    if live != 0 { return Outcome::Leak(live); }
    let ret = v.as_int();
    if ret != 0 { return Outcome::WrongRet(ret); }
    Outcome::Ok
}

const ITER: u64 = 2_000;
const STEP_CAP: u64 = 1_000_000;
const EXAMPLES_PER_BUCKET: usize = 3;

#[test]
fn fuzz_no_leak_no_panic() {
    let mut st = Stats::default();
    let mut hangs = Bucket::default();
    let mut leaks = Bucket::default();
    let mut wrongs = Bucket::default();
    let mut errs = Bucket::default();
    for seed in 0..ITER {
        st.total += 1;
        let src = Gen::new(seed).gen_program();
        match try_run_with_timeout(src.clone(), STEP_CAP) {
            Outcome::Ok => { st.parsed += 1; st.compiled += 1; st.ran += 1; }
            Outcome::ParseFail => {}
            Outcome::CompileFail => { st.parsed += 1; }
            Outcome::Hang => {
                st.parsed += 1; st.compiled += 1; st.hung += 1;
                hangs.record(seed, format!(">{} ops", STEP_CAP), src, EXAMPLES_PER_BUCKET);
            }
            Outcome::RunErr(e) => {
                st.parsed += 1; st.compiled += 1; st.run_err += 1;
                errs.record(seed, e, src, EXAMPLES_PER_BUCKET);
            }
            Outcome::Leak(n) => {
                st.parsed += 1; st.compiled += 1; st.ran += 1;
                st.leaked += 1;
                leaks.record(seed, format!("live={}", n), src, EXAMPLES_PER_BUCKET);
            }
            Outcome::WrongRet(r) => {
                st.parsed += 1; st.compiled += 1; st.ran += 1;
                st.wrong_val += 1;
                wrongs.record(seed, format!("ret={}", r), src, EXAMPLES_PER_BUCKET);
            }
        }
    }
    eprintln!(
        "\nfuzz stats: total={} parsed={} compiled={} ran={} | run_err={} hung={} leaked={} wrong_val={}",
        st.total, st.parsed, st.compiled, st.ran,
        st.run_err, st.hung, st.leaked, st.wrong_val
    );
    let report = |name: &str, b: &Bucket| {
        if b.count == 0 { return String::new(); }
        let mut s = format!("\n=== {} ({} total) ===\n", name, b.count);
        for (seed, detail, src) in &b.examples {
            s.push_str(&format!("--- seed={} {} ---\n{}\n", seed, detail, src));
        }
        s
    };
    let body = format!("{}{}{}{}",
        report("HANG", &hangs),
        report("LEAK", &leaks),
        report("WRONG_RETURN", &wrongs),
        report("RUN_ERROR", &errs),
    );
    if !body.is_empty() { eprintln!("{}", body); }
    assert!(st.ran * 4 >= st.total,
        "coverage too low: only {}/{} programs reached VM run", st.ran, st.total);
    assert!(
        st.hung == 0 && st.leaked == 0 && st.wrong_val == 0 && st.run_err == 0,
        "fuzz found bugs: hung={} leaked={} wrong_val={} run_err={} (see stderr report)",
        st.hung, st.leaked, st.wrong_val, st.run_err
    );
}
