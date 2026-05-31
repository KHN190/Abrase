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

/// Variant: construction + match on each arm.
fn gen_variant_match(rng: &mut Rng) -> (String, i64) {
    // type Shape = Circle(Int) | Rect(Int, Int) | Point
    let r  = rng.range(1, 15);
    let w  = rng.range(1, 15);
    let h  = rng.range(1, 15);
    // area: Circle(r)→r*r, Rect(w,h)→w*h, Point→0
    let expected = r*r + w*h + 0;
    let src = format!(r#"
type Shape = Circle(Int) | Rect(Int, Int) | Point
fn area(s: Shape) -> Int {{
  match s {{
    Circle(r)    => r * r,
    Rect(w, h)   => w * h,
    Point        => 0,
  }}
}}
fn main() -> Int {{
  area(Circle({r})) + area(Rect({w}, {h})) + area(Point)
}}
"#);
    (src, expected)
}

/// Variant: match with guard selects correct arm.
fn gen_variant_guard_match(rng: &mut Rng) -> (String, i64) {
    let vals: Vec<i64> = (0..4).map(|_| rng.range(0, 20)).collect();
    let thresh = rng.range(5, 15);
    // Some(n): n > thresh → n, else → 0; None → -1
    let classify = |v: i64| v;  // all are Some(v), result = v if > thresh else 0
    let expected: i64 = vals.iter().map(|&v| {
        if v > thresh { v } else { 0 }
    }).sum();
    let src = format!(r#"
type Opt = None | Some(Int)
fn extract(o: Opt) -> Int {{
  match o {{
    Some(n) if n > {thresh} => n,
    Some(_) => 0,
    None    => -1,
  }}
}}
fn main() -> Int {{
  extract(Some({v0})) + extract(Some({v1})) + extract(Some({v2})) + extract(Some({v3}))
}}
"#, thresh=thresh, v0=vals[0], v1=vals[1], v2=vals[2], v3=vals[3]);
    (src, expected)
}

/// Variant: static mut array of variants, pack write then read.
fn gen_variant_static_array(rng: &mut Rng) -> (String, i64) {
    let vals: Vec<i64> = (0..4).map(|_| rng.range(1, 20)).collect();
    let expected: i64 = vals.iter().sum();
    let writes: String = vals.iter().enumerate()
        .map(|(i, v)| format!("  TAGS[{i}] = Some({v});\n"))
        .collect();
    let src = format!(r#"
type Tag = None | Some(Int)
static mut TAGS: Array<Tag> = [None; 4]
fn read_sum() -> Int {{
  let mut acc = 0;
  let mut i = 0;
  while i < 4 {{
    match TAGS[i] {{
      Some(n) => {{ acc = acc + n }},
      None    => (),
    }};
    i = i + 1
  }};
  acc
}}
fn main() -> Int {{
{writes}  read_sum()
}}
"#);
    (src, expected)
}

/// for loop: range accumulation.
fn gen_for_loop(rng: &mut Rng) -> (String, i64) {
    let start = rng.range(0, 5);
    let end   = start + rng.range(3, 15);
    let mul   = rng.range(1, 4);
    let expected: i64 = (start..end).map(|i| i * mul).sum();
    let src = format!(r#"
fn main() -> Int {{
  let mut acc = 0;
  for i in {start}..{end} {{ acc = acc + i * {mul} }};
  acc
}}
"#);
    (src, expected)
}

/// for loop with break.
fn gen_for_break(rng: &mut Rng) -> (String, i64) {
    let n     = rng.range(5, 20);
    let stop  = rng.range(2, n - 1);
    let expected: i64 = (0..stop).sum();  // accumulates 0..stop, breaks at stop
    let src = format!(r#"
fn main() -> Int {{
  let mut acc = 0;
  for i in 0..{n} {{
    if i == {stop} {{ break }};
    acc = acc + i
  }};
  acc
}}
"#);
    (src, expected)
}

/// loop { break value }.
fn gen_loop_break_value(rng: &mut Rng) -> (String, i64) {
    let n        = rng.range(3, 15);
    let target   = rng.range(1, n - 1);
    let expected = target * target;
    let src = format!(r#"
fn main() -> Int {{
  let mut i = 0;
  let result = loop {{
    if i == {target} {{ break i * i }};
    i = i + 1
  }};
  result
}}
"#);
    (src, expected)
}

/// Closure: capture and call.
fn gen_closure_capture(rng: &mut Rng) -> (String, i64) {
    let base = rng.range(1, 20);
    let step = rng.range(1, 10);
    let n    = rng.range(2, 8);
    // add(i) = base + i*step; sum over 0..n
    let expected: i64 = (0..n).map(|i| base + i * step).sum();
    let src = format!(r#"
fn main() -> Int {{
  let base = {base};
  let step = {step};
  let add = |i| base + i * step;
  let mut acc = 0;
  let mut i = 0;
  while i < {n} {{ acc = acc + add(i); i = i + 1 }};
  acc
}}
"#);
    (src, expected)
}

/// Exception: throw + handle exn.
fn gen_exception(rng: &mut Rng) -> (String, i64) {
    let good = rng.range(2, 10);
    let bad  = 0i64;
    let expected = good / 2; // divide(good, 2) ok; divide(bad, 0) → Err → 0
    let src = format!(r#"
fn divide(x: Int, y: Int) -> <exn<Int>> Int {{
  if y == 0 {{ throw -1 }} else {{ x / y }}
}}
fn safe_div(x: Int, y: Int) -> Int {{
  handle divide(x, y) {{
    return v  => v,
    exn _     => 0,
  }}
}}
fn main() -> Int {{
  safe_div({good}, 2) + safe_div({bad}, 0)
}}
"#);
    (src, expected)
}

/// Effect + static: handler accumulates into a static.
fn gen_effect_static(rng: &mut Rng) -> (String, i64) {
    let n = rng.range(2, 12);
    let expected = n * (n - 1) / 2; // sum 0..n-1
    let src = format!(r#"
static mut TOTAL: Int = 0
effect Acc {{ op add(x: Int) -> Unit }}
fn fire(n: Int) -> <Acc> Unit {{
  let mut i = 0;
  while i < n {{ Acc.add(i); i = i + 1 }}
}}
fn main() -> Int {{
  handle fire({n}) {{
    return _   => TOTAL,
    Acc.add x  => {{ TOTAL = TOTAL + x; resume(()) }}
  }}
}}
"#);
    (src, expected)
}

/// Char: to_i / to_c round-trip.
fn gen_char_ops(rng: &mut Rng) -> (String, i64) {
    let n     = rng.range(0, 26) as u8;
    let code  = b'A' + n;
    let expected = code as i64;
    let src = format!(r#"
fn main() -> Int {{
  let c: Char = {code}.to_c();
  c.to_i()
}}
"#);
    (src, expected)
}

/// Tuple destructure.
fn gen_tuple_destructure(rng: &mut Rng) -> (String, i64) {
    let a = rng.range(1, 50);
    let b = rng.range(1, 50);
    let c = rng.range(1, 20);
    let expected = (a + c) * (b - c);
    let src = format!(r#"
fn swap_add(p: (Int, Int), d: Int) -> Int {{
  let (x, y) = p;
  (x + d) * (y - d)
}}
fn main() -> Int {{ swap_add(({a}, {b}), {c}) }}
"#);
    (src, expected)
}

/// Recursion + static: recursive fn reads a static multiplier.
fn gen_recursion_static(rng: &mut Rng) -> (String, i64) {
    let mul = rng.range(1, 5);
    let n   = rng.range(1, 8);
    // sum_mul(n) = MUL * (1 + 2 + ... + n) = MUL * n*(n+1)/2
    let expected = mul * n * (n + 1) / 2;
    let src = format!(r#"
static MUL: Int = {mul}
fn sum_mul(n: Int) -> Int {{
  if n <= 0 {{ 0 }} else {{ MUL * n + sum_mul(n - 1) }}
}}
fn main() -> Int {{ sum_mul({n}) }}
"#);
    (src, expected)
}

/// Multi-module + variant: lib exports a variant type and match function.
/// Multi-module + variant: lib owns the type and constructors, main calls factory fns.
fn gen_multi_module_variant(rng: &mut Rng) -> (String, String, i64) {
    let present_val = rng.range(1, 30);
    let default_val = rng.range(1, 20);
    let expected = present_val + default_val; // get_or(present) + get_or(absent, default)
    let lib = format!(r#"
pub type Opt = Absent | Present(Int)
pub fn make_present(n: Int) -> Opt {{ Present(n) }}
pub fn make_absent() -> Opt {{ Absent }}
pub fn get_or(o: Opt, def: Int) -> Int {{
  match o {{ Present(n) => n, _ => def }}
}}
fn main() -> Int {{ 0 }}
"#);
    let main = format!(r#"
use lib::{{Opt, make_present, make_absent, get_or}}
fn main() -> Int {{
  let a = get_or(make_present({present_val}), 0);
  let b = get_or(make_absent(), {default_val});
  a + b
}}
"#);
    (lib, main, expected)
}

/// Array literal, indexing, in-place mutation.
fn gen_array_ops(rng: &mut Rng) -> (String, i64) {
    let vals: Vec<i64> = (0..5).map(|_| rng.range(1, 20)).collect();
    let add = rng.range(1, 10);
    let idx = rng.range(0, 5) as usize;
    let mut expected_vals = vals.clone();
    expected_vals[idx] += add;
    let expected: i64 = expected_vals.iter().sum();
    let items = vals.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", ");
    let src = format!(r#"
fn main() -> Int {{
  let mut arr = [{items}];
  arr[{idx}] = arr[{idx}] + {add};
  arr[0] + arr[1] + arr[2] + arr[3] + arr[4]
}}
"#);
    (src, expected)
}

/// Bitwise operations: &, |, ^, <<, >>.
fn gen_bitwise(rng: &mut Rng) -> (String, i64) {
    let a = rng.range(0, 255);
    let b = rng.range(0, 255);
    let shift = rng.range(0, 4);
    let expected = (a & b) + (a | b) + (a ^ b) + (a << shift) + (a >> shift);
    let src = format!(r#"
fn main() -> Int {{
  let a = {a};
  let b = {b};
  let sh = {shift};
  (a & b) + (a | b) + (a ^ b) + (a << sh) + (a >> sh)
}}
"#);
    (src, expected)
}

/// Move closure captures ownership of local array.
fn gen_move_closure(rng: &mut Rng) -> (String, i64) {
    let offset = rng.range(1, 15);
    let vals: Vec<i64> = (0..4).map(|_| rng.range(1, 10)).collect();
    let expected: i64 = vals.iter().map(|v| v + offset).sum();
    let items = vals.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", ");
    let src = format!(r#"
fn main() -> Int {{
  let off = {offset};
  let arr = [{items}];
  let f = move |x| x + off;
  f(arr[0]) + f(arr[1]) + f(arr[2]) + f(arr[3])
}}
"#);
    (src, expected)
}

/// Region block: value escapes to outer scope.
fn gen_region_escape(rng: &mut Rng) -> (String, i64) {
    let a = rng.range(1, 20);
    let b = rng.range(1, 20);
    let c = rng.range(1, 10);
    let expected = (a + b) * c;
    let src = format!(r#"
type Pt = {{ x: Int, y: Int }}
fn main() -> Int {{
  let p = region {{ Pt {{ x: {a}, y: {b} }} }};
  (p.x + p.y) * {c}
}}
"#);
    (src, expected)
}

/// Multi-frame: static Float array across call_export frames.
fn gen_multi_frame_float(rng: &mut Rng) -> (String, Vec<Value>, i64) {
    let vals: Vec<f64> = (0..4).map(|_| rng.range(1, 10) as f64).collect();
    let inc = rng.range(1, 5) as f64;
    let slot = rng.range(0, 4) as usize;
    let expected = (vals[slot] + inc) as i64;
    let items = vals.iter().map(|v| format!("{}.0", v)).collect::<Vec<_>>().join(", ");
    let src = format!(r#"
static mut FA: Array<Float> = [{items}]

pub fn add_to(slot: Int, delta: Float) -> Unit {{
  FA[slot] = FA[slot] + delta
}}

pub fn read_result() -> Int {{
  FA[{slot}].to_i()
}}

fn main() -> Int {{ 0 }}
"#);
    (src, vec![Value::from_int(slot as i64), Value::from_float(inc)], expected)
}

/// Multi-frame: for loop + static across call_export.
fn gen_multi_frame_for_loop(rng: &mut Rng) -> (String, Vec<Value>, i64) {
    let n = rng.range(2, 10);
    let expected: i64 = (0..n).sum(); // each call adds one pass of 0..n to TOTAL
    let src = format!(r#"
static mut TOTAL: Int = 0

pub fn accumulate(n: Int) -> Unit {{
  for i in 0..n {{ TOTAL = TOTAL + i }}
}}

pub fn get_total() -> Int {{ TOTAL }}

fn main() -> Int {{ 0 }}
"#);
    (src, vec![Value::from_int(n)], expected)
}

/// Multi-module + effect: lib exports a stateful effect runner.
fn gen_multi_module_effect(rng: &mut Rng) -> (String, String, i64) {
    let n = rng.range(2, 10);
    let expected = n * (n + 1) / 2; // 1+2+...+n
    let lib = format!(r#"
effect Counter {{ op bump(x: Int) -> Unit }}
pub fn run_sum(n: Int) -> <Counter> Unit {{
  let mut i = 1;
  while i <= n {{ Counter.bump(i); i = i + 1 }}
}}
fn main() -> Int {{ 0 }}
"#);
    let main = format!(r#"
use lib::{{run_sum}}
fn main() -> Int {{
  let mut total = 0;
  handle run_sum({n}) {{
    return _     => total,
    Counter.bump x => {{ total = total + x; resume(()) }}
  }}
}}
"#);
    (lib, main, expected)
}

/// Multi-frame: static array of variants, pack write then call_export read.
fn gen_static_variant_array_multiframe(rng: &mut Rng) -> (String, Vec<Value>, i64) {
    let v    = rng.range(1, 50);
    let slot = rng.range(0, 4);
    let expected = v;
    let src = format!(r#"
type Tag = None | Some(Int)
static mut TAGS: Array<Tag> = [None; 4]

pub fn spawn(slot: Int, val: Int) -> Unit {{
  TAGS[slot] = Some(val)
}}

pub fn read() -> Int {{
  match TAGS[{slot}] {{
    Some(n) => n,
    None    => -1,
  }}
}}

fn main() -> Int {{ 0 }}
"#);
    (src, vec![Value::from_int(slot), Value::from_int(v)], expected)
}

// ── test harness ─────────────────────────────────────────────────────────────

type Gen = fn(&mut Rng) -> (String, i64);
type MMGen = fn(&mut Rng) -> (String, String, i64);
type MFGen = fn(&mut Rng) -> (String, Vec<Value>, i64);

fn run_multi_frame(src: &str, export: &str, args: &[Value], read_export: &str) -> Result<i64, String> {
    let mut p = Parser::new(Lexer::new(src)).with_source(src.to_string());
    let ast = p.parse_program();
    if !p.errors.is_empty() { return Err(format!("parse: {}", p.pretty_print_errors())); }
    let mut c = Compiler::new().with_source(src.to_string());
    let module = c.compile_module(&ast).map_err(|_| c.pretty_print_errors())?;
    let mut vm = VirtualMachine::new().with_step_cap(5_000_000);
    vm.run_module(&module).map_err(|e| format!("vm: {}", e))?;
    vm.call_export(&module, export, args).map_err(|e| format!("call {}: {}", export, e))?;
    let v = vm.call_export(&module, read_export, &[]).map_err(|e| format!("read: {}", e))?;
    Ok(v.as_int())
}

const SINGLE_GENS: &[(&str, Gen)] = &[
    ("int_arith",           gen_int_arith),
    ("float_arith",         gen_float_arith),
    ("int_match",           gen_int_match),
    ("range_match",         gen_range_match),
    ("loop",                gen_loop),
    ("static_mut",          gen_static_mut),
    ("record_pack",         gen_record_pack),
    ("record_mut_unpack",   gen_record_mut_unpack),
    ("effect_counter",      gen_effect_counter),
    ("effect_accumulate",   gen_effect_accumulate),
    ("recursion_fib",       gen_recursion_fib),
    ("recursion_sum",       gen_recursion_sum),
    ("static_match_loop",   gen_static_match_loop),
    ("record_array_float",  gen_record_array_float),
    ("variant_match",       gen_variant_match),
    ("variant_guard_match", gen_variant_guard_match),
    ("variant_static_array",gen_variant_static_array),
    ("for_loop",            gen_for_loop),
    ("for_break",           gen_for_break),
    ("loop_break_value",    gen_loop_break_value),
    ("closure_capture",     gen_closure_capture),
    ("move_closure",        gen_move_closure),
    ("exception",           gen_exception),
    ("effect_static",       gen_effect_static),
    ("char_ops",            gen_char_ops),
    ("tuple_destructure",   gen_tuple_destructure),
    ("recursion_static",    gen_recursion_static),
    ("array_ops",           gen_array_ops),
    ("bitwise",             gen_bitwise),
    ("region_escape",       gen_region_escape),
];

const MM_GENS: &[(&str, MMGen)] = &[
    ("multi_module_static",  gen_multi_module),
    ("multi_module_record",  gen_multi_module_record),
    ("multi_module_variant", gen_multi_module_variant),
    ("multi_module_effect",  gen_multi_module_effect),
];

const MF_GENS: &[(&str, MFGen)] = &[
    ("multi_frame_float",    gen_multi_frame_float),
    ("multi_frame_for_loop", gen_multi_frame_for_loop),
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
        for (name, f) in MF_GENS {
            let mut rng = Rng::new(seed * 61 + name.len() as u64);
            let (src, args, expected) = f(&mut rng);
            // float: add_to(slot, delta) then read_slot(slot)
            // for_loop: accumulate(n) then get_total()
            let (write_fn, read_fn): (&str, &str) = if name.contains("float") {
                ("add_to", "read_result")
            } else {
                ("accumulate", "get_total")
            };
            let write_args: Vec<Value> = args.clone();
            let result = run_multi_frame(&src, write_fn, &write_args, read_fn);
            match result {
                Ok(got) if got == expected => {}
                Ok(got) => failures.push((seed, name,
                    format!("expected {}, got {}", expected, got), src)),
                Err(e) => failures.push((seed, name, e, src)),
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

fn run_multiframe_expect(src: &str, spawn_args: &[Value], expected: i64) -> Result<(), String> {
    let mut p = Parser::new(Lexer::new(src)).with_source(src.to_string());
    let ast = p.parse_program();
    if !p.errors.is_empty() { return Err(format!("parse:\n{}", p.pretty_print_errors())); }
    let mut c = Compiler::new().with_source(src.to_string());
    let module = c.compile_module(&ast).map_err(|_| c.pretty_print_errors())?;
    let mut vm = VirtualMachine::new();
    vm.run_module(&module).map_err(|e| format!("vm: {}", e))?;
    vm.call_export(&module, "spawn", spawn_args)
        .map_err(|e| format!("spawn: {}", e))?;
    let v = vm.call_export(&module, "read", &[])
        .map_err(|e| format!("read: {}", e))?;
    let got = v.as_int();
    if got != expected { return Err(format!("expected {}, got {}", expected, got)); }
    Ok(())
}

fn gen_static_record_pack_multiframe(rng: &mut Rng) -> (String, Vec<Value>, i64) {
    let a = rng.range(1, 50);
    let b = rng.range(1, 50);
    let slot = rng.range(0, 4);
    let expected = a + b;
    let src = format!(r#"
type R = {{ a: Int, b: Int }}
static mut ARR: Array<R> = [R {{ a: 0, b: 0 }}; 4]

pub fn spawn(slot: Int, a: Int, b: Int) -> Unit {{
  ARR[slot] = R {{ a: a, b: b }}
}}

pub fn read() -> Int {{
  ARR[{slot}].a + ARR[{slot}].b
}}

fn main() -> Int {{ 0 }}
"#);
    (src, vec![Value::from_int(slot), Value::from_int(a), Value::from_int(b)], expected)
}

fn gen_static_record_pack_churn(rng: &mut Rng) -> (String, Vec<Value>, i64) {
    // Multiple slots written then read; each value must survive region pop.
    let vals: Vec<(i64, i64)> = (0..4).map(|_| (rng.range(1, 20), rng.range(1, 20))).collect();
    let expected: i64 = vals.iter().map(|(a, b)| a + b).sum();
    let spawns: String = vals.iter().enumerate()
        .map(|(i, (a, b))| format!("  ARR[{i}] = R {{ a: {a}, b: {b} }};\n"))
        .collect();
    let src = format!(r#"
type R = {{ a: Int, b: Int }}
static mut ARR: Array<R> = [R {{ a: 0, b: 0 }}; 4]

pub fn spawn_all() -> Unit {{
{spawns}}}

pub fn sum_all() -> Int {{
  ARR[0].a + ARR[0].b + ARR[1].a + ARR[1].b +
  ARR[2].a + ARR[2].b + ARR[3].a + ARR[3].b
}}

fn main() -> Int {{ 0 }}
"#);
    (src, vec![], expected)
}

#[test]
fn fuzz_static_record_pack_multiframe() {
    let mut failures: Vec<(u64, String)> = Vec::new();

    for seed in 0..300u64 {
        let mut rng = Rng::new(seed * 31 + 7);
        let (src, args, expected) = gen_static_record_pack_multiframe(&mut rng);
        if let Err(e) = run_multiframe_expect(&src, &args, expected) {
            failures.push((seed, format!("{}\n{}", e, src)));
        }

        let mut rng2 = Rng::new(seed * 17 + 3);
        let (src2, _, expected2) = gen_static_record_pack_churn(&mut rng2);
        // For churn test: run spawn_all then sum_all
        let result = (|| {
            let mut p = Parser::new(Lexer::new(&src2)).with_source(src2.clone());
            let ast = p.parse_program();
            if !p.errors.is_empty() { return Err(format!("parse: {}", p.pretty_print_errors())); }
            let mut c = Compiler::new().with_source(src2.clone());
            let module = c.compile_module(&ast).map_err(|_| c.pretty_print_errors())?;
            let mut vm = VirtualMachine::new();
            vm.run_module(&module).map_err(|e| format!("vm: {}", e))?;
            vm.call_export(&module, "spawn_all", &[])
                .map_err(|e| format!("spawn_all: {}", e))?;
            let v = vm.call_export(&module, "sum_all", &[])
                .map_err(|e| format!("sum_all: {}", e))?;
            let got = v.as_int();
            if got != expected2 { return Err(format!("expected {}, got {}", expected2, got)); }
            Ok(())
        })();
        if let Err(e) = result {
            failures.push((seed, format!("churn: {}\n{}", e, src2)));
        }
    }

    if !failures.is_empty() {
        for (seed, msg) in &failures {
            eprintln!("--- seed={} ---\n{}", seed, msg);
        }
        panic!("fuzz_static_record_pack_multiframe: {} failure(s)", failures.len());
    }
}

#[test]
fn fuzz_static_variant_array_multiframe() {
    let mut failures: Vec<(u64, String)> = Vec::new();
    for seed in 0..300u64 {
        let mut rng = Rng::new(seed * 41 + 11);
        let (src, args, expected) = gen_static_variant_array_multiframe(&mut rng);
        if let Err(e) = run_multiframe_expect(&src, &args, expected) {
            failures.push((seed, format!("{}\n{}", e, src)));
        }
    }
    if !failures.is_empty() {
        for (seed, msg) in &failures { eprintln!("--- seed={} ---\n{}", seed, msg); }
        panic!("fuzz_static_variant_array_multiframe: {} failure(s)", failures.len());
    }
}
