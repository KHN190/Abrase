use abrase::bytecode::{Chunk, OpCode};
use abrase::compiler::Compiler;
use abrase::lexer::Lexer;
use abrase::parser::Parser;
use abrase::typeck::Checker;
use myriad::{Value, VirtualMachine, read_string};
use std::fs;

// Compile `src` and return the entry function's opcodes (for bytecode-shape tests).
fn compile_entry_ops(src: &str) -> Vec<OpCode> {
    let mut parser = Parser::new(Lexer::new(src)).with_source(src.to_string());
    let ast = parser.parse_program();
    assert!(parser.errors.is_empty(), "{}", parser.pretty_print_errors());
    let mut compiler = Compiler::new().with_source(src.to_string());
    let module = compiler.compile_module(&ast)
        .unwrap_or_else(|_| panic!("\n{}", compiler.pretty_print_errors()));
    match &module.functions[module.entry] {
        Chunk::Bytecode(bc) => bc.code.clone(),
        _ => panic!("entry is not a bytecode chunk"),
    }
}

fn run_file(path: &str) -> Result<Value, String> {
    let (v, _) = run_file_full(path)?;
    Ok(v)
}

fn run_file_string(path: &str) -> Result<String, String> {
    let (v, vm) = run_file_full(path)?;
    read_string(vm.heap_ref(), v).ok_or_else(|| format!("expected String handle, got {:?}", v))
}

fn run_file_full(path: &str) -> Result<(Value, VirtualMachine), String> {
    let source = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    run_src_full(&source)
}

fn run_src(src: &str) -> Result<Value, String> {
    let (v, _) = run_src_full(src)?;
    Ok(v)
}

fn run_src_string(src: &str) -> Result<String, String> {
    let (v, vm) = run_src_full(src)?;
    read_string(vm.heap_ref(), v).ok_or_else(|| format!("expected String handle, got {:?}", v))
}

fn run_src_full(source: &str) -> Result<(Value, VirtualMachine), String> {
    let mut parser = Parser::new(Lexer::new(source)).with_source(source.to_string());
    let ast = parser.parse_program();

    if !parser.errors.is_empty() {
        return Err(format!("Parser errors:\n{}", parser.pretty_print_errors()));
    }

    if ast.is_empty() {
        return Err("Parser produced empty AST".to_string());
    }

    let mut compiler = Compiler::new().with_source(source.to_string());
    let module = compiler.compile_module(&ast)
        .map_err(|_| compiler.pretty_print_errors())?;

    let mut vm = VirtualMachine::new();
    let v = vm.run_module(&module)
        .map_err(|e| format!("VM error: {}", e))?;
    Ok((v, vm))
}

fn typeck_file(path: &str) -> Vec<String> {
    let source = fs::read_to_string(path).expect("script missing");
    let mut parser = Parser::new(Lexer::new(&source)).with_source(source.clone());
    let ast = parser.parse_program();
    assert!(parser.errors.is_empty(),
        "unexpected parse errors in {}: {}", path, parser.pretty_print_errors());
    let mut checker = Checker::new();
    checker.check_program(&ast);
    checker.errors.iter().map(|e| e.message.clone()).collect()
}

#[test]
fn arithmetic_recursion_and_loop() {
    // fib(10) = 55 via recursion; sum_to(10) = 55 via mut + while; total = 110.
    let v = run_file("tests/scripts/arithmetic.abe")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(110));
}

const CONST_DECL: &str = r#"
const N: Int = 2 + 3 * 5
const NEG: Int = -7
const FLAG: Bool = true && !(false || false)
const PI: Float = 3.0 + 0.14
const DERIVED: Int = N + NEG

fn main() -> Int {
    let local_override = {
        let N = 100;
        N
    };
    if FLAG {
        DERIVED + local_override
    } else {
        -1
    }
}
"#;

#[test]
fn test_const_decl() {
    let v = run_src(CONST_DECL)
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(110));
}

const STATIC_MUT: &str = r#"
static mut COUNT: Int = 0
static STEP: Int = 7

fn bump() -> Int { COUNT = COUNT + 1; COUNT }
fn add(x: Int) -> Int { COUNT = COUNT + x; COUNT }

fn main() -> Int {
    let _a = bump();
    let _b = bump();
    let _c = add(STEP);
    add(STEP)
}
"#;

#[test]
fn test_static_mut() {
    let v = run_src(STATIC_MUT)
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(16));
}

const STATIC_DROP: &str = r#"
fn build() -> Array<Int> { [10, 20, 30] }

static mut DATA: Array<Int> = build()

