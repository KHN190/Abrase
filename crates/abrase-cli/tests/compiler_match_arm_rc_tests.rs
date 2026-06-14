#[path = "compiler_codegen_common.rs"]
mod compiler_codegen_common;

use abrase::bytecode::OpCode;
use abrase::compiler::Compiler;
use abrase::lexer::Lexer;
use abrase::parser::Parser;
use compiler_codegen_common::*;
use myriad::Value;

fn compile_fn_ops(src: &str, fn_name: &str) -> Vec<OpCode> {
    let mut parser = Parser::new(Lexer::new(src)).with_source(src.to_string());
    let ast = parser.parse_program();
    assert!(parser.errors.is_empty(), "{}", parser.pretty_print_errors());
    let mut c = Compiler::new().with_source(src.to_string());
    let module = c
        .compile_module(&ast)
        .unwrap_or_else(|_| panic!("\n{}", c.pretty_print_errors()));
    let idx = c
        .fn_names()
        .iter()
        .position(|n| n == fn_name)
        .unwrap_or_else(|| panic!("fn {} not found", fn_name));
    match &module.functions[idx] {
        abrase::bytecode::Chunk::Bytecode(bc) => bc.code.clone(),
        _ => panic!("{} is not bytecode", fn_name),
    }
}

fn drop_skipped_by_exit_jmp(ops: &[OpCode]) -> Option<usize> {
    ops.windows(2).position(|w| {
        matches!(w[0], OpCode::Jmp(n) if n > 0) && matches!(w[1], OpCode::Drop(_))
    })
}

fn assert_no_skipped_drop(src: &str, want: i64) {
    let ops = compile_fn_ops(src, "pick");
    assert!(
        drop_skipped_by_exit_jmp(&ops).is_none(),
        "arm cleanup Drop placed after its exit Jmp: {:?}",
        ops
    );
    assert_eq!(run_source(src), Ok(Value::from_int(want)), "wrong value");
}

#[test]
fn match_literal_arm_cleanup_precedes_exit_jmp() {
    assert_no_skipped_drop(
        r#"
static A: Array<Int> = [10, 20, 30, 40]
fn pick(k: Int) -> Int { match k { 0 => 0, 1 => A[1] + A[2], _ => 99 } }
fn main() -> Int { pick(1) }
"#,
        50,
    );
}

#[test]
fn match_range_arm_cleanup_precedes_exit_jmp() {
    assert_no_skipped_drop(
        r#"
static A: Array<Int> = [10, 20, 30, 40]
fn pick(k: Int) -> Int { match k { 0..=5 => A[0] + A[3], _ => 0 } }
fn main() -> Int { pick(2) }
"#,
        50,
    );
}

#[test]
fn match_guarded_arm_cleanup_precedes_exit_jmp() {
    assert_no_skipped_drop(
        r#"
static A: Array<Int> = [10, 20, 30, 40]
fn pick(k: Int) -> Int { match k { n if n > 0 => A[0] + A[1], _ => 0 } }
fn main() -> Int { pick(7) }
"#,
        30,
    );
}

#[test]
fn match_variant_tag_arm_cleanup_precedes_exit_jmp() {
    assert_no_skipped_drop(
        r#"
static A: Array<Int> = [10, 20, 30, 40]
type Cell = Empty | Queen(Int)
fn pick(c: Cell) -> Int { match c { Empty => A[0] + A[2], Queen(_) => 0 } }
fn main() -> Int { pick(Empty) }
"#,
        40,
    );
}

#[test]
fn match_variant_pattern_arm_cleanup_precedes_exit_jmp() {
    assert_no_skipped_drop(
        r#"
static A: Array<Int> = [10, 20, 30, 40]
type Cell = Empty | Queen(Int)
fn pick(c: Cell) -> Int { match c { Queen(r) => A[1] + r, Empty => 0 } }
fn main() -> Int { pick(Queen(5)) }
"#,
        25,
    );
}
