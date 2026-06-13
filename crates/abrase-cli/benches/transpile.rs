use abrase::compiler::Compiler;
use abrase::lexer::Lexer;
use abrase::parser::Parser;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use polka::Module;

fn compile(src: &str) -> Module {
    let mut parser = Parser::new(Lexer::new(src)).with_source(src.to_string());
    let ast = parser.parse_program();
    assert!(parser.errors.is_empty(), "{}", parser.pretty_print_errors());
    let mut compiler = Compiler::new().with_source(src.to_string());
    compiler.compile_module(&ast)
        .unwrap_or_else(|_| panic!("{}", compiler.pretty_print_errors()))
}

const RECURSION: &str = r#"
fn fib(n: Int) -> Int { if n < 2 { n } else { fib(n - 1) + fib(n - 2) } }
fn main() -> Int { fib(20) }
"#;

const RECORD: &str = r#"
type Pt = { x: Int, y: Int }
fn dist2(p: &Pt) -> Int { (*p).x * (*p).x + (*p).y * (*p).y }
fn main() -> Int { let p = Pt { x: 3, y: 4 }; dist2(&p) }
"#;

const ARITH_LOOP: &str = r#"
fn main() -> Int {
    let mut acc = 0;
    let mut i = 0;
    loop {
        if i == 1000 { break acc };
        acc = acc + i * 2 - 1;
        i = i + 1
    }
}
"#;

fn bench_transpile(c: &mut Criterion) {
    let cases = [("recursion", RECURSION), ("record", RECORD), ("arith_loop", ARITH_LOOP)];
    let mut group = c.benchmark_group("transpile_module");
    for (name, src) in cases {
        let module = compile(src);
        group.bench_with_input(BenchmarkId::from_parameter(name), &module, |b, m| {
            b.iter(|| polka_rustc::transpile_module(m).expect("transpile"));
        });
    }
    group.finish();
}

criterion_group!(benches, bench_transpile);
criterion_main!(benches);