fn peek() -> Int {
    let d = DATA;
    d[1]
}

fn main() -> Int {
    let _a = peek();
    let _b = peek();
    peek()
}
"#;

#[test]
fn test_static_handle_drop_is_safe() {
    let v = run_src(STATIC_DROP)
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(20));
}

const STATIC_INIT_CALL: &str = r#"
fn build_bh(mh: Int) -> Array<Int> {
    let mut a = [0; 8];
    let mut y = 0;
    while y < 8 { a[y] = y * mh; y = y + 1 };
    a
}

static BH: Array<Int> = build_bh(110)

fn main() -> Int {
    BH[3]
}
"#;

#[test]
fn test_static_init_call() {
    let v = run_src(STATIC_INIT_CALL)
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(330));
}

const STATIC_UPDATE_FRAMES: &str = r#"
fn build_bh(mh: Int) -> Array<Int> {
    let mut a = [0; 8];
    let mut y = 0;
    while y < 8 { a[y] = y * mh; y = y + 1 };
    a
}

static BH: Array<Int> = build_bh(110)

pub fn update() -> Int {
    BH[3]
}

fn main() -> Int { 0 }
"#;

#[test]
fn test_static_update_frames_no_leak() {
    let source = STATIC_UPDATE_FRAMES;
    let mut parser = Parser::new(Lexer::new(source)).with_source(source.to_string());
    let ast = parser.parse_program();
    assert!(parser.errors.is_empty(), "{}", parser.pretty_print_errors());
    let mut compiler = Compiler::new().with_source(source.to_string());
    let module = compiler.compile_module(&ast)
        .unwrap_or_else(|_| panic!("\n{}", compiler.pretty_print_errors()));

    let mut vm = VirtualMachine::new();
    let mut counts = Vec::new();
    for _ in 0..50 {
        let v = vm.call_export(&module, "update", &[])
            .unwrap_or_else(|e| panic!("\n{}", e));
        assert_eq!(v, Value::from_int(330));
        counts.push(vm.heap_live_count());
    }
    let plateau = counts[1];
    assert!(counts[1..].iter().all(|&c| c == plateau),
        "heap not flat across frames: {:?}", &counts);
}

#[test]
fn vm_counts_executed_steps() {
    let (_v, vm) = run_file_full("tests/scripts/arithmetic.abe")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert!(vm.steps() > 0, "step counter must advance after a run");
}

const FIELD_ASSIGN: &str = r#"
type Pt = { x: Int, y: Int }

fn main() -> Int {
    let mut p = Pt { x: 10, y: 20 };
    p.x = 100;
    p.y = p.x + p.y;
    p.x + p.y
}
"#;

#[test]
fn test_field_assign() {
    let v = run_src(FIELD_ASSIGN)
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(220));
}

const SHARED_DEREF_WRITE: &str = r#"
fn main() -> Int {
    region r {
        let s = Shared(1);
        *s = 7;
        *s = *s + 35;
        *s
    }
}
"#;

#[test]
fn test_shared_deref_write() {
    let v = run_src(SHARED_DEREF_WRITE)
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(42));
}

const HANDLER_LET_MUT_CELL: &str = r#"
effect Counter { op tick() -> Unit }

fn fire_three() -> <Counter> Unit {
    Counter.tick();
    Counter.tick();
    Counter.tick();
}

fn main() -> Int {
    let mut n = 0;
    handle fire_three() {
        return _   => n,
        Counter.tick => {
            n = n + 1;
            resume(())
        }
    }
}
"#;

#[test]
fn test_handler_let_mut_cell() {
    let v = run_src(HANDLER_LET_MUT_CELL)
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(3));
}

#[test]
fn test_bst() {
    let v = run_file("tests/scripts/bst.abe")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(15));
}

const SHAPES: &str = r#"
type Pt = { x: Int, y: Int }

fn dist_sq(p: Pt) -> Int {
  p.x * p.x + p.y * p.y
}

fn main() -> Int {
  let a: Pt = Pt { x: 1, y: 2 };
  let b: Pt = Pt { x: 3, y: 4 };
  let c: Pt = Pt { x: 5, y: 12 };
  let pts = [a, b, c];
  dist_sq(pts[2])
}
"#;

#[test]
fn test_shapes() {
    // record decl + literal + field access + array + indexing + function call
    let v = run_src(SHAPES)
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(169));
}

#[test]
fn test_memory() {
    // &/* (ref+deref) + Shared (heap alloc/load) + Move (String) + scope-exit drop
    let v = run_file("tests/scripts/memory.abe")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(30));
}

