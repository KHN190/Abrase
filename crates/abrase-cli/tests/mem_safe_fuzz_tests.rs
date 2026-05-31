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
    has_variant: bool,
    has_static_rec: bool,
    has_static_rec_arr: bool,
}

impl Gen {
    fn new(seed: u64) -> Self {
        Self {
            rng: Rng::new(seed), next_var: 0, next_fn: 0,
            out: String::new(), helpers: String::new(), fuel: 250,
            has_record: false, has_static: false,
            has_variant: false, has_static_rec: false, has_static_rec_arr: false,
        }
    }
    fn fresh(&mut self) -> String { let n = self.next_var; self.next_var += 1; format!("v{}", n) }
    fn fresh_fn(&mut self) -> String { let n = self.next_fn; self.next_fn += 1; format!("f{}", n) }
    fn push(&mut self, s: &str) { self.out.push_str(s); }

    fn expected_static_live(&self) -> usize {
        let mut n = 0;
        if self.has_static_rec     { n += 1; }
        if self.has_static_rec_arr { n += 5; } // 1 Array + 4 R records
        n
    }

    fn gen_program(&mut self) -> String {
        // Optional borrow-taking helper.
        if self.rng.pick(2) == 0 {
            let name = self.fresh_fn();
            self.helpers.push_str(&format!("fn {}(x: &Int) -> Int {{ *x }}\n", name));
        }
        // Optional record type R = { a: Int, b: Int }.
        if self.rng.pick(2) == 0 {
            self.helpers.push_str("type R = { a: Int, b: Int }\n");
            self.has_record = true;
        }
        // Optional variant type Tag = Zero | One(Int).
        if self.has_record && self.rng.pick(3) == 0 {
            self.helpers.push_str("type Tag = Zero | One(Int)\n");
            self.has_variant = true;
        }
        // Optional scalar static.
        if self.rng.pick(2) == 0 {
            let v = self.rng.pick(100) as i64;
            self.helpers.push_str(&format!("static S: Int = {};\n", v));
            self.has_static = true;
        }
        // Optional static mut record.
        if self.has_record && self.rng.pick(3) == 0 {
            self.helpers.push_str("static mut SR: R = R { a: 0, b: 0 }\n");
            self.has_static_rec = true;
            self.has_static = true;
        }
        // Optional static mut Array<R> (exercises the alias-fix for heap array-repeat).
        if self.has_record && self.rng.pick(3) == 0 {
            self.helpers.push_str("static mut SRA: Array<R> = [R { a: 0, b: 0 }; 4]\n");
            self.has_static_rec_arr = true;
            self.has_static = true;
        }
        self.push("fn main() -> Int {\n");
        self.gen_block(0, &mut vec![], &mut vec![], &mut vec![], &mut vec![], &mut vec![], 0);
        self.push("  0\n}\n");
        format!("{}{}", std::mem::take(&mut self.helpers), std::mem::take(&mut self.out))
    }

