use abrase::bytecode::OpCode;
use abrase::compiler::Compiler;
use abrase::lexer::Lexer;
use abrase::loader::load_program;
use abrase::parser::Parser;
use myriad::{Value, VirtualMachine};
use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_CTR: AtomicU64 = AtomicU64::new(0);

fn compile_entry_ops(src: &str) -> Vec<OpCode> {
    let mut parser = Parser::new(Lexer::new(src)).with_source(src.to_string());
    let ast = parser.parse_program();
    assert!(parser.errors.is_empty(), "{}", parser.pretty_print_errors());
    let mut compiler = Compiler::new().with_source(src.to_string());
    let module = compiler
        .compile_module(&ast)
        .unwrap_or_else(|_| panic!("\n{}", compiler.pretty_print_errors()));
    match &module.functions[module.entry] {
        abrase::bytecode::Chunk::Bytecode(bc) => bc.code.clone(),
        _ => panic!("entry is not a bytecode chunk"),
    }
}

fn run_src(src: &str) -> Result<Value, String> {
    let mut parser = Parser::new(Lexer::new(src)).with_source(src.to_string());
    let ast = parser.parse_program();
    if !parser.errors.is_empty() {
        return Err(format!("Parser errors:\n{}", parser.pretty_print_errors()));
    }
    let mut compiler = Compiler::new().with_source(src.to_string());
    let module = compiler
        .compile_module(&ast)
        .map_err(|_| compiler.pretty_print_errors())?;
    let mut vm = VirtualMachine::new();
    vm.run_module(&module).map_err(|e| format!("VM error: {}", e))
}

fn run_src_with_heap(src: &str) -> Result<(Value, usize), String> {
    let mut parser = Parser::new(Lexer::new(src)).with_source(src.to_string());
    let ast = parser.parse_program();
    if !parser.errors.is_empty() {
        return Err(format!("Parser errors:\n{}", parser.pretty_print_errors()));
    }
    let mut compiler = Compiler::new().with_source(src.to_string());
    let module = compiler
        .compile_module(&ast)
        .map_err(|_| compiler.pretty_print_errors())?;
    let mut vm = VirtualMachine::new();
    let v = vm.run_module(&module).map_err(|e| format!("VM error: {}", e))?;
    Ok((v, vm.heap_live_count()))
}

fn run_files(entry: &std::path::Path) -> Result<(Value, usize), String> {
    let loaded = load_program(entry).map_err(|e| format!("Load error: {:?}", e))?;
    let mut compiler = Compiler::new().with_source(loaded.entry_source.clone());
    let module = compiler
        .compile_module(&loaded.decls)
        .map_err(|_| loaded.render_errors(&compiler.errors))?;
    let mut vm = VirtualMachine::new();
    let v = vm.run_module(&module).map_err(|e| format!("VM error: {}", e))?;
    Ok((v, vm.heap_live_count()))
}

fn with_temp_dir(f: impl FnOnce(&std::path::Path) -> Result<(Value, usize), String>) -> (Value, usize) {
    let n = TEMP_CTR.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir()
        .join(format!("abrase_test_{}_{}", std::process::id(), n));
    fs::create_dir_all(&dir).expect("create temp dir");
    let result = f(&dir);
    fs::remove_dir_all(&dir).ok();
    result.unwrap_or_else(|e| panic!("\n{}", e))
}

// ── single-module: for loop + static (primitive) ─────────────────────────────

const FOR_READS_STATIC: &str = r#"
static S: Int = 7
fn main() -> Int {
  let mut acc = 0;
  for i in 0..4 { acc = acc + S };
  acc
}
"#;

#[test]
fn for_loop_reads_static_correctly() {
    let v = run_src(FOR_READS_STATIC).unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(28));
}

#[test]
fn for_loop_static_module_table_hoisted() {
    let ops = compile_entry_ops(FOR_READS_STATIC);
    let dei = ops.iter().filter(|o| matches!(o, OpCode::Dei(..))).count();
    assert_eq!(dei, 1, "module-table load must be hoisted before the for loop: {ops:?}");
}

const FOR_WRITES_STATIC: &str = r#"
static mut A: Int = 0
fn main() -> Int {
  for _ in 0..3 { A = A + 1 };
  A
}
"#;

#[test]
fn for_loop_writes_static_mut_correctly() {
    let v = run_src(FOR_WRITES_STATIC).unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(3));
}

const FOR_MULTI_STATIC: &str = r#"
static mut A: Int = 1
static mut B: Int = 2
fn main() -> Int {
  let mut acc = 0;
  for _ in 0..4 { acc = acc + A + B };
  acc
}
"#;

#[test]
fn for_loop_reads_multiple_statics_correctly() {
    let v = run_src(FOR_MULTI_STATIC).unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(12));
}

