// Correctness fuzz: generate programs with known expected outputs, verify results.
// Each generator produces (source, expected_i64). Covers: int, float, static mut,
// multi-module, record pack/unpack, mut destructure, effect, match, loop, recursion.

use abrase::{compiler::Compiler, lexer::Lexer, loader::load_program, parser::Parser};
use myriad::{Value, VirtualMachine};
use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};

static DIR_CTR: AtomicU64 = AtomicU64::new(0);

// ── RNG ──────────────────────────────────────────────────────────────────────

struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(6364136223846793005).wrapping_add(1))
    }
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0
    }
    fn pick(&mut self, n: u64) -> u64 { self.next() % n }
    fn range(&mut self, lo: i64, hi: i64) -> i64 {
        lo + (self.pick((hi - lo) as u64) as i64)
    }
}

// ── run helpers ──────────────────────────────────────────────────────────────

fn run_src_expect(src: &str, expected: i64) -> Result<(), String> {
    let mut p = Parser::new(Lexer::new(src)).with_source(src.to_string());
    let ast = p.parse_program();
    if !p.errors.is_empty() {
        return Err(format!("parse:\n{}", p.pretty_print_errors()));
    }
    let mut c = Compiler::new().with_source(src.to_string());
    let module = c.compile_module(&ast).map_err(|_| c.pretty_print_errors())?;
    let mut vm = VirtualMachine::new().with_step_cap(5_000_000);
    let v = vm.run_module(&module).map_err(|e| format!("vm: {}", e))?;
    let got = v.as_int();
    if got != expected {
        return Err(format!("expected {}, got {}", expected, got));
    }
    Ok(())
}

fn run_files_expect(lib_src: &str, main_src: &str, expected: i64) -> Result<(), String> {
    let n = DIR_CTR.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir()
        .join(format!("abrase_correct_{}_{}", std::process::id(), n));
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    fs::write(dir.join("lib.abe"), lib_src).map_err(|e| e.to_string())?;
    fs::write(dir.join("main.abe"), main_src).map_err(|e| e.to_string())?;
    let entry = dir.join("main.abe");
    let result = (|| {
        let loaded = load_program(&entry).map_err(|e| format!("load: {:?}", e))?;
        let mut c = Compiler::new().with_source(loaded.entry_source.clone());
        let module = c.compile_module(&loaded.decls)
            .map_err(|_| loaded.render_errors(&c.errors))?;
        let mut vm = VirtualMachine::new().with_step_cap(5_000_000);
        let v = vm.run_module(&module).map_err(|e| format!("vm: {}", e))?;
        let got = v.as_int();
        if got != expected {
            return Err(format!("expected {}, got {}", expected, got));
        }
        Ok(())
    })();
    fs::remove_dir_all(&dir).ok();
    result
}

// ── generators ───────────────────────────────────────────────────────────────

/// Int arithmetic: a*b + c - d, verify to_i of float path matches int path.
fn gen_int_arith(rng: &mut Rng) -> (String, i64) {
    let a = rng.range(1, 50);
    let b = rng.range(1, 30);
    let c = rng.range(0, 100);
    let d = rng.range(0, 50);
    let expected = a * b + c - d;
    let src = format!(
        "fn main() -> Int {{ {a} * {b} + {c} - {d} }}"
    );
    (src, expected)
}