    #[allow(clippy::too_many_arguments)]
    fn gen_block(
        &mut self,
        region_depth: u32,
        shareds: &mut Vec<String>,
        ints: &mut Vec<String>,
        shared_recs: &mut Vec<String>,
        records: &mut Vec<String>,
        variants: &mut Vec<String>,
        loop_depth: u32,
    ) {
        let stmts = (self.rng.pick(6) + 1) as usize;
        let s_snap = shareds.len();
        let i_snap = ints.len();
        let sr_snap = shared_recs.len();
        let rec_snap = records.len();
        let var_snap = variants.len();
        for _ in 0..stmts {
            if self.fuel == 0 { break; }
            self.fuel -= 1;
            let choices = if region_depth > 0 { 23 } else { 16 };
            match self.rng.pick(choices) {
                // ── scalars ──────────────────────────────────────────────────
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
                    self.gen_block(region_depth + 1, shareds, ints, shared_recs, records, variants, loop_depth);
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
                // ── Shared (region-only) ──────────────────────────────────
                6 if region_depth > 0 => {
                    let name = self.fresh();
                    let v = self.rng.pick(1000) as i64;
                    self.push(&format!("  let {}: Shared<Int> = Shared({});\n", name, v));
                    shareds.push(name);
                }
                7 if region_depth > 0 && !ints.is_empty() => {
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
                    self.push("  loop {\n");
                    self.gen_block(region_depth, shareds, ints, shared_recs, records, variants, loop_depth + 1);
                    self.push("    break;\n");
                    self.push("  }\n");
                }
                // ── static branch regression (Dei cache in if/match arms) ─
                13 if self.has_static => {
                    let name = self.fresh();
                    self.push(&format!("  let {}: Int = if false {{ 0 }} else {{ S }};\n", name));
                    ints.push(name);
                }
                14 if self.has_static => {
                    let name = self.fresh();
                    self.push(&format!("  let {}: Int = match 1 {{ 0 => 0, _ => S }};\n", name));
                    ints.push(name);
                }
                // ── records ──────────────────────────────────────────────
                15 if self.has_record => {
                    let name = self.fresh();
                    let a = self.rng.pick(100) as i64;
                    let b = self.rng.pick(100) as i64;
                    self.push(&format!("  let {}: R = R {{ a: {}, b: {} }};\n", name, a, b));
                    records.push(name);
                }
                // ── record field reads ────────────────────────────────────
                _ if !records.is_empty() && self.has_record => {
                    let name = self.fresh();
                    let rec = records[self.rng.pick(records.len() as u64) as usize].clone();
                    let field = if self.rng.pick(2) == 0 { "a" } else { "b" };
                    self.push(&format!("  let {}: Int = {}.{};\n", name, rec, field));
                    ints.push(name);
                }
                // ── static mut record ────────────────────────────────────
                _ if self.has_static_rec && self.rng.pick(3) == 0 => {
                    let v = self.rng.pick(50) as i64;
                    let field = if self.rng.pick(2) == 0 { "a" } else { "b" };
                    self.push(&format!("  SR.{} = {};\n", field, v));
                }
                _ if self.has_static_rec => {
                    let name = self.fresh();
                    let field = if self.rng.pick(2) == 0 { "a" } else { "b" };
                    self.push(&format!("  let {}: Int = SR.{};\n", name, field));
                    ints.push(name);
                }
                // ── static mut Array<R> (alias regression) ───────────────
                _ if self.has_static_rec_arr && !ints.is_empty() && self.rng.pick(2) == 0 => {
                    let v = self.rng.pick(50) as i64;
                    let i = self.rng.pick(4);
                    let field = if self.rng.pick(2) == 0 { "a" } else { "b" };
                    self.push(&format!("  SRA[{}].{} = {};\n", i, field, v));
                }
                _ if self.has_static_rec_arr => {
                    let name = self.fresh();
                    let i = self.rng.pick(4);
                    let field = if self.rng.pick(2) == 0 { "a" } else { "b" };
                    self.push(&format!("  let {}: Int = SRA[{}].{};\n", name, i, field));
                    ints.push(name);
                }
                // ── variants ─────────────────────────────────────────────
                _ if self.has_variant && variants.len() < 4 => {
                    let name = self.fresh();
                    if self.rng.pick(2) == 0 {
                        let v = self.rng.pick(100) as i64;
                        self.push(&format!("  let {}: Tag = One({});\n", name, v));
                    } else {
                        self.push(&format!("  let {}: Tag = Zero;\n", name));
                    }
                    variants.push(name);
                }
                _ if self.has_variant && !variants.is_empty() => {
                    let name = self.fresh();
                    let v = variants[self.rng.pick(variants.len() as u64) as usize].clone();
                    self.push(&format!(
                        "  let {}: Int = match {} {{ One(n) => n, _ => 0 }};\n",
                        name, v
                    ));
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
        records.truncate(rec_snap);
        variants.truncate(var_snap);
    }
}

#[derive(Default)]
struct Stats {
    total: u64, parsed: u64, compiled: u64, ran: u64,
    leaked: u64, wrong_val: u64, run_err: u64, hung: u64,
}

#[derive(Default)]
struct Bucket { count: u64, examples: Vec<(u64, String, String)> }
impl Bucket {
    fn record(&mut self, seed: u64, detail: String, src: String, cap: usize) {
        self.count += 1;
        if self.examples.len() < cap { self.examples.push((seed, detail, src)); }
    }
}

enum Outcome { Ok, ParseFail, CompileFail, RunErr(String), Leak(usize), WrongRet(i64), Hang }

fn try_run(src: &str, step_cap: u64, expected_live: usize) -> Outcome {
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
    if live != expected_live { return Outcome::Leak(live); }
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
        let mut g = Gen::new(seed);
        let src = g.gen_program();
        let expected_live = g.expected_static_live();
        match try_run(&src, STEP_CAP, expected_live) {
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
                st.parsed += 1; st.compiled += 1; st.ran += 1; st.leaked += 1;
                leaks.record(seed, format!("live={}", n), src, EXAMPLES_PER_BUCKET);
            }
            Outcome::WrongRet(r) => {
                st.parsed += 1; st.compiled += 1; st.ran += 1; st.wrong_val += 1;
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
        report("HANG", &hangs), report("LEAK", &leaks),
        report("WRONG_RETURN", &wrongs), report("RUN_ERROR", &errs),
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

// ── multi-module fuzz ────────────────────────────────────────────────────────
// Generates a two-file program: lib.abe exports statics + record helpers,
// main.abe imports and exercises them across call_export frames.

use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};
static MM_CTR: AtomicU64 = AtomicU64::new(0);

fn try_run_files(entry: &std::path::Path, step_cap: u64, expected_live: usize) -> Outcome {
    let loaded = match abrase::loader::load_program(entry) {
        Ok(l) => l,
        Err(_) => return Outcome::ParseFail,
    };
    let mut compiler = Compiler::new().with_source(loaded.entry_source.clone());
    let module = match compiler.compile_module(&loaded.decls) {
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
    // Exercise exports a few times to catch cross-frame static aliasing.
    for export in &["tick", "read"] {
        if module.exports.iter().any(|e| e.name == *export) {
            match vm.call_export(&module, export, &[]) {
                Err(e) => return Outcome::RunErr(e),
                Ok(_) => {}
            }
        }
    }
    let live = vm.heap_live_count();
    if live != expected_live { return Outcome::Leak(live); }
    let ret = v.as_int();
    if ret != 0 { return Outcome::WrongRet(ret); }
    Outcome::Ok
}

struct MMGen { rng: Rng, seed: u64 }

impl MMGen {
    fn new(seed: u64) -> Self { Self { rng: Rng::new(seed), seed } }

    fn generate(&mut self) -> (String, String, usize) {
        let n = MM_CTR.fetch_add(1, Ordering::Relaxed);
        let _ = n;

        let has_rec  = self.rng.pick(2) == 0;
        let has_sarr = has_rec && self.rng.pick(2) == 0;
        let expected_live: usize = if has_sarr { 5 } else { 0 }; // 1 Array + 4 Ent records
        let n_int    = (self.rng.pick(3) + 1) as usize;

        let mut lib = String::new();
        if has_rec {
            lib.push_str("type Ent = { hp: Int, active: Int }\n");
        }
        for i in 0..n_int {
            lib.push_str(&format!("pub static mut X{}: Int = {}\n", i, i as i64 * 10));
        }
        if has_sarr {
            lib.push_str("pub static mut EA: Array<Ent> = [Ent { hp: 5, active: 0 }; 4]\n");
        }
        // tick: mutate statics
        lib.push_str("pub fn tick() -> Unit {\n");
        for i in 0..n_int {
            lib.push_str(&format!("  X{i} = X{i} + 1;\n"));
        }
        if has_sarr {
            lib.push_str("  EA[0].hp = EA[0].hp - 1;\n");
            lib.push_str("  EA[1].active = 1;\n");
        }
        lib.push_str("}\n");
        // read: return sum of statics (verifiable)
        lib.push_str("pub fn read() -> Int {\n  ");
        let sum: String = (0..n_int).map(|i| format!("X{}", i)).collect::<Vec<_>>().join(" + ");
        lib.push_str(&sum);
        lib.push_str("\n}\n");
        lib.push_str("fn main() -> Int { 0 }\n");

        // main imports and calls
        let imports: String = (0..n_int).map(|i| format!("X{}", i)).collect::<Vec<_>>().join(", ");
        let mut main = format!("use lib::{{{}}}\nuse lib::{{tick, read}}\n", imports);
        if has_sarr {
            main.push_str("use lib::{EA}\n");
        }
        main.push_str("fn main() -> Int {\n");
        main.push_str("  tick();\n  tick();\n");
        if has_sarr {
            main.push_str("  let _hp = EA[0].hp;\n");
        }
        main.push_str("  0\n}\n");

        (lib, main, expected_live)
    }
}

#[test]
fn fuzz_multi_module_no_crash() {
    let mut st = Stats::default();
    let mut errs = Bucket::default();
    let mut leaks = Bucket::default();

    for seed in 0..500u64 {
        let (lib_src, main_src, expected_live) = MMGen::new(seed).generate();
        let n = MM_CTR.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir()
            .join(format!("abrase_mm_fuzz_{}_{}", std::process::id(), n));
        fs::create_dir_all(&dir).expect("create temp dir");
        let lib_dir = dir.join("lib");
        fs::create_dir_all(&lib_dir).expect("create lib dir");
        fs::write(lib_dir.join("mod.abe"), &lib_src).expect("write lib");
        fs::write(dir.join("main.abe"), &main_src).expect("write main");

        st.total += 1;
        let entry = dir.join("main.abe");
        match try_run_files(&entry, STEP_CAP, expected_live) {
            Outcome::Ok => { st.parsed += 1; st.compiled += 1; st.ran += 1; }
            Outcome::ParseFail  => {}
            Outcome::CompileFail => { st.parsed += 1; }
            Outcome::Hang => { st.parsed += 1; st.compiled += 1; st.hung += 1; }
            Outcome::RunErr(e) => {
                st.parsed += 1; st.compiled += 1; st.run_err += 1;
                let detail = format!("seed={} {}", seed, e);
                let combined = format!("--- lib ---\n{}\n--- main ---\n{}", lib_src, main_src);
                errs.record(seed, detail, combined, EXAMPLES_PER_BUCKET);
            }
            Outcome::Leak(n) => {
                st.parsed += 1; st.compiled += 1; st.ran += 1; st.leaked += 1;
                let detail = format!("seed={} live={}", seed, n);
                let combined = format!("--- lib ---\n{}\n--- main ---\n{}", lib_src, main_src);
                leaks.record(seed, detail, combined, EXAMPLES_PER_BUCKET);
            }
            Outcome::WrongRet(_) => { st.parsed += 1; st.compiled += 1; st.ran += 1; }
        }
        fs::remove_dir_all(&dir).ok();
    }

    eprintln!(
        "\nmm fuzz: total={} parsed={} compiled={} ran={} | run_err={} leaked={}",
        st.total, st.parsed, st.compiled, st.ran, st.run_err, st.leaked
    );
    let report = |name: &str, b: &Bucket| {
        if b.count == 0 { return String::new(); }
        let mut s = format!("\n=== {} ({} total) ===\n", name, b.count);
        for (seed, detail, src) in &b.examples {
            s.push_str(&format!("--- seed={} {} ---\n{}\n", seed, detail, src));
        }
        s
    };
    let body = format!("{}{}", report("RUN_ERROR", &errs), report("LEAK", &leaks));
    if !body.is_empty() { eprintln!("{}", body); }
    assert!(
        st.run_err == 0 && st.leaked == 0,
        "multi-module fuzz found bugs: run_err={} leaked={} (see stderr)",
        st.run_err, st.leaked
    );
}
