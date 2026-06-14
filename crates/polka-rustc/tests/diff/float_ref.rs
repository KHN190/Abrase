// Float-semantics oracle: abrase eval vs IEEE f64 reference.
//
// The interp-vs-AOT differential cannot catch float type-inference bugs: both
// sides run the same (mis)compiled bytecode and agree. This oracle compiles
// abrase float source, runs it in the interpreter, and checks the result
// against the same expression evaluated in Rust f64. It caught the codegen bug
// where `x*x + y*y > 4.0` lowers to integer `Add`/`Gt` on float bit patterns
// (float-ness not propagated through arithmetic-result registers).
use abrase::{compiler::Compiler, lexer::Lexer, parser::Parser};
use myriad::VirtualMachine;

fn run_int(src: &str) -> i64 {
    let mut p = Parser::new(Lexer::new(src)).with_source(src.to_string());
    let decls = p.parse_program();
    assert!(p.errors.is_empty(), "parse errors: {:?}\nsrc={}", p.errors, src);
    let module = Compiler::new().compile_module(&decls).unwrap_or_else(|errs| {
        panic!("compile errors: {}\nsrc={}", errs.iter()
            .map(|e| format!("{:?}: {}", e.code, e.message)).collect::<Vec<_>>().join("\n"), src)
    });
    let mut vm = VirtualMachine::new().with_step_cap(1_000_000);
    myriad::Host::default().install_into(&mut vm);
    vm.run_module(&module).expect("run").raw() as i64
}

fn lit(v: f64) -> String {
    if v < 0.0 { format!("(0.0 - {:?})", -v) } else { format!("{:?}", v) }
}

// Leaves must be float variables, not literals: literal operands get correct
// types via constant context; variable/param results lose float-ness (the bug).
#[test]
fn sum_of_products_compare() {
    let src = "fn main() -> Int { let a = 2.0; let b = 1.0; if a * a + b * b > 4.0 { 1 } else { 0 } }";
    assert_eq!(run_int(src), 1, "5.0 > 4.0 should be 1");
}

#[test]
fn negative_float_compare() {
    let src = "fn main() -> Int { let a = 0.0 - 1.0; if a * a - 5.0 > 0.0 { 1 } else { 0 } }";
    assert_eq!(run_int(src), 0, "1.0 - 5.0 = -4.0 > 0.0 should be 0");
}

#[test]
fn product_difference_compare() {
    let src = "fn main() -> Int { let a = 3.0; let b = 2.0; if a * a - b * b > 4.0 { 1 } else { 0 } }";
    assert_eq!(run_int(src), 1, "9 - 4 = 5 > 4 should be 1");
}

// Fuzzer: random nested float expression (+,-,*) over literals, compared to a
// threshold; oracle is the identical expression evaluated in Rust f64.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 { self.0 ^= self.0 << 13; self.0 ^= self.0 >> 7; self.0 ^= self.0 << 17; self.0 }
    fn below(&mut self, n: usize) -> usize { (self.next() % n as u64) as usize }
    fn val(&mut self) -> f64 { (self.below(21) as f64) - 10.0 }
}

fn gen_expr(rng: &mut Rng, depth: usize, vars: &[(String, f64)]) -> (String, f64) {
    if depth == 0 || rng.below(3) == 0 {
        let (n, v) = &vars[rng.below(vars.len())];
        return (n.clone(), *v);
    }
    let (ls, lv) = gen_expr(rng, depth - 1, vars);
    let (rs, rv) = gen_expr(rng, depth - 1, vars);
    let (op, val) = match rng.below(3) {
        0 => ("+", lv + rv),
        1 => ("-", lv - rv),
        _ => ("*", lv * rv),
    };
    (format!("({} {} {})", ls, op, rs), val)
}

#[test]
fn fuzz_float_expr_matches_f64() {
    let mut rng = Rng(0x9E3779B97F4A7C15);
    for _ in 0..200 {
        let vars: Vec<(String, f64)> = (0..3).map(|i| (format!("v{}", i), rng.val())).collect();
        let binds: String = vars.iter().map(|(n, v)| format!("let {} = {}; ", n, lit(*v))).collect();
        let (expr, expected) = gen_expr(&mut rng, 4, &vars);
        let thresh = (rng.below(21) as f64) - 10.0;
        let want = if expected > thresh { 1 } else { 0 };
        let src = format!("fn main() -> Int {{ {}if {} > {} {{ 1 }} else {{ 0 }} }}", binds, expr, lit(thresh));
        let got = run_int(&src);
        assert_eq!(got, want, "src={}\nexpected_f64={} thresh={} -> want {} got {}",
            src, expected, thresh, want, got);
    }
}
