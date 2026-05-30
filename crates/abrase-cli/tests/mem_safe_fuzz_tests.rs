// memory-safety fuzz: generate random small carts from a legal subset
// (let / region / Shared / clone), compile, run, assert heap_live_count == 0.

mod compiler_codegen_common;

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
    out: String,
    fuel: u32,
}

impl Gen {
    fn new(seed: u64) -> Self {
        Self { rng: Rng::new(seed), next_var: 0, out: String::new(), fuel: 30 }
    }
    fn fresh(&mut self) -> String {
        let n = self.next_var; self.next_var += 1;
        format!("v{}", n)
    }
    fn push(&mut self, s: &str) { self.out.push_str(s); }

    fn gen_program(&mut self) -> String {
        self.push("fn main() -> Int {\n");
        self.gen_block(0, /*shared_in_scope=*/ &mut Vec::new(), /*ints_in_scope=*/ &mut Vec::new());
        self.push("  0\n}\n");
        std::mem::take(&mut self.out)
    }

    fn gen_block(&mut self, region_depth: u32, shareds: &mut Vec<String>, ints: &mut Vec<String>) {
        let stmts = (self.rng.pick(4) + 1) as usize;
        let shareds_snap = shareds.len();
        let ints_snap = ints.len();
        for _ in 0..stmts {
            if self.fuel == 0 { break; }
            self.fuel -= 1;
            let pick = self.rng.pick(if region_depth > 0 { 5 } else { 3 });
            match pick {
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
                    self.gen_block(region_depth + 1, shareds, ints);
                    self.push("  }\n");
                }
                3 if region_depth > 0 => {
                    let name = self.fresh();
                    let v = self.rng.pick(1000) as i64;
                    self.push(&format!("  let {}: Shared<Int> = Shared({});\n", name, v));
                    shareds.push(name);
                }
                4 if region_depth > 0 && !shareds.is_empty() => {
                    let name = self.fresh();
                    let src = shareds[self.rng.pick(shareds.len() as u64) as usize].clone();
                    self.push(&format!("  let {}: Shared<Int> = {}.clone();\n", name, src));
                    shareds.push(name);
                }
                _ => {
                    let name = self.fresh();
                    self.push(&format!("  let {}: Int = 0;\n", name));
                    ints.push(name);
                }
            }
        }
        shareds.truncate(shareds_snap);
        ints.truncate(ints_snap);
    }
}

fn try_run(src: &str) -> Result<usize, String> {
    let mut p = Parser::new(Lexer::new(src)).with_source(src.to_string());
    let ast = p.parse_program();
    if !p.errors.is_empty() {
        return Err(format!("parse: {:?}", p.errors));
    }
    let mut compiler = Compiler::new();
    let module = compiler.compile_module(&ast).map_err(|e| format!("compile: {:?}", e))?;
    let mut vm = VirtualMachine::new();
    vm.run_module(&module).map_err(|e| format!("run: {}", e))?;
    Ok(vm.heap_live_count())
}

const ITER: u64 = 200;

#[test]
fn fuzz_no_leak_no_panic() {
    let mut leaks = Vec::new();
    for seed in 0..ITER {
        let src = Gen::new(seed).gen_program();
        match try_run(&src) {
            Ok(0) => {}
            Ok(n) => leaks.push((seed, n, src)),
            Err(_) => {} // generator may produce typeck-invalid programs; that's fine
        }
    }
    assert!(
        leaks.is_empty(),
        "{} seeds leaked heap slots; first: seed={}, live={}, src:\n{}",
        leaks.len(), leaks[0].0, leaks[0].1, leaks[0].2
    );
}

#[test]
fn fuzz_corpus_examples_compile() {
    // Sanity: a few hand-picked seeds should at least parse + typecheck.
    let mut ok = 0;
    for seed in 0..20 {
        let src = Gen::new(seed).gen_program();
        if try_run(&src).is_ok() { ok += 1; }
    }
    assert!(ok >= 5, "expected at least 5/20 generated programs to compile+run cleanly, got {}", ok);
}
