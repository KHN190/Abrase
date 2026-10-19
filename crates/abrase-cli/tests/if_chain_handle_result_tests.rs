// An if/else-if chain is a nested if in the else arm. Its value is defined on
// several paths, so the branch result must reach the outer result register on
// every path. Handle-valued arms (variants, arrays, records, strings) are the
// sensitive case: a missed path yields a plain word where a handle is expected.

use abrase::compiler::Compiler;
use abrase::lexer::Lexer;
use abrase::parser::Parser;
use myriad::VirtualMachine;

fn run(src: &str, inline: bool) -> Result<(i64, usize), String> {
    let mut p = Parser::new(Lexer::new(src)).with_source(src.to_string());
    let ast = p.parse_program();
    if !p.errors.is_empty() { return Err(format!("parse:\n{}", p.pretty_print_errors())); }
    let mut c = Compiler::new().with_source(src.to_string()).with_inline(inline);
    let module = c.compile_module(&ast).map_err(|_| c.pretty_print_errors())?;
    let mut vm = VirtualMachine::new().with_step_cap(1_000_000);
    let v = vm.run_module(&module)?;
    Ok((v.as_int(), vm.heap_live_count()))
}

fn assert_clean(src: &str, expected: i64) {
    for inline in [false, true] {
        let (v, live) = run(src, inline).unwrap_or_else(|e| panic!("inline={}: {}\n{}", inline, e, src));
        assert_eq!(v, expected, "inline={} value\n{}", inline, src);
        assert_eq!(live, 0, "inline={} leaked {} slots\n{}", inline, live, src);
    }
}

const FX: &str = "type Fx = | Crush | Echo | Wash\ntype W = { fx: Fx }\n\
fn idx(f: Fx) -> Int { match f { Crush => 0, Echo => 1, Wash => 2 } }\n\
fn chain(b: Int) -> Fx { if b == 0 { Crush } else if b == 1 { Echo } else { Wash } }\n\
fn store(w: &mut W, f: Fx) -> Unit { w.fx = f }\n";

#[test]
fn three_way_chain_middle_variant_passed_to_fn() {
    assert_clean(&format!("{FX}fn main() -> Int {{ idx(chain(1)) }}"), 1);
}

#[test]
fn three_way_chain_first_variant_passed_to_fn() {
    assert_clean(&format!("{FX}fn main() -> Int {{ idx(chain(0)) }}"), 0);
}

#[test]
fn three_way_chain_last_variant_passed_to_fn() {
    assert_clean(&format!("{FX}fn main() -> Int {{ idx(chain(2)) }}"), 2);
}

#[test]
fn three_way_chain_middle_variant_via_let() {
    assert_clean(&format!("{FX}fn main() -> Int {{ let f = chain(1); idx(f) }}"), 1);
}

#[test]
fn inline_three_way_chain_middle_variant() {
    assert_clean(&format!("{FX}fn main() -> Int {{ let b = 1; let f = if b == 0 {{ Crush }} else if b == 1 {{ Echo }} else {{ Wash }}; idx(f) }}"), 1);
}

#[test]
fn inline_three_way_chain_middle_variant_stored_through_mut_ref() {
    assert_clean(&format!("{FX}fn main() -> Int {{ let b = 1; let mut w = W {{ fx: Crush }}; let f = if b == 0 {{ Crush }} else if b == 1 {{ Echo }} else {{ Wash }}; store(&mut w, f); idx(w.fx) }}"), 1);
}

#[test]
fn inline_three_way_chain_last_variant_stored_through_mut_ref() {
    assert_clean(&format!("{FX}fn main() -> Int {{ let b = 2; let mut w = W {{ fx: Crush }}; let f = if b == 0 {{ Crush }} else if b == 1 {{ Echo }} else {{ Wash }}; store(&mut w, f); idx(w.fx) }}"), 2);
}

#[test]
fn two_way_variant_stored_through_mut_ref() {
    assert_clean(&format!("{FX}fn main() -> Int {{ let b = 1; let mut w = W {{ fx: Crush }}; let f = if b == 0 {{ Crush }} else {{ Echo }}; store(&mut w, f); idx(w.fx) }}"), 1);
}

#[test]
fn fn_chain_result_stored_through_mut_ref() {
    assert_clean(&format!("{FX}fn main() -> Int {{ let mut w = W {{ fx: Crush }}; store(&mut w, chain(1)); idx(w.fx) }}"), 1);
}

#[test]
fn chain_middle_variant_matched_directly() {
    assert_clean("type Fx = | Crush | Echo | Wash\nfn main() -> Int { let b = 1; let f = if b == 0 { Crush } else if b == 1 { Echo } else { Wash }; match f { Echo => 1, _ => 0 } }", 1);
}

#[test]
fn explicit_nested_if_in_else_block_middle_variant() {
    assert_clean("type Fx = | Crush | Echo | Wash\nfn main() -> Int { let b = 1; let f = if b == 0 { Crush } else { if b == 1 { Echo } else { Wash } }; match f { Echo => 1, _ => 0 } }", 1);
}

#[test]
fn nested_if_in_then_arm_middle_variant() {
    assert_clean("type Fx = | Crush | Echo | Wash\nfn main() -> Int { let b = 1; let f = if b < 2 { if b == 0 { Crush } else { Echo } } else { Wash }; match f { Echo => 1, _ => 0 } }", 1);
}