/// Float arithmetic: (a + b) * c - d, converted back to Int.
fn gen_float_arith(rng: &mut Rng) -> (String, i64) {
    let a = rng.range(1, 20);
    let b = rng.range(1, 20);
    let c = rng.range(1, 10);
    let d = rng.range(0, 20);
    let expected = (a + b) * c - d;
    let src = format!(r#"
fn main() -> Int {{
  let fa = {a}.to_f();
  let fb = {b}.to_f();
  let fc = {c}.to_f();
  let fd = {d}.to_f();
  ((fa + fb) * fc - fd).to_i()
}}
"#);
    (src, expected)
}

/// Match with guard conditions.
fn gen_int_match(rng: &mut Rng) -> (String, i64) {
    let b1 = rng.range(2, 8);
    let b2 = rng.range(b1 + 2, 18);
    let vals: Vec<i64> = (0..4).map(|_| rng.range(0, 22)).collect();
    let classify = |n: i64| if n < b1 { 1 } else if n < b2 { 2 } else { 3 };
    let expected = classify(vals[0])
        + classify(vals[1]) * 10
        + classify(vals[2]) * 100
        + classify(vals[3]) * 1000;
    let src = format!(r#"
fn classify(n: Int) -> Int {{
  match n {{
    _ if n < {b1} => 1,
    _ if n < {b2} => 2,
    _ => 3,
  }}
}}
fn main() -> Int {{
  classify({v0}) + classify({v1}) * 10 + classify({v2}) * 100 + classify({v3}) * 1000
}}
"#, b1=b1, b2=b2, v0=vals[0], v1=vals[1], v2=vals[2], v3=vals[3]);
    (src, expected)
}

/// Range patterns in match.
fn gen_range_match(rng: &mut Rng) -> (String, i64) {
    let vals: Vec<i64> = (0..5).map(|_| rng.range(0, 30)).collect();
    let bucket = |n: i64| if n < 10 { 0 } else if n < 20 { 1 } else { 2 };
    let expected: i64 = vals.iter().map(|&v| bucket(v)).sum();
    let src = format!(r#"
fn bucket(n: Int) -> Int {{
  match n {{
    0..10  => 0,
    10..20 => 1,
    _      => 2,
  }}
}}
fn main() -> Int {{
  bucket({v0}) + bucket({v1}) + bucket({v2}) + bucket({v3}) + bucket({v4})
}}
"#, v0=vals[0], v1=vals[1], v2=vals[2], v3=vals[3], v4=vals[4]);
    (src, expected)
}

/// Loop accumulation.
fn gen_loop(rng: &mut Rng) -> (String, i64) {
    let n = rng.range(3, 25);
    let step = rng.range(1, 4);
    let expected: i64 = (0..n).step_by(step as usize).sum();
    let src = format!(r#"
fn main() -> Int {{
  let mut acc = 0;
  let mut i = 0;
  while i < {n} {{ acc = acc + i; i = i + {step} }};
  acc
}}
"#);
    (src, expected)
}

/// Static mut accumulation across function calls.
fn gen_static_mut(rng: &mut Rng) -> (String, i64) {
    let start = rng.range(0, 5);
    let end = start + rng.range(2, 15);
    let expected: i64 = (start..end).sum();
    let src = format!(r#"
static mut ACC: Int = 0
fn add(n: Int) -> Unit {{ ACC = ACC + n }}
fn main() -> Int {{
  let mut i = {start};
  while i < {end} {{ add(i); i = i + 1 }};
  ACC
}}
"#);
    (src, expected)
}

/// Record pack / unpack / field access.
fn gen_record_pack(rng: &mut Rng) -> (String, i64) {
    let ax = rng.range(-8, 8);
    let ay = rng.range(-8, 8);
    let bx = rng.range(-8, 8);
    let by = rng.range(-8, 8);
    let s = rng.range(1, 5);
    let expected = (ax * s) * bx + (ay * s) * by;
    let src = format!(r#"
type Vec2 = {{ x: Int, y: Int }}
fn dot(a: Vec2, b: Vec2) -> Int {{ a.x * b.x + a.y * b.y }}
fn scale(v: Vec2, k: Int) -> Vec2 {{ Vec2 {{ x: v.x * k, y: v.y * k }} }}
fn main() -> Int {{
  let a = Vec2 {{ x: {ax}, y: {ay} }};
  let b = Vec2 {{ x: {bx}, y: {by} }};
  dot(scale(a, {s}), b)
}}
"#);
    (src, expected)
}

/// Mutable record destructure and write-back.
fn gen_record_mut_unpack(rng: &mut Rng) -> (String, i64) {
    let x = rng.range(1, 15);
    let y = rng.range(1, 15);
    let dx = rng.range(0, 8);
    let dy = rng.range(0, 8);
    let s = rng.range(1, 4);
    let expected = (x + dx) * s + (y + dy) * s;
    let src = format!(r#"
type Pt = {{ x: Int, y: Int }}
fn transform(p: Pt, dx: Int, dy: Int, scale: Int) -> Int {{
  let mut Pt {{ x, y }} = p;
  x = (x + dx) * scale;
  y = (y + dy) * scale;
  x + y
}}
fn main() -> Int {{
  transform(Pt {{ x: {x}, y: {y} }}, {dx}, {dy}, {s})
}}
"#);
    (src, expected)
}

/// Effect handler counter.
fn gen_effect_counter(rng: &mut Rng) -> (String, i64) {
    let n = rng.range(1, 20) as i64;
    let expected = n;
    let src = format!(r#"
effect Ctr {{ op inc() -> Unit }}
fn run_n(n: Int) -> <Ctr> Unit {{
  let mut i = 0;
  while i < n {{ Ctr.inc(); i = i + 1 }}
}}
fn main() -> Int {{
  let mut total = 0;
  handle run_n({n}) {{
    return _ => total,
    Ctr.inc => {{ total = total + 1; resume(()) }}
  }}
}}
"#);
    (src, expected)
}

/// Effect handler accumulator (passes a value via resume).
fn gen_effect_accumulate(rng: &mut Rng) -> (String, i64) {
    let vals: Vec<i64> = (0..5).map(|_| rng.range(1, 10)).collect();
    let expected: i64 = vals.iter().sum();
    let items = vals.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", ");
    let src = format!(r#"
effect Adder {{ op push(n: Int) -> Unit }}
fn feed() -> <Adder> Unit {{
  let arr = [{items}];
  let mut i = 0;
  while i < 5 {{ Adder.push(arr[i]); i = i + 1 }}
}}
fn main() -> Int {{
  let mut sum = 0;
  handle feed() {{
    return _  => sum,
    Adder.push n => {{ sum = sum + n; resume(()) }}
  }}
}}
"#);
    (src, expected)
}

/// Recursion: fibonacci.
fn gen_recursion_fib(rng: &mut Rng) -> (String, i64) {
    let n = rng.range(1, 12) as usize;
    let fib = {
        let mut a = 0i64; let mut b = 1i64;
        for _ in 0..n { let t = a + b; a = b; b = t; }
        a
    };
    let src = format!(r#"
fn fib(n: Int) -> Int {{
  if n <= 1 {{ n }} else {{ fib(n - 1) + fib(n - 2) }}
}}
fn main() -> Int {{ fib({n}) }}
"#);
    (src, fib)
}

/// Recursion: sum 1..n.
fn gen_recursion_sum(rng: &mut Rng) -> (String, i64) {
    let n = rng.range(1, 30);
    let expected = n * (n + 1) / 2;
    let src = format!(r#"
fn sum_to(n: Int) -> Int {{
  if n <= 0 {{ 0 }} else {{ n + sum_to(n - 1) }}
}}
fn main() -> Int {{ sum_to({n}) }}
"#);
    (src, expected)
}

/// Combined: static mut + match + loop.
fn gen_static_match_loop(rng: &mut Rng) -> (String, i64) {
    let n = rng.range(5, 20);
    let b1 = 5i64; let b2 = 10i64; let b3 = 15i64;
    let mut buckets = [0i64; 4];
    for i in 0..n {
        let b = if i < b1 { 0 } else if i < b2 { 1 } else if i < b3 { 2 } else { 3 };
        buckets[b] += 1;
    }
    let expected = buckets[0] * 1000 + buckets[1] * 100 + buckets[2] * 10 + buckets[3];
    let src = format!(r#"
static mut BUCKET: Array<Int> = [0; 4]
fn classify(n: Int) -> Int {{
  match n {{
    _ if n < {b1} => 0,
    _ if n < {b2} => 1,
    _ if n < {b3} => 2,
    _ => 3,
  }}
}}
fn main() -> Int {{
  let mut i = 0;
  while i < {n} {{
    let b = classify(i);
    BUCKET[b] = BUCKET[b] + 1;
    i = i + 1
  }};
  BUCKET[0] * 1000 + BUCKET[1] * 100 + BUCKET[2] * 10 + BUCKET[3]
}}
"#);
    (src, expected)
}

/// Combined: record array + loop + float field.
fn gen_record_array_float(rng: &mut Rng) -> (String, i64) {
    let n = rng.range(2, 6) as usize;
    let xs: Vec<i64> = (0..n).map(|_| rng.range(1, 10)).collect();
    let ys: Vec<i64> = (0..n).map(|_| rng.range(1, 10)).collect();
    let dx = rng.range(1, 5);
    let expected: i64 = xs.iter().zip(ys.iter()).map(|(x, y)| (x + dx) * y).sum();
    let inits: String = xs.iter().zip(ys.iter())
        .map(|(x, y)| format!("Ent {{ x: {x}.to_f(), y: {y} }}"))
        .collect::<Vec<_>>()
        .join(", ");
    let src = format!(r#"
type Ent = {{ x: Float, y: Int }}
fn main() -> Int {{
  let arr = [{inits}];
  let mut acc = 0;
  let mut i = 0;
  while i < {n} {{
    let e = arr[i];
    acc = acc + (e.x.to_i() + {dx}) * e.y;
    i = i + 1
  }};
  acc
}}
"#);
    (src, expected)
}

/// Multi-module: statics + function calls across modules.
fn gen_multi_module(rng: &mut Rng) -> (String, String, i64) {
    let base = rng.range(1, 20);
    let vals: Vec<i64> = (0..4).map(|_| rng.range(0, 10)).collect();
    // accumulate(n) adds n + base to COUNTER each call
    let expected: i64 = vals.iter().map(|v| v + base).sum();
    let lib = format!(r#"
pub static BASE: Int = {base}
pub static mut COUNTER: Int = 0
pub fn accumulate(n: Int) -> Unit {{ COUNTER = COUNTER + n + BASE }}
pub fn result() -> Int {{ COUNTER }}
fn main() -> Int {{ 0 }}
"#);
    let calls: String = vals.iter()
        .map(|v| format!("  accumulate({v});\n"))
        .collect();
    let main = format!(
        "use lib::{{BASE, COUNTER, accumulate, result}}\nfn main() -> Int {{\n{calls}  result()\n}}\n"
    );
    (lib, main, expected)
}

/// Multi-module: record type defined in lib, used in main.
fn gen_multi_module_record(rng: &mut Rng) -> (String, String, i64) {
    let ax = rng.range(1, 10);
    let ay = rng.range(1, 10);
    let bx = rng.range(1, 10);
    let by = rng.range(1, 10);
    let expected = ax * bx + ay * by; // dot product
    let lib = format!(r#"
pub type Vec2 = {{ x: Int, y: Int }}
pub fn dot(a: Vec2, b: Vec2) -> Int {{ a.x * b.x + a.y * b.y }}
fn main() -> Int {{ 0 }}
"#);
    let main = format!(r#"
use lib::{{Vec2, dot}}
fn main() -> Int {{
  let a = Vec2 {{ x: {ax}, y: {ay} }};
  let b = Vec2 {{ x: {bx}, y: {by} }};
  dot(a, b)
}}
"#);
    (lib, main, expected)
}

// ── test harness ─────────────────────────────────────────────────────────────

type Gen = fn(&mut Rng) -> (String, i64);
type MMGen = fn(&mut Rng) -> (String, String, i64);

const SINGLE_GENS: &[(&str, Gen)] = &[
    ("int_arith",          gen_int_arith),
    ("float_arith",        gen_float_arith),
    ("int_match",          gen_int_match),
    ("range_match",        gen_range_match),
    ("loop",               gen_loop),
    ("static_mut",         gen_static_mut),
    ("record_pack",        gen_record_pack),
    ("record_mut_unpack",  gen_record_mut_unpack),
    ("effect_counter",     gen_effect_counter),
    ("effect_accumulate",  gen_effect_accumulate),
    ("recursion_fib",      gen_recursion_fib),
    ("recursion_sum",      gen_recursion_sum),
    ("static_match_loop",  gen_static_match_loop),
    ("record_array_float", gen_record_array_float),
];

const MM_GENS: &[(&str, MMGen)] = &[
    ("multi_module_static", gen_multi_module),
    ("multi_module_record", gen_multi_module_record),
];

const ITERS: u64 = 300;

#[test]
fn fuzz_correctness() {
    let mut failures: Vec<(u64, &str, String, String)> = Vec::new();

    for seed in 0..ITERS {
        for (name, f) in SINGLE_GENS {
            let mut rng = Rng::new(seed * 97 + name.len() as u64);
            let (src, expected) = f(&mut rng);
            if let Err(e) = run_src_expect(&src, expected) {
                failures.push((seed, name, e, src));
            }
        }
    }

    if !failures.is_empty() {
        for (seed, name, err, src) in &failures {
            eprintln!("--- seed={} gen={} ---\n{}\nError: {}", seed, name, src, err);
        }
        panic!("fuzz_correctness: {} failure(s)", failures.len());
    }
}

#[test]
fn fuzz_correctness_multi_module() {
    let mut failures: Vec<(u64, &str, String)> = Vec::new();

    for seed in 0..150u64 {
        for (name, f) in MM_GENS {
            let mut rng = Rng::new(seed * 53 + name.len() as u64);
            let (lib, main, expected) = f(&mut rng);
            if let Err(e) = run_files_expect(&lib, &main, expected) {
                let combined = format!("--- lib ---\n{}--- main ---\n{}", lib, main);
                failures.push((seed, name, format!("{}\n{}", e, combined)));
            }
        }
    }

    if !failures.is_empty() {
        for (seed, name, msg) in &failures {
            eprintln!("--- seed={} gen={} ---\n{}", seed, name, msg);
        }
        panic!("fuzz_correctness_multi_module: {} failure(s)", failures.len());
    }
}
