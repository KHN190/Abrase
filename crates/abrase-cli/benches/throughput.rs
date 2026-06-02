use abrase::compiler::Compiler;
use abrase::lexer::Lexer;
use abrase::parser::Parser;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use myriad::{Host, Value, VirtualMachine};
use polka::Module;

fn compile(src: &str) -> Module {
    let mut parser = Parser::new(Lexer::new(src)).with_source(src.to_string());
    let ast = parser.parse_program();
    assert!(parser.errors.is_empty(), "{}", parser.pretty_print_errors());
    let mut compiler = Compiler::new().with_source(src.to_string());
    compiler.compile_module(&ast)
        .unwrap_or_else(|_| panic!("{}", compiler.pretty_print_errors()))
}

fn run(module: &Module) -> Value {
    let mut vm = VirtualMachine::new();
    vm.run_module(module).expect("run failed")
}

fn steps_of(module: &Module) -> u64 {
    let mut vm = VirtualMachine::new();
    vm.run_module(module).expect("run failed");
    vm.steps()
}

const ARITH_LOOP: &str = r#"
fn main() -> Int {
  let mut i = 0;
  let mut acc = 0;
  while i < 100000 { acc = acc + i * 2; i = i + 1 };
  acc
}
"#;

const FIB_REC: &str = r#"
fn fib(n: Int) -> Int {
  if n <= 1 { n } else { fib(n - 1) + fib(n - 2) }
}
fn main() -> Int { fib(28) }
"#;

const ALLOC_LOOP: &str = r#"
fn main() -> Int {
  let mut i = 0;
  let mut acc = 0;
  while i < 5000 {
    let s = Shared(i + 1);
    acc = acc + *s;
    i = i + 1
  };
  acc
}
"#;

const STATIC_ACCESS: &str = r#"
static A: Int = 1
static B: Int = 2
static C: Int = 3
fn main() -> Int {
  let mut acc = 0;
  let mut i = 0;
  while i < 50000 { acc = acc + A + B + C; i = i + 1 };
  acc
}
"#;

const EFFECTS: &str = r#"
effect tick { op step() -> Int }
fn body(n: Int) -> <tick> Int {
  let mut acc = 0;
  let mut i = 0;
  while i < n { acc = acc + tick.step(); i = i + 1 };
  acc
}
fn main() -> Int {
  handle body(10000) {
    return v => v,
    tick.step => resume(1)
  }
}
"#;

const RECORDS: &str = r#"
type Vec3 = { x: Float, y: Float, z: Float }
fn dot(a: Vec3, b: Vec3) -> Float {
  a.x * b.x + a.y * b.y + a.z * b.z
}
fn main() -> Float {
  let mut acc = 0.0;
  let mut i = 0;
  while i < 20000 {
    let a = Vec3 { x: 1.0, y: 2.0, z: 3.0 };
    let b = Vec3 { x: 4.0, y: 5.0, z: 6.0 };
    acc = acc + dot(a, b);
    i = i + 1
  };
  acc
}
"#;

const FLOAT_ARITH: &str = r#"
fn main() -> Float {
  let mut x = 1.0;
  let mut i = 0;
  while i < 100000 {
    x = x * 1.000001 + 0.000001;
    i = i + 1
  };
  x
}
"#;

struct Prog { name: &'static str, src: &'static str }

fn bench_vm(c: &mut Criterion) {
    let progs = [
        Prog { name: "arith_loop",    src: ARITH_LOOP    },
        Prog { name: "fib_rec",       src: FIB_REC       },
        Prog { name: "alloc_loop",    src: ALLOC_LOOP     },
        Prog { name: "static_access", src: STATIC_ACCESS  },
        Prog { name: "effects",       src: EFFECTS        },
        Prog { name: "records",       src: RECORDS        },
        Prog { name: "float_arith",   src: FLOAT_ARITH    },
    ];

    let mut group = c.benchmark_group("vm_run");
    for p in &progs {
        let module = compile(p.src);
        let steps = steps_of(&module);
        group.throughput(Throughput::Elements(steps));
        group.bench_with_input(BenchmarkId::from_parameter(p.name), p.name, |b, _| {
            b.iter(|| run(&module));
        });
    }
    group.finish();
}

fn bench_compile(c: &mut Criterion) {
    let progs = [
        Prog { name: "arith_loop",    src: ARITH_LOOP    },
        Prog { name: "fib_rec",       src: FIB_REC       },
        Prog { name: "static_access", src: STATIC_ACCESS  },
        Prog { name: "records",       src: RECORDS        },
    ];

    let mut group = c.benchmark_group("compile");
    for p in &progs {
        group.bench_with_input(BenchmarkId::from_parameter(p.name), p.src, |b, src| {
            b.iter(|| compile(src));
        });
    }
    group.finish();
}

fn bench_e2e(c: &mut Criterion) {
    let progs = [
        Prog { name: "arith_loop",    src: ARITH_LOOP    },
        Prog { name: "static_access", src: STATIC_ACCESS  },
        Prog { name: "records",       src: RECORDS        },
    ];

    let mut group = c.benchmark_group("e2e");
    for p in &progs {
        group.bench_with_input(BenchmarkId::from_parameter(p.name), p.src, |b, src| {
            b.iter(|| {
                let m = compile(src);
                run(&m)
            });
        });
    }
    group.finish();
}

// @cart frame pump: 1 000 frames of run_to_yield + resume.
// Compare against arith_loop (same arithmetic, no frame boundary) to isolate
// the cost of each yield/resume round-trip.
const CART_FRAME_PUMP: &str = r#"
@cart fn main() -> <frame> Unit {
  let mut acc = 0;
  let mut i = 0;
  while i < 1000 {
    acc = acc + i * 2;
    i = i + 1;
    frame.present()
  };
  halt(acc)
}
"#;

fn bench_cart(c: &mut Criterion) {
    let module = compile(CART_FRAME_PUMP);
    c.bench_function("cart_1000_frames", |b| {
        b.iter(|| {
            let mut vm = VirtualMachine::new();
            Host::default().install_into(&mut vm);
            vm.run_to_yield(&module).unwrap();
            while vm.resume(&module, Value::from_int(0)).unwrap() {}
        });
    });
}

// Closure capture + HOF: move closure passed to a higher-order function,
// called 50 000 times. Baseline for closure dispatch overhead.
const CLOSURE_HOF: &str = r#"
fn apply_n(f: (Int) -> Int, n: Int) -> Int {
  let mut acc = 0;
  let mut i = 0;
  while i < n { acc = acc + f(i); i = i + 1 };
  acc
}
fn main() -> Int {
  let base = 7;
  apply_n(move |x: Int| x * 2 + base, 50000)
}
"#;

fn bench_closures(c: &mut Criterion) {
    let module = compile(CLOSURE_HOF);
    let steps = steps_of(&module);
    let mut group = c.benchmark_group("vm_run");
    group.throughput(Throughput::Elements(steps));
    group.bench_function("closure_hof", |b| {
        b.iter(|| run(&module));
    });
    group.finish();
}

criterion_group!(benches, bench_vm, bench_compile, bench_e2e, bench_cart, bench_closures);
criterion_main!(benches);
