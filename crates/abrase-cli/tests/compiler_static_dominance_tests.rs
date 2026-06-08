#[path = "compiler_codegen_common.rs"]
mod compiler_codegen_common;

use abrase::bytecode::OpCode;
use abrase::compiler::Compiler;
use abrase::lexer::Lexer;
use abrase::parser::Parser;
use compiler_codegen_common::*;
use myriad::Value;

// The module static-table base (Dei) must dominate every static use and be
// rc-balanced by exactly one Drop per call. Bug: the Dei was emitted lazily at
// the first static use, which could sit inside a non-dominating branch (&&/||
// short-circuit) -> skipped on one path -> later use reads HANDLE_NONE (crash);
// and its rc_inc was never dropped -> module-table refcount leaks every call.

fn fn_ops(src: &str, name: &str) -> Vec<OpCode> {
    let mut parser = Parser::new(Lexer::new(src)).with_source(src.to_string());
    let ast = parser.parse_program();
    assert!(parser.errors.is_empty(), "{}", parser.pretty_print_errors());
    let mut c = Compiler::new().with_source(src.to_string());
    let module = c
        .compile_module(&ast)
        .unwrap_or_else(|_| panic!("\n{}", c.pretty_print_errors()));
    let idx = c.fn_names().iter().position(|n| n == name).unwrap();
    match &module.functions[idx] {
        abrase::bytecode::Chunk::Bytecode(bc) => bc.code.clone(),
        _ => panic!("{} not bytecode", name),
    }
}

// ── A: crash — short-circuit skip path then static use ──────────────────────

#[test]
fn or_short_circuit_skip_then_static_use_runs() {
    // f(5): k>0 true -> `||` skips A[0]; A[1] must still read a defined table.
    let src = r#"
static A: Array<Int> = [10, 20, 30, 40]
fn f(k: Int) -> Int { let _g = k > 0 || A[0] > 0; A[1] }
fn main() -> Int { f(5) }
"#;
    assert_eq!(run_source(src), Ok(Value::from_int(20)));
}

#[test]
fn and_short_circuit_skip_then_static_use_runs() {
    // f(5): k>100 false -> `&&` skips A[0]; A[1] must still read a defined table.
    let src = r#"
static A: Array<Int> = [10, 20, 30, 40]
fn f(k: Int) -> Int { let _g = k > 100 && A[0] > 0; A[1] }
fn main() -> Int { f(5) }
"#;
    assert_eq!(run_source(src), Ok(Value::from_int(20)));
}

#[test]
fn nested_short_circuit_skip_then_static_use_runs() {
    let src = r#"
static A: Array<Int> = [10, 20, 30, 40]
fn f(k: Int) -> Int { let _g = k > 0 || k < 0 || A[0] > 0; A[2] }
fn main() -> Int { f(5) }
"#;
    assert_eq!(run_source(src), Ok(Value::from_int(30)));
}

// ── A: structural — the table Dei dominates the first conditional branch ─────

#[test]
fn table_dei_dominates_first_branch() {
    let src = r#"
static A: Array<Int> = [10, 20, 30, 40]
fn f(k: Int) -> Int { let _g = k > 0 || A[0] > 0; A[1] }
fn main() -> Int { f(0) }
"#;
    let ops = fn_ops(src, "f");
    let first_dei = ops.iter().position(|o| matches!(o, OpCode::Dei(_, _)));
    let first_branch = ops
        .iter()
        .position(|o| matches!(o, OpCode::Jz(_, _) | OpCode::Jnz(_, _)));
    let dei = first_dei.expect("f uses a static, must load the module table");
    if let Some(br) = first_branch {
        assert!(
            dei < br,
            "table Dei at {} does not dominate first branch at {}: {:?}",
            dei, br, ops
        );
    }
}

// ── B: leak — module-table rc balanced regardless of call count ──────────────

fn one(name: &str) -> String {
    format!("  {}(0);\n", name)
}

#[test]
fn module_table_rc_balanced_across_calls_plain() {
    let helper = "fn us(k: Int) -> Int { A[0] + A[1] }\n";
    let mk = |n: usize| {
        format!(
            "static A: Array<Int> = [1, 2, 3, 4]\n{}fn main() -> Int {{\n{}  0\n}}\n",
            helper,
            one("us").repeat(n)
        )
    };
    let rc1 = run_source_table_rc(&mk(1)).expect("run 1x");
    let rc7 = run_source_table_rc(&mk(7)).expect("run 7x");
    assert_eq!(rc1, rc7, "module-table rc leaks with call count: 1x={:?} 7x={:?}", rc1, rc7);
}

#[test]
fn module_table_rc_balanced_across_calls_short_circuit() {
    // k < 0 path evaluates A[0] (Dei runs) but never crashes; pre-fix it leaks.
    let helper = "fn us(k: Int) -> Int { let _g = k > 0 || A[0] > 0; A[1] }\n";
    let mk = |n: usize| {
        format!(
            "static A: Array<Int> = [1, 2, 3, 4]\n{}fn main() -> Int {{\n{}  0\n}}\n",
            helper,
            "  us(-1);\n".repeat(n)
        )
    };
    let rc1 = run_source_table_rc(&mk(1)).expect("run 1x");
    let rc7 = run_source_table_rc(&mk(7)).expect("run 7x");
    assert_eq!(rc1, rc7, "module-table rc leaks via short-circuit path: 1x={:?} 7x={:?}", rc1, rc7);
}