#[test]
fn exceptions_ok_and_err_paths() {
    // pipeline(20,4) hits `?` happy path -> Ok(6); pipeline(10,0) throws -> Err -> 1.
    let v = run_file("tests/scripts/exceptions.abe")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(7));
}

const CLOSURES: &str = r#"
fn main() -> Int {
  let no_cap = |x| x + x;
  let r1 = no_cap(7);

  let x = 5;
  let one_cap = |y| x + y;
  let r2 = one_cap(3);

  let a = 1;
  let b = 2;
  let multi_cap = |c| a + b + c;
  let r3 = multi_cap(3);

  r1 + r2 + r3
}
"#;

#[test]
fn closures_no_single_and_multi_capture() {
    // no_cap(7)=14 + one_cap(3)=8 (captures x=5) + multi_cap(3)=6 (captures a=1,b=2)
    let v = run_src(CLOSURES)
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(28));
}

#[test]
fn closures_complex_bodies() {
    let v = run_file("tests/scripts/closures_complex.abe")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(145));
}

#[test]
fn destructuring_tuple_record_array() {
    let v = run_file("tests/scripts/destructuring.abe")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(1260));
}

#[test]
fn effect_dispatch_runs() {
    let v = run_file("tests/scripts/effect_dispatch.abe")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(22));
}

const MULTI_EFFECT: &str = r#"
effect scale { op apply(x: Int) -> Int }

fn transform(a: Int, b: Int) -> <scale> Int {
  let x = scale.apply(a);
  let y = scale.apply(b);
  x + y
}

fn main() -> Int {
  handle transform(3, 7) {
    scale.apply x => resume(x * 2)
    return v      => v
  }
}
"#;

#[test]
fn multiple_suspension_points() {
    let v = run_src(MULTI_EFFECT)
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(20));
}

#[test]
fn effect_resume_paths_nested_handlers_and_return_arm() {
    let v = run_file("tests/scripts/effect_resume_paths.abe")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(2050));
}

#[test]
fn region_all_allowed_shapes() {
    let (v, vm) = run_file_full("tests/scripts/region.abe")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(382),
        "11 (a) + 20 (b) + 300 (c) + 30 (d) + 6 (e) + 7 (f) + 8 (g)");
    assert_eq!(vm.heap_live_count(), 0,
        "all region-tagged allocs must be force-freed at exit, got live={}",
        vm.heap_live_count());
}

#[test]
fn effect_handlers_typecheck() {
    let errs = typeck_file("tests/scripts/effect_handlers.abe");
    assert!(errs.is_empty(),
        "expected no typeck errors for effect handler patterns, got: {:?}", errs);
}

const TRAITS: &str = r#"
trait Doubler {
  fn double(self) -> Int { 0 }
}

impl Doubler for Int {
  fn double(self) -> Int {
    self * 2
  }
}

fn id<T>(x: T) -> T { x }

fn main() -> Int {
  let flag = id(true);
  let n = id(42);
  let d = (5).double();
  if flag { n + d } else { d }
}
"#;

#[test]
fn traits_and_generics() {
    let v = run_src(TRAITS)
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(52));
}

#[test]
fn generics_with_bounds() {
    let v = run_file("tests/scripts/generics.abe")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(99));
}

#[test]
fn generic_bound_violation_rejected() {
    // `show` requires T: ToS. Calling with a record that lacks impl ToS for it
    // should be rejected by typeck.
    let src = r#"
        type Bag = { n: Int }
        fn show<T>(x: T) -> String where T: ToS { x.to_s() }
        fn main() -> Int { let _ = show(Bag { n: 1 }); 0 }
    "#;
    let mut compiler = abrase::compiler::Compiler::new().with_source(src.into());
    let mut p = abrase::parser::Parser::new(abrase::lexer::Lexer::new(src));
    let ast = p.parse_program();
    let result = compiler.compile_module(&ast);
    assert!(result.is_err(), "expected typeck error for Bag : ToS violation");
}

#[test]
fn generic_overload_restriction() {
    let src = r#"
        fn foo<T>(x: T) -> T { x }
        fn main() -> Int { 0 }
    "#;
    let mut compiler = abrase::compiler::Compiler::new().with_source(src.into());
    let mut p = abrase::parser::Parser::new(abrase::lexer::Lexer::new(src));
    let ast = p.parse_program();
    let result = compiler.compile_module(&ast);
    assert!(result.is_ok(), "plain generic fn should compile, got {:?}", result.err());
}