#[test]
fn for_loop_multi_static_single_module_table_load() {
    let ops = compile_entry_ops(FOR_MULTI_STATIC);
    let dei = ops.iter().filter(|o| matches!(o, OpCode::Dei(..))).count();
    assert_eq!(dei, 1, "both statics must share one hoisted Dei: {ops:?}");
}

const NESTED_FOR_STATIC: &str = r#"
static S: Int = 2
fn main() -> Int {
  let mut acc = 0;
  for i in 0..3 {
    for j in 0..3 { acc = acc + S }
  };
  acc
}
"#;

#[test]
fn nested_for_loops_read_static_correctly() {
    let v = run_src(NESTED_FOR_STATIC).unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(18));
}

const FOR_BREAK_STATIC: &str = r#"
static S: Int = 10
fn main() -> Int {
  let mut acc = 0;
  for i in 0..10 {
    if i == 3 { break };
    acc = acc + S
  };
  acc
}
"#;

#[test]
fn for_loop_break_static_reads_correct() {
    let v = run_src(FOR_BREAK_STATIC).unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(30));
}

const FOR_CONTINUE_STATIC: &str = r#"
static S: Int = 5
fn main() -> Int {
  let mut acc = 0;
  for i in 0..6 {
    if i % 2 == 0 { continue };
    acc = acc + S
  };
  acc
}
"#;

#[test]
fn for_loop_continue_static_reads_correct() {
    let v = run_src(FOR_CONTINUE_STATIC).unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(15));
}

// handle statics: heap_live_count() subtracts the module table itself but not
// handles stored inside it — those stay live for the module's lifetime by design.
const FOR_HANDLE_STATIC: &str = r#"
static ARR: Array<Int> = [1, 2, 3]
fn main() -> Int {
  let mut acc = 0;
  for _ in 0..3 { acc = acc + ARR[0] + ARR[1] + ARR[2] };
  acc
}
"#;

#[test]
fn for_loop_handle_static_value_correct() {
    let v = run_src(FOR_HANDLE_STATIC).unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(18));
}

#[test]
fn for_loop_handle_static_no_extra_leak() {
    let (v, live) = run_src_with_heap(FOR_HANDLE_STATIC).unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(18));
    // 1 live = ARR array cell (static; intentional). No loop-iteration leaks.
    assert_eq!(live, 1, "only the static ARR cell must remain live: {live}");
}

// ── single-module: for loop + record static ───────────────────────────────────

const RECORD_STATIC_FOR: &str = r#"
type Point = { x: Int, y: Int }
static ORIGIN: Point = Point { x: 3, y: 4 }
fn main() -> Int {
  let mut acc = 0;
  for _ in 0..3 { acc = acc + ORIGIN.x + ORIGIN.y };
  acc
}
"#;

#[test]
fn for_loop_reads_record_static_correctly() {
    let v = run_src(RECORD_STATIC_FOR).unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(21));
}

#[test]
fn for_loop_record_static_no_extra_leak() {
    let (v, live) = run_src_with_heap(RECORD_STATIC_FOR).unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(21));
    // 1 live = ORIGIN record cell (static; intentional).
    assert_eq!(live, 1, "only the static ORIGIN cell must remain live: {live}");
}

const RECORD_STATIC_MUT_FOR: &str = r#"
type Counter = { n: Int }
static mut C: Counter = Counter { n: 0 }
fn main() -> Int {
  for _ in 0..5 { C.n = C.n + 1 };
  C.n
}
"#;

#[test]
fn for_loop_mutates_record_static_field() {
    let v = run_src(RECORD_STATIC_MUT_FOR).unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(5));
}

const RECORD_ARRAY_STATIC_FOR: &str = r#"
type Pair = { a: Int, b: Int }
static PAIRS: Array<Pair> = [Pair { a: 1, b: 2 }, Pair { a: 3, b: 4 }]
fn main() -> Int {
  let mut sum = 0;
  for i in 0..2 { sum = sum + PAIRS[i].a + PAIRS[i].b };
  sum
}
"#;

#[test]
fn for_loop_reads_record_array_static() {
    let v = run_src(RECORD_ARRAY_STATIC_FOR).unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(10));
}

#[test]
fn for_loop_record_array_static_no_extra_leak() {
    let (v, live) = run_src_with_heap(RECORD_ARRAY_STATIC_FOR).unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(10));
    // 3 live = PAIRS outer Array + 2 Pair records (all static; intentional).
    assert_eq!(live, 3, "only the 3 static cells (array + 2 pairs) must remain live: {live}");
}

// ── multi-module: cross-file static (primitive) ───────────────────────────────

