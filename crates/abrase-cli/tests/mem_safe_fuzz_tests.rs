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
    has_array_static: bool,
    has_variant: bool,
    has_effect: bool,
    has_recursive_fn: bool,
}

impl Gen {
    fn new(seed: u64) -> Self {
        Self {
            rng: Rng::new(seed), next_var: 0, next_fn: 0,
            out: String::new(), helpers: String::new(), fuel: 250,
            has_record: false, has_static: false, has_array_static: false,
            has_variant: false,
            has_effect: false, has_recursive_fn: false,
        }
    }
    fn fresh(&mut self) -> String { let n = self.next_var; self.next_var += 1; format!("v{}", n) }
    fn fresh_fn(&mut self) -> String { let n = self.next_fn; self.next_fn += 1; format!("f{}", n) }
    fn push(&mut self, s: &str) { self.out.push_str(s); }

    fn expected_static_live(&self) -> usize { 0 }

    fn gen_program(&mut self) -> String {
        // Optional borrow-taking helper.
        if self.rng.pick(2) == 0 {
            let name = self.fresh_fn();
            self.helpers.push_str(&format!("fn {}(x: &Int) -> Int {{ *x }}\n", name));
        }
        // Optional record type R = { a: Int, b: Int } + a &mut-taking mutator.
        if self.rng.pick(2) == 0 {
            self.helpers.push_str("type R = { a: Int, b: Int }\n");
            self.helpers.push_str("fn mutr(r: &mut R) -> Int { r.a = r.a + 1; r.a }\n");
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
        if self.rng.pick(3) == 0 {
            self.helpers.push_str(
                "effect Tick { op go() -> Unit }\n\
                 fn tick_n(n: Int) -> <Tick> Unit {\n\
                   let mut i = 0;\n\
                   while i < n { Tick.go(); i = i + 1 }\n\
                 }\n"
            );
            self.has_effect = true;
        }
        if self.rng.pick(3) == 0 {
            self.helpers.push_str(
                "fn countdown(n: Int) -> Int {\n\
                   if n <= 0 { 0 } else { 1 + countdown(n - 1) }\n\
                 }\n"
            );
            self.has_recursive_fn = true;
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
            let choices = if region_depth > 0 { 38 } else { 34 };
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
                5 if self.has_static || self.has_array_static => {
                    let name = self.fresh();
                    if self.has_array_static && (!self.has_static || self.rng.pick(2) == 0) {
                        let idx = self.rng.pick(8);
                        self.push(&format!("  let {}: Int = BH[{}];\n", name, idx));
                    } else {
                        self.push(&format!("  let {}: Int = S;\n", name));
                    }
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
                // ── match with guard (guard codegen regression) ──────────
                15 if !ints.is_empty() => {
                    let name = self.fresh();
                    let src = ints[self.rng.pick(ints.len() as u64) as usize].clone();
                    let thresh = self.rng.pick(200) as i64 - 100;
                    self.push(&format!(
                        "  let {}: Int = match {src} {{ _ if {src} > {thresh} => 0, _ => 0 }};\n",
                        name, src=src, thresh=thresh
                    ));
                    ints.push(name);
                }
                // ── float conversion ──────────────────────────────────────
                16 if !ints.is_empty() => {
                    let name = self.fresh();
                    let a = ints[self.rng.pick(ints.len() as u64) as usize].clone();
                    let b = ints[self.rng.pick(ints.len() as u64) as usize].clone();
                    self.push(&format!(
                        "  let {}: Int = ({a}.to_f() + {b}.to_f()).to_i();\n",
                        name, a=a, b=b
                    ));
                    ints.push(name);
                }
                // ── effect handler ────────────────────────────────────────
                20 if self.has_effect && loop_depth < 1 => {
                    let n = self.rng.pick(8) + 1;
                    self.push(&format!(
                        "  handle tick_n({n}) {{ return _ => (), Tick.go => resume(()) }};\n"
                    ));
                }
                // ── recursion ─────────────────────────────────────────────
                21 if self.has_recursive_fn => {
                    let name = self.fresh();
                    let n = self.rng.pick(8) as i64;
                    self.push(&format!("  let {}: Int = countdown({n});\n", name));
                    ints.push(name);
                }
                // ── for loop ─────────────────────────────────────────────
                22 if loop_depth < 1 => {
                    let end = self.rng.pick(8) + 2;
                    let name = self.fresh();
                    self.push(&format!(
                        "  let mut {name}: Int = 0;\n  for _i in 0..{end} {{ {name} = {name} + 1 }};\n"
                    ));
                    ints.push(name);
                }
                // ── exception handling ────────────────────────────────────
                23 if !ints.is_empty() => {
                    let name = self.fresh();
                    let src = ints[self.rng.pick(ints.len() as u64) as usize].clone();
                    // safe_div: throw if src==0, else return 1
                    self.push(&format!(
                        "  let {name}: Int = handle (if {src} == 0 {{ throw 0 }} else {{ 1 }}) {{ return v => v, exn _ => 0 }};\n"
                    ));
                    ints.push(name);
                }
                // ── closure capture ───────────────────────────────────────
                24 if !ints.is_empty() => {
                    let name = self.fresh();
                    let cap = ints[self.rng.pick(ints.len() as u64) as usize].clone();
                    let add = self.rng.pick(20) as i64;
                    self.push(&format!(
                        "  let {name}: Int = (|x| x + {add})({cap});\n"
                    ));
                    ints.push(name);
                }
                // ── tuple pack/unpack ─────────────────────────────────────
                25 if !ints.is_empty() => {
                    let name = self.fresh();
                    let a = ints[self.rng.pick(ints.len() as u64) as usize].clone();
                    let b = ints[self.rng.pick(ints.len() as u64) as usize].clone();
                    self.push(&format!(
                        "  let ({name}, _) = ({a} + {b}, 0);\n"
                    ));
                    ints.push(name);
                }
                // ── move closure capturing Int ────────────────────────────
                26 if !ints.is_empty() => {
                    let name = self.fresh();
                    let cap = ints[self.rng.pick(ints.len() as u64) as usize].clone();
                    let add = self.rng.pick(10) as i64;
                    let arg = self.rng.pick(10) as i64;
                    self.push(&format!(
                        "  let {name}: Int = (move |x| x + {cap} + {add})({arg});\n"
                    ));
                    ints.push(name);
                }
                // ── closure capturing record (heap type RC balance) ───────
                27 if self.has_record && !ints.is_empty() => {
                    let name = self.fresh();
                    let a = self.rng.pick(50) as i64;
                    let b = self.rng.pick(50) as i64;
                    let arg = self.rng.pick(5) as i64;
                    self.push(&format!(
                        "  let {name}: Int = (move |s| (R {{ a: {a}, b: {b} }}).a * s)({arg});\n"
                    ));
                    ints.push(name);
                }
                // ── char round-trip ───────────────────────────────────────
                28 if !ints.is_empty() => {
                    let name = self.fresh();
                    let n = (self.rng.pick(26) + 65) as i64; // 'A'..'Z'
                    self.push(&format!("  let {name}: Int = {n}.to_c().to_i();\n"));
                    ints.push(name);
                }
                // ── record mut destructure ────────────────────────────────
                27 if self.has_record && !ints.is_empty() => {
                    let a = self.rng.pick(50) as i64;
                    let b = self.rng.pick(50) as i64;
                    let dx = self.rng.pick(20) as i64;
                    let vname = self.fresh();
                    let rname = self.fresh();
                    self.push(&format!(
                        "  let {rname}: R = R {{ a: {a}, b: {b} }};\n  let mut R {{ a: {vname}, .. }} = {rname};\n  {vname} = {vname} + {dx};\n"
                    ));
                    ints.push(vname);
                }
                // ── region returning record (region_forget path) ─────────
                30 if region_depth == 0 && self.has_record => {
                    let name = self.fresh();
                    let a = self.rng.pick(50) as i64;
                    let b = self.rng.pick(50) as i64;
                    self.push(&format!(
                        "  let {name}: R = region {{ R {{ a: {a}, b: {b} }} }};\n"
                    ));
                    records.push(name);
                }
                // ── region returning move closure (region_forget + env-copy) ─
                31 if region_depth == 0 && !ints.is_empty() => {
                    let cap = ints[self.rng.pick(ints.len() as u64) as usize].clone();
                    let arg = self.rng.pick(10) as i64;
                    let f = self.fresh();
                    let name = self.fresh();
                    self.push(&format!(
                        "  let {f} = region {{ move |x: Int| x + {cap} }};\n  let {name}: Int = {f}({arg});\n"
                    ));
                    ints.push(name);
                }
                // ── bitwise ops ───────────────────────────────────────────
                _ if !ints.is_empty() && self.rng.pick(4) == 0 => {
                    let name = self.fresh();
                    let a = ints[self.rng.pick(ints.len() as u64) as usize].clone();
                    let b = ints[self.rng.pick(ints.len() as u64) as usize].clone();
                    let op = ["&", "|", "^"][self.rng.pick(3) as usize];
                    self.push(&format!("  let {name}: Int = {a} {op} {b};\n"));
                    ints.push(name);
                }
                // ── closure value: bind, capture an Int, call ────────────────
                _ if !ints.is_empty() && self.rng.pick(3) == 0 => {
                    let cap = ints[self.rng.pick(ints.len() as u64) as usize].clone();
                    let arg = ints[self.rng.pick(ints.len() as u64) as usize].clone();
                    let f = self.fresh();
                    let name = self.fresh();
                    // call once or twice (exercises the env-copy / no-leak path)
                    if self.rng.pick(2) == 0 {
                        self.push(&format!(
                            "  let {f} = move |x: Int| x + {cap};\n  let {name}: Int = {f}({arg}) + {f}({arg});\n"));
                    } else {
                        self.push(&format!(
                            "  let {f} = move |x: Int| x + {cap};\n  let {name}: Int = {f}({arg});\n"));
                    }
                    ints.push(name);
                }
                // ── sequential &mut borrows of one binding (last-use release) ─
                _ if self.has_record && self.rng.pick(3) == 0 => {
                    let a = self.rng.pick(50) as i64;
                    let b = self.rng.pick(50) as i64;
                    let x = self.fresh();
                    let r1 = self.fresh();
                    let r2 = self.fresh();
                    self.push(&format!(
                        "  let mut {x}: R = R {{ a: {a}, b: {b} }};\n  let {r1}: Int = mutr(&mut {x});\n  let {r2}: Int = mutr(&mut {x});\n"
                    ));
                    ints.push(r1);
                    ints.push(r2);
                }
                // ── records ──────────────────────────────────────────────
                _ if self.has_record && self.rng.pick(3) == 0 => {
                    let name = self.fresh();
                    let a = self.rng.pick(100) as i64;
                    let b = self.rng.pick(100) as i64;
                    self.push(&format!("  let {}: R = R {{ a: {}, b: {} }};\n", name, a, b));
                    records.push(name);
                }
                // ── record field reads ────────────────────────────────────
                _ if !records.is_empty() && self.has_record && self.rng.pick(2) == 0 => {
                    let name = self.fresh();
                    let rec = records[self.rng.pick(records.len() as u64) as usize].clone();
                    let field = if self.rng.pick(2) == 0 { "a" } else { "b" };
                    self.push(&format!("  let {}: Int = {}.{};\n", name, rec, field));
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
    try_run_checked(src, step_cap, expected_live, false)
}

fn try_run_checked(src: &str, step_cap: u64, expected_live: usize, heap_check: bool) -> Outcome {
    let mut p = Parser::new(Lexer::new(src)).with_source(src.to_string());
    let ast = p.parse_program();
    if !p.errors.is_empty() { return Outcome::ParseFail; }
    let mut compiler = Compiler::new();
    let module = match compiler.compile_module(&ast) {
        Ok(m) => m,
        Err(_) => return Outcome::CompileFail,
    };
    let mut vm = VirtualMachine::new().with_step_cap(step_cap).with_heap_check(heap_check);
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

// Compile+run with drop-elision forced on or off. Err collapses all failure
// kinds (parse/compile/hang/runtime) — used only to compare on-vs-off agreement.
fn run_elision(src: &str, step_cap: u64, elide: bool) -> Result<(i64, usize), String> {
    let mut p = Parser::new(Lexer::new(src)).with_source(src.to_string());
    let ast = p.parse_program();
    if !p.errors.is_empty() { return Err("parse".into()); }
    let mut c = Compiler::new().with_drop_elision(elide);
    let module = c.compile_module(&ast).map_err(|_| "compile".to_string())?;
    let mut vm = VirtualMachine::new().with_step_cap(step_cap);
    let v = vm.run_module(&module).map_err(|e| e)?;
    Ok((v.as_int(), vm.heap_live_count()))
}

// Empirical proof that drop-elision is a sound transform: across random programs
// (closures, effects, handlers, statics, records) it changes neither the result
// value nor the live-cell count vs no-elision. This is how we "prove" the handle
// tag reliable enough to elide — oracle + fuzz, not a static borrow proof.
#[test]
fn fuzz_drop_elision_equivalence() {
    let mut diverged = 0u32;
    let mut report = String::new();
    for seed in 0..ITER {
        let mut g = Gen::new(seed);
        let src = g.gen_program();
        let off = run_elision(&src, STEP_CAP, false);
        let on  = run_elision(&src, STEP_CAP, true);
        match (&off, &on) {
            (Ok((vo, lo)), Ok((ve, le))) if vo == ve && lo == le => {}
            (Err(_), Err(_)) => {}
            _ => {
                diverged += 1;
                if report.len() < 4000 {
                    report.push_str(&format!("--- seed={} off={:?} on={:?} ---\n{}\n", seed, off, on, src));
                }
            }
        }
    }
    assert_eq!(diverged, 0, "drop-elision diverged on {} program(s):\n{}", diverged, report);
}

fn run_inline(src: &str, step_cap: u64, inline: bool) -> Result<(i64, usize), String> {
    let mut p = Parser::new(Lexer::new(src)).with_source(src.to_string());
    let ast = p.parse_program();
    if !p.errors.is_empty() { return Err("parse".into()); }
    let mut c = Compiler::new().with_inline(inline);
    let module = c.compile_module(&ast).map_err(|_| "compile".to_string())?;
    let mut vm = VirtualMachine::new().with_step_cap(step_cap);
    let v = vm.run_module(&module).map_err(|e| e)?;
    Ok((v.as_int(), vm.heap_live_count()))
}

#[test]
fn fuzz_inline_equivalence() {
    let mut diverged = 0u32;
    let mut report = String::new();
    for seed in 0..ITER {
        let mut g = Gen::new(seed);
        let src = g.gen_program();
        let off = run_inline(&src, STEP_CAP, false);
        let on  = run_inline(&src, STEP_CAP, true);
        match (&off, &on) {
            (Ok((vo, lo)), Ok((ve, le))) if vo == ve && lo == le => {}
            (Err(_), Err(_)) => {}
            _ => {
                diverged += 1;
                if report.len() < 4000 {
                    report.push_str(&format!("--- seed={} off={:?} on={:?} ---\n{}\n", seed, off, on, src));
                }
            }
        }
    }
    assert_eq!(diverged, 0, "inline diverged on {} program(s):\n{}", diverged, report);
}

// Focused: a small pure leaf fn (abs_diff shape) must inline and compute right.
#[test]
fn inline_leaf_preserves_value() {
    let src = "fn ad(a: Int, b: Int) -> Int { if a > b { a - b } else { b - a } }\n\
               fn main() -> Int { ad(3, 10) + ad(10, 3) }";
    let on = run_inline(src, STEP_CAP, true).expect("inline run");
    let off = run_inline(src, STEP_CAP, false).expect("noinline run");
    assert_eq!(on.0, 14, "value");
    assert_eq!(on, off, "inline must match no-inline (value, heap)");
    assert_eq!(on.1, 0, "no leak");
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

const UAF_ITER: u64 = 1_000;

#[test]
fn fuzz_no_uaf_stale_handle_under_heap_check() {
    let mut bad: Vec<(u64, String, String)> = Vec::new();
    for seed in 0..UAF_ITER {
        let mut g = Gen::new(seed);
        let src = g.gen_program();
        let expected_live = g.expected_static_live();
        if let Outcome::RunErr(e) = try_run_checked(&src, STEP_CAP, expected_live, true) {
            if bad.len() < EXAMPLES_PER_BUCKET { bad.push((seed, e, src)); }
        }
    }
    if !bad.is_empty() {
        for (seed, e, src) in &bad {
            eprintln!("--- UAF/stale seed={} {} ---\n{}\n", seed, e, src);
        }
        panic!("heap-check fuzz found {} dangling-tag program(s)", bad.len());
    }
}