#[test]
#[ignore = "codegen: chained generic method call .max().to_s() — receiver type inference loses T's bounds"]
fn generic_chained_method_via_bound() {
    let src = r#"
        fn show_max<T>(a: T, b: T) -> String where T: Ord, T: ToS {
          a.max(b).to_s()
        }
        fn main() -> Int { let _ = show_max(3, 7); 0 }
    "#;
    let mut compiler = abrase::compiler::Compiler::new().with_source(src.into());
    let mut p = abrase::parser::Parser::new(abrase::lexer::Lexer::new(src));
    let ast = p.parse_program();
    let result = compiler.compile_module(&ast);
    assert!(result.is_ok(), "expected ok compile, got {:?}", result.err());
}

#[test]
fn string_interp_with_records_recursion_and_closures() {
    let v = run_file_string("tests/scripts/interp.abe")
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, "user=[Alice:30] total=10 next=11");
}

#[test]
fn built_ins() {
    // print / math / type conversions — all core natives, no clock/random.
    let src = fs::read_to_string("tests/scripts/built_ins.abe")
        .expect("built_ins.abe missing");
    let (mut rt, console) = abrase_cli::host::Runtime::new_for_tests();
    let v = rt.eval(&src).unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(0), "main should return 0");
    let (out_handle, _) = console.handles();
    let out = String::from_utf8(out_handle.borrow().clone()).expect("stdout utf-8");
    assert!(out.contains("hello, myriad"),    "missing greeting in: {:?}", out);
    assert!(out.contains("7.min(3)=3"),       "Int .min() broken in: {:?}", out);
    assert!(out.contains("7.max(3)=7"),       "Int .max() broken in: {:?}", out);
    assert!(out.contains("(-9).abs()=9"),     "Int .abs() broken in: {:?}", out);
    assert!(out.contains("sqrt(16)=4"),       "sqrt broken in: {:?}", out);
    assert!(out.contains("ceil(2.3)=3"),      "ceil broken in: {:?}", out);
    assert!(out.contains("flr(2.7)=2"),       "flr broken in: {:?}", out);
    assert!(out.contains("(-3.5).abs()=3.5"), "Float .abs() broken in: {:?}", out);
    assert!(out.contains("1.5.max(2.5)=2.5"), "Float .max() broken in: {:?}", out);
    assert!(out.contains("1.5.min(2.5)=1.5"), "Float .min() broken in: {:?}", out);
    assert!(out.contains("7.to_f()=7"),       ".to_f() broken in: {:?}", out);
    assert!(out.contains("3.9.to_i()=3"),     ".to_i() (Float→Int trunc) broken in: {:?}", out);
    assert!(out.contains("'A'.to_i()=65"),    ".to_i() (Char→Int) broken in: {:?}", out);
    assert!(out.contains("66.to_c()=B"),      ".to_c() (Int→Char) broken in: {:?}", out);
    assert!(out.contains("true.to_i()=1"),    "Bool→Int broken in: {:?}", out);
    assert!(out.contains("42.to_s()=42"),     "Int.to_s broken in: {:?}", out);
    assert!(out.contains("3.14.to_s()=3.14"), "Float.to_s broken in: {:?}", out);
    assert!(out.contains("false.to_s()=false"),"Bool.to_s broken in: {:?}", out);
    assert!(out.contains("'Z'.to_s()=Z"),     "Char.to_s broken in: {:?}", out);
}

const ARRAY_INDEX_METHOD: &str = r#"
fn first_str(xs: Array<Float>) -> String {
    xs[0].to_s()
}

fn main() -> String {
    let fa = [1.5; 4];
    first_str(fa)
}
"#;

#[test]
fn method_call_on_array_index_infers_element_type() {
    let v = run_src_string(ARRAY_INDEX_METHOD)
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, "1.5");
}

const STATIC_FLOAT_ARRAY_ACCUMULATE: &str = r#"
static AR: Array<Float> = [0.0; 5]

fn run() -> Float {
  let mut i = 0;
  while i < 5 { AR[3] = AR[3] + 1.0; i = i + 1 };
  AR[3]
}

fn main() -> Int { run().to_i() }
"#;

#[test]
fn static_float_array_in_place_add_accumulates() {
    // AR[3] = AR[3] + 1.0 in a loop must use float add, not int add on the
    // bit pattern — static element types must be inferred as Float.
    let v = run_src(STATIC_FLOAT_ARRAY_ACCUMULATE)
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(5));
}

const CONST_FLOAT_TIMES_CALL: &str = r#"
const SQ: Float = 2.0
fn itof(x: Int) -> Float { x.to_f() }
fn main() -> Int {
  let a = SQ * itof(1);
  let b = itof(1) * SQ;
  (a + b).to_i()
}
"#;