#[test]
fn cross_module_static_read_in_for_loop() {
    let (v, _) = with_temp_dir(|dir| {
        fs::write(dir.join("lib.abe"), "pub static X: Int = 7\n").expect("write lib");
        fs::write(
            dir.join("main.abe"),
            "use lib::{ X }\nfn main() -> Int { let mut acc = 0; for _ in 0..4 { acc = acc + X }; acc }\n",
        ).expect("write main");
        run_files(&dir.join("main.abe"))
    });
    assert_eq!(v, Value::from_int(28));
}

#[test]
fn cross_module_static_init_order_correct() {
    let (v, _) = with_temp_dir(|dir| {
        fs::write(dir.join("lib.abe"), "pub static BASE: Int = 100\n").expect("write lib");
        fs::write(
            dir.join("main.abe"),
            "use lib::{ BASE }\nstatic DERIVED: Int = 5\nfn main() -> Int { BASE + DERIVED }\n",
        ).expect("write main");
        run_files(&dir.join("main.abe"))
    });
    assert_eq!(v, Value::from_int(105));
}

#[test]
fn cross_module_handle_static_no_extra_leak() {
    let (v, live) = with_temp_dir(|dir| {
        fs::write(dir.join("lib.abe"), "pub static ARR: Array<Int> = [10, 20, 30]\n").expect("write lib");
        fs::write(
            dir.join("main.abe"),
            "use lib::{ ARR }\nfn main() -> Int { ARR[0] + ARR[1] + ARR[2] }\n",
        ).expect("write main");
        run_files(&dir.join("main.abe"))
    });
    assert_eq!(v, Value::from_int(60));
    // 1 live = ARR array cell (static; intentional).
    assert_eq!(live, 1, "only static ARR must remain live: {live}");
}

#[test]
fn cross_module_fn_in_for_loop_reads_its_own_static() {
    let (v, _) = with_temp_dir(|dir| {
        fs::write(
            dir.join("lib.abe"),
            "pub static FACTOR: Int = 3\npub fn scaled(n: Int) -> Int { n * FACTOR }\n",
        ).expect("write lib");
        fs::write(
            dir.join("main.abe"),
            "use lib::{ scaled }\nfn main() -> Int { let mut acc = 0; for i in 1..4 { acc = acc + scaled(i) }; acc }\n",
        ).expect("write main");
        run_files(&dir.join("main.abe"))
    });
    assert_eq!(v, Value::from_int(18));
}

// ── multi-module: cross-file record static ────────────────────────────────────

#[test]
fn cross_module_record_static_read() {
    let (v, live) = with_temp_dir(|dir| {
        fs::write(
            dir.join("lib.abe"),
            "pub type Vec2 = { x: Int, y: Int }\npub static ORIGIN: Vec2 = Vec2 { x: 5, y: 8 }\n",
        ).expect("write lib");
        fs::write(
            dir.join("main.abe"),
            "use lib::{ ORIGIN }\nfn main() -> Int { ORIGIN.x + ORIGIN.y }\n",
        ).expect("write main");
        run_files(&dir.join("main.abe"))
    });
    assert_eq!(v, Value::from_int(13));
    // 1 live = ORIGIN record (static; intentional).
    assert_eq!(live, 1, "only static ORIGIN must remain live: {live}");
}

#[test]
fn cross_module_record_static_in_for_loop() {
    let (v, live) = with_temp_dir(|dir| {
        fs::write(
            dir.join("lib.abe"),
            "pub type Vec2 = { x: Int, y: Int }\npub static ORIGIN: Vec2 = Vec2 { x: 2, y: 3 }\n",
        ).expect("write lib");
        fs::write(
            dir.join("main.abe"),
            "use lib::{ ORIGIN }\nfn main() -> Int { let mut acc = 0; for _ in 0..4 { acc = acc + ORIGIN.x + ORIGIN.y }; acc }\n",
        ).expect("write main");
        run_files(&dir.join("main.abe"))
    });
    assert_eq!(v, Value::from_int(20));
    // 1 live = ORIGIN record (static; intentional).
    assert_eq!(live, 1, "only static ORIGIN must remain live: {live}");
}

#[test]
fn cross_module_fn_returns_record_static_field() {
    let (v, _) = with_temp_dir(|dir| {
        fs::write(
            dir.join("lib.abe"),
            "pub type Cfg = { limit: Int }\nstatic CFG: Cfg = Cfg { limit: 6 }\npub fn get_limit() -> Int { CFG.limit }\n",
        ).expect("write lib");
        fs::write(
            dir.join("main.abe"),
            "use lib::{ get_limit }\nfn main() -> Int { let mut acc = 0; for i in 0..get_limit() { acc = acc + 1 }; acc }\n",
        ).expect("write main");
        run_files(&dir.join("main.abe"))
    });
    assert_eq!(v, Value::from_int(6));
}
