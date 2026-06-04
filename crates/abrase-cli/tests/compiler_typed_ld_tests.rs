// Typed-Ld drop elaboration: when typeck proves a loaded field/element scalar,
// the Ld dest is not marked handle, so no cleanup Drop is emitted for it.
// Synthetic paths (closure env loads) stay pessimistic.

use abrase::bytecode::OpCode;
use abrase::compiler::Compiler;
use abrase::lexer::Lexer;
use abrase::parser::Parser;
use myriad::VirtualMachine;

fn compile_fn_ops(src: &str, fn_name: &str, typed_ld: bool) -> Vec<OpCode> {
    let mut parser = Parser::new(Lexer::new(src)).with_source(src.to_string());
    let ast = parser.parse_program();
    assert!(parser.errors.is_empty(), "{}", parser.pretty_print_errors());
    let mut c = Compiler::new().with_source(src.to_string()).with_typed_ld(typed_ld);
    let module = c.compile_module(&ast)
        .unwrap_or_else(|_| panic!("\n{}", c.pretty_print_errors()));
    let idx = c.fn_names().iter().position(|n| n == fn_name)
        .unwrap_or_else(|| panic!("fn {} not found", fn_name));
    match &module.functions[idx] {
        abrase::bytecode::Chunk::Bytecode(bc) => bc.code.clone(),
        _ => panic!("{} is not bytecode", fn_name),
    }
}

fn drops(ops: &[OpCode]) -> usize {
    ops.iter().filter(|op| matches!(op, OpCode::Drop(_))).count()
}

fn run_value(src: &str, typed_ld: bool) -> (i64, usize) {
    let mut parser = Parser::new(Lexer::new(src)).with_source(src.to_string());
    let ast = parser.parse_program();
    assert!(parser.errors.is_empty(), "{}", parser.pretty_print_errors());
    let mut c = Compiler::new().with_source(src.to_string()).with_typed_ld(typed_ld);
    let module = c.compile_module(&ast)
        .unwrap_or_else(|_| panic!("\n{}", c.pretty_print_errors()));
    let mut vm = VirtualMachine::new();
    let v = vm.run_module(&module).expect("run failed");
    (v.as_int(), vm.heap_live_count())
}

fn assert_equiv_and_fewer_drops(src: &str, fn_name: &str) {
    let off = compile_fn_ops(src, fn_name, false);
    let on = compile_fn_ops(src, fn_name, true);
    assert!(drops(&on) < drops(&off),
        "{}: expected fewer Drops (off={}, on={})", fn_name, drops(&off), drops(&on));
    assert_eq!(run_value(src, false), run_value(src, true), "{}: value/heap diverged", fn_name);
}

const VARIANT_TAG: &str = r#"
type Cell = Empty | Queen(Int)
fn tag_of(c: Cell) -> Int {
  match c { Empty => 0, Queen(r) => r }
}
fn main() -> Int { tag_of(Queen(3)) + tag_of(Empty) }
"#;

#[test]
fn variant_tag_read_emits_no_drop() {
    assert_equiv_and_fewer_drops(VARIANT_TAG, "tag_of");
}

const INT_FIELD: &str = r#"
type P = { x: Int, y: Int }
fn sum(p: P) -> Int { match p.x { 0 => p.y, _ => p.x + p.y } }
fn main() -> Int { sum(P { x: 3, y: 4 }) }
"#;

#[test]
fn scalar_record_field_read_emits_no_drop() {
    assert_equiv_and_fewer_drops(INT_FIELD, "sum");
}

const INT_ELEM: &str = r#"
fn pick(xs: Array<Int>, i: Int) -> Int { match xs[i] { 0 => 1, _ => 2 } }
fn main() -> Int { let a = [10, 20, 30]; pick(a, 1) }
"#;

#[test]
fn scalar_array_element_read_is_equivalent_and_adds_no_drop() {
    // Scalar LdIdx temps carry no Drop today; the typed clearing serves
    // copy-prop untainting. Pin: no regression, identical behavior.
    let off = compile_fn_ops(INT_ELEM, "pick", false);
    let on = compile_fn_ops(INT_ELEM, "pick", true);
    assert!(drops(&on) <= drops(&off));
    assert_eq!(run_value(INT_ELEM, false), run_value(INT_ELEM, true));
}

const HANDLE_FIELD: &str = r#"
type Row = { cells: Array<Int>, n: Int }
fn first(r: Row) -> Int { let c = r.cells; c[0] }
fn main() -> Int { first(Row { cells: [7, 8], n: 2 }) }
"#;

#[test]
fn handle_field_read_keeps_its_drop() {
    // r.cells is an Array handle — its cleanup Drop must survive typed-ld.
    let off = compile_fn_ops(HANDLE_FIELD, "first", false);
    let on = compile_fn_ops(HANDLE_FIELD, "first", true);
    assert!(drops(&on) > 0, "handle field cleanup vanished");
    assert_eq!(run_value(HANDLE_FIELD, false), run_value(HANDLE_FIELD, true));
    let _ = off;
}

const CLOSURE_ENV: &str = r#"
fn apply(f: (Int) -> Int, x: Int) -> Int { f(x) }
fn main() -> Int {
  let base = 5;
  apply(move |v: Int| v + base, 2)
}
"#;

#[test]
fn closure_env_loads_stay_pessimistic_and_equivalent() {
    // Synthetic env types are untrusted: behavior + heap must be identical.
    assert_eq!(run_value(CLOSURE_ENV, false), run_value(CLOSURE_ENV, true));
}

const NESTED_MATCH_LOOP: &str = r#"
type Cell = Empty | Queen(Int)
fn count(cells: Array<Cell>, n: Int) -> Int {
  let mut acc = 0;
  let mut i = 0;
  while i < n {
    acc = acc + match cells[i] { Empty => 0, Queen(_) => 1 };
    i = i + 1
  };
  acc
}
fn main() -> Int { count([Queen(0), Empty, Queen(2)], 3) }
"#;

#[test]
fn variant_scan_loop_is_equivalent_with_fewer_drops() {
    assert_equiv_and_fewer_drops(NESTED_MATCH_LOOP, "count");
}