#[test]
fn const_float_times_call_uses_float_mul() {
    // const-Float operand must infer as Float so SQ * itof(1) emits FMul, not
    // an integer multiply over float bit patterns (which read back as 0).
    let v = run_src(CONST_FLOAT_TIMES_CALL)
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(4));
}

const STATIC_SUM3: &str = r#"
static mut A: Int = 10
static mut B: Int = 20
static mut C: Int = 30
fn main() -> Int { A + B + C }
"#;

#[test]
fn static_reads_share_one_module_table_load() {
    let ops = compile_entry_ops(STATIC_SUM3);
    // O1: the three static reads share a single module-table load (Dei).
    let deis: Vec<_> = ops.iter()
        .filter_map(|o| if let OpCode::Dei(d, _) = o { Some(*d) } else { None })
        .collect();
    assert_eq!(deis.len(), 1, "expected 1 Dei for 3 static reads, got {}: {ops:?}", deis.len());
    let table = deis[0];

    // O2: every scalar static value (Ld from the table) is never Drop-ed.
    let scalar_vals: Vec<_> = ops.iter()
        .filter_map(|o| if let OpCode::Ld(d, b, _) = o { (*b == table).then_some(*d) } else { None })
        .collect();
    assert_eq!(scalar_vals.len(), 3, "expected 3 static Lds: {ops:?}");
    for d in &scalar_vals {
        assert!(!ops.iter().any(|o| matches!(o, OpCode::Drop(x) if x == d)),
            "scalar static value r{} must not be Drop-ed (O2): {ops:?}", d.0);
    }
}

const STATIC_LOOP_ACCUM: &str = r#"
static mut A: Int = 10
static mut B: Int = 20
static mut C: Int = 30
fn main() -> Int {
  let r = A + B + C;
  let mut i = 0;
  let mut acc = 0;
  while i < 3 { acc = acc + A + B + C; i = i + 1 };
  r + acc
}
"#;

#[test]
fn cached_module_table_stays_correct_across_loop_iterations() {
    let v = run_src(STATIC_LOOP_ACCUM)
        .unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(240));
}

const STATIC_IN_LOOP: &str = r#"
static mut A: Int = 1
static mut B: Int = 2
fn main() -> Int {
  let mut i = 0;
  let mut acc = 0;
  while i < 4 { acc = acc + A + B; i = i + 1 };
  acc
}
"#;

#[test]
fn loop_hoists_module_table_load_out_of_body() {
    let ops = compile_entry_ops(STATIC_IN_LOOP);
    let dei = ops.iter().filter(|o| matches!(o, OpCode::Dei(..))).count();
    assert_eq!(dei, 1, "O3: module-table load must be hoisted before the loop, got {dei} Dei: {ops:?}");
    let v = run_src(STATIC_IN_LOOP).unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(12));
}

const ARITH_LOOP: &str = r#"
fn main() -> Int {
  let mut i = 0;
  let mut acc = 0;
  while i < 10 { acc = acc + i * 2; i = i + 1 };
  acc
}
"#;

#[test]
fn alloc_free_loop_elides_region_markers() {
    let ops = compile_entry_ops(ARITH_LOOP);
    let deo = ops.iter().filter(|o| matches!(o, OpCode::Deo(..))).count();
    assert_eq!(deo, 0, "alloc-free loop must emit no per-iteration region markers: {ops:?}");
    assert_eq!(run_src(ARITH_LOOP).unwrap_or_else(|e| panic!("\n{}", e)), Value::from_int(90));
}

const SHARED_LOOP: &str = r#"
fn main() -> Int {
  let mut i = 0;
  let mut acc = 0;
  while i < 5 { let s = Shared(i + 1); acc = acc + *s; i = i + 1 };
  acc
}
"#;

#[test]
fn allocating_loop_keeps_region_and_frees_per_iteration() {
    let ops = compile_entry_ops(SHARED_LOOP);
    let deo = ops.iter().filter(|o| matches!(o, OpCode::Deo(..))).count();
    assert!(deo >= 2, "allocating loop must keep its per-iteration region push/pop: {ops:?}");
    let (v, vm) = run_src_full(SHARED_LOOP).unwrap_or_else(|e| panic!("\n{}", e));
    assert_eq!(v, Value::from_int(15));
    assert_eq!(vm.heap_live_count(), 0, "per-iteration Shared must be freed, got live={}", vm.heap_live_count());
}