#[test]
fn chain_middle_array() {
    assert_clean("fn main() -> Int { let b = 1; let f = if b == 0 { [0] } else if b == 1 { [1] } else { [2] }; f[0] }", 1);
}

#[test]
fn chain_middle_record() {
    assert_clean("type R = { v: Int }\nfn main() -> Int { let b = 1; let f = if b == 0 { R { v: 0 } } else if b == 1 { R { v: 1 } } else { R { v: 2 } }; f.v }", 1);
}

#[test]
fn chain_middle_string() {
    assert_clean("fn main() -> Int { let b = 1; let f = if b == 0 { \"a\" } else if b == 1 { \"bb\" } else { \"ccc\" }; f.len() }", 2);
}

#[test]
fn chain_middle_payload_variant() {
    assert_clean("type P = | A(Int) | B(Int)\nfn main() -> Int { let b = 1; let f = if b == 0 { A(0) } else if b == 1 { B(7) } else { A(2) }; match f { B(n) => n, _ => 0 } }", 7);
}

#[test]
fn four_way_chain_each_branch_array() {
    for (b, want) in [(0, 10), (1, 11), (2, 12), (3, 13)] {
        assert_clean(&format!("fn main() -> Int {{ let b = {b}; let f = if b == 0 {{ [10] }} else if b == 1 {{ [11] }} else if b == 2 {{ [12] }} else {{ [13] }}; f[0] }}"), want);
    }
}

#[test]
fn chain_middle_variant_with_block_body() {
    assert_clean("type Fx = | Crush | Echo | Wash\nfn main() -> Int { let b = 1; let f = if b == 0 { Crush } else if b == 1 { let k = b + 1; Echo } else { Wash }; match f { Echo => 1, _ => 0 } }", 1);
}

#[test]
fn chain_inside_loop_middle_variant_each_iteration() {
    assert_clean("type Fx = | Crush | Echo | Wash\nfn main() -> Int { let mut n = 0; for i in 0..3 { let f = if i == 5 { Crush } else if i < 3 { Echo } else { Wash }; n = n + match f { Echo => 1, _ => 0 } }; n }", 3);
}

#[test]
fn chain_as_match_arm_body_middle_variant() {
    assert_clean("type Fx = | Crush | Echo | Wash\nfn main() -> Int { let b = 1; let f = match b { 1 => if b == 0 { Crush } else if b == 1 { Echo } else { Wash }, _ => Crush }; match f { Echo => 1, _ => 0 } }", 1);
}

#[test]
fn chain_scalar_middle_is_baseline() {
    assert_clean("fn main() -> Int { let b = 1; let f = if b == 0 { 0 } else if b == 1 { 1 } else { 2 }; f }", 1);
}

struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self { Self(seed.wrapping_mul(6364136223846793005).wrapping_add(1)) }
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0
    }
    fn pick(&mut self, n: u64) -> u64 { self.next() % n }
}

#[derive(Clone, Copy)]
enum Kind { Variant, Array, Record, Str }

fn leaf(kind: Kind, v: i64) -> String {
    match kind {
        Kind::Variant => format!("V({v})"),
        Kind::Array => format!("[{v}]"),
        Kind::Record => format!("R {{ v: {v} }}"),
        Kind::Str => format!("\"{}\"", "x".repeat(v as usize)),
    }
}

fn extract(kind: Kind) -> &'static str {
    match kind {
        Kind::Variant => "match f { V(n) => n }",
        Kind::Array => "f[0]",
        Kind::Record => "f.v",
        Kind::Str => "f.len()",
    }
}

fn gen_tree(rng: &mut Rng, kind: Kind, depth: u32, b: i64, next: &mut i64) -> (String, i64) {
    if depth == 0 || rng.pick(3) == 0 {
        let v = *next; *next += 1;
        return (leaf(kind, v), v);
    }
    let pivot = rng.pick(6) as i64;
    let (t, tv) = gen_tree(rng, kind, depth - 1, b, next);
    let (e, ev) = gen_tree(rng, kind, depth - 1, b, next);
    let taken = b < pivot;
    let chain = e.starts_with("if ") && rng.pick(3) != 0;
    let e = if chain { e } else { format!("{{ {} }}", e) };
    (format!("if b < {pivot} {{ {t} }} else {e}"), if taken { tv } else { ev })
}

#[test]
fn fuzz_if_trees_with_handle_leaves_match_model() {
    let mut n_fail = 0u32;
    let mut report = String::new();
    for seed in 0..300u64 {
        let mut rng = Rng::new(seed);
        let kind = [Kind::Variant, Kind::Array, Kind::Record, Kind::Str][rng.pick(4) as usize];
        let b = rng.pick(6) as i64;
        let mut next = 1i64;
        let (tree, expected) = gen_tree(&mut rng, kind, 4, b, &mut next);
        let src = format!("type V = | V(Int)\ntype R = {{ v: Int }}\nfn main() -> Int {{ let b = {b}; let f = {tree}; {} }}", extract(kind));
        for inline in [false, true] {
            match run(&src, inline) {
                Ok((v, 0)) if v == expected => {}
                other => {
                    n_fail += 1;
                    if report.len() < 5000 {
                        report.push_str(&format!("--- seed={seed} inline={inline} expected={expected} got={other:?} ---\n{src}\n"));
                    }
                }
            }
        }
    }
    assert_eq!(n_fail, 0, "{} failing run(s):\n{}", n_fail, report);
}
