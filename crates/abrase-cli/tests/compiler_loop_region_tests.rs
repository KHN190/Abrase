// Tests for the implicit per-iteration region 

#[path = "compiler_codegen_common.rs"]
mod compiler_codegen_common;

use compiler_codegen_common::*;

fn run_with_heap(src: &str) -> Result<(Value, usize), String> {
    let ast = parse_source(src);
    let mut compiler = Compiler::new();
    let module = compiler.compile_module(&ast).map_err(|errs| {
        errs.iter()
            .map(|e| format!("{:?} at {}:{}: {}", e.code, e.span.line, e.span.col, e.message))
            .collect::<Vec<_>>()
            .join("\n")
    })?;
    let mut vm = VirtualMachine::new();
    let v = vm.run_module(&module)?;
    Ok((v, vm.heap_live_count()))
}

// `while` body's per-iter region should free `&i` allocations each iter.
#[test]
fn while_per_iter_region_frees_refs() {
    let src = r#"
        fn main() -> Int {
            let mut i = 0;
            while i < 100 {
                let r = &i;
                i = i + 1;
            };
            i
        }
    "#;
    let (val, live) = run_with_heap(src).expect("while loop must run cleanly");
    assert_eq!(val, Value::from_int(100));
    assert_eq!(live, 0, "per-iter Refs should be freed; heap_live_count = {}", live);
}

// break unwinds the per-iter region.
#[test]
fn loop_break_pops_region_cleanly() {
    let src = r#"
        fn main() -> Int {
            let mut i = 0;
            loop {
                let r = &i;
                if i == 50 { break };
                i = i + 1;
            };
            i
        }
    "#;
    let (val, live) = run_with_heap(src).expect("loop with break must run");
    assert_eq!(val, Value::from_int(50));
    assert_eq!(live, 0, "break must pop the iter's region; live = {}", live);
}

// `for` loop counterpart: per-iter region must close between increments.
#[test]
fn for_per_iter_region_frees_refs() {
    let src = r#"
        fn main() -> Int {
            let mut acc = 0;
            for i in 0..50 {
                let r = &i;
                acc = acc + i;
            };
            acc
        }
    "#;
    let (val, live) = run_with_heap(src).expect("for loop must run");
    assert_eq!(val, Value::from_int(50 * 49 / 2));
    assert_eq!(live, 0, "for-loop per-iter region should free Refs; live = {}", live);
}

// `continue` mid-body must still pop the per-iter region; otherwise refs
// from continue-skipped iterations would accumulate.
#[test]
fn loop_continue_pops_region_cleanly() {
    let src = r#"
        fn main() -> Int {
            let mut i = 0;
            let mut taken = 0;
            while i < 30 {
                let r = &i;
                i = i + 1;
                if i % 2 == 0 { continue };
                taken = taken + 1;
            };
            taken
        }
    "#;
    let (val, live) = run_with_heap(src).expect("continue must not leak");
    assert_eq!(val, Value::from_int(15));
    assert_eq!(live, 0, "continue must pop iter region; live = {}", live);
}

// `return` from inside a loop must unwind every compiler-emitted region
#[test]
fn return_from_inside_loop_unwinds_all_regions() {
    let src = r#"
        fn run() -> Int {
            let mut i = 0;
            loop {
                let r = &i;
                if i == 7 { return i };
                i = i + 1;
            }
        }

        fn main() -> Int { run() }
    "#;
    let (val, live) = run_with_heap(src).expect("return from inside loop must run");
    assert_eq!(val, Value::from_int(7));
    assert_eq!(live, 0, "return must clear loop's region too; live = {}", live);
}

// Nested loops: break exits only the inner loop and unwinds exactly.
#[test]
fn nested_loops_break_only_inner() {
    let src = r#"
        fn main() -> Int {
            let mut outer = 0;
            let mut acc = 0;
            while outer < 5 {
                let mut inner = 0;
                while inner < 10 {
                    let r = &inner;
                    if inner == outer { break };
                    inner = inner + 1;
                };
                acc = acc + inner;
                outer = outer + 1;
            };
            acc
        }
    "#;
    let (val, live) = run_with_heap(src).expect("nested loops + break must run");
    // inner ends at `inner == outer` (0..=4), sum = 0+1+2+3+4 = 10
    assert_eq!(val, Value::from_int(10));
    assert_eq!(live, 0, "nested break must balance both regions; live = {}", live);
}

// A bare block in statement position should open and close its own region.
#[test]
fn stmt_position_block_auto_regions() {
    let src = r#"
        fn make() -> Int {
            let n = 7;
            let r = &n;
            42
        }

        fn main() -> Int {
            make();
            0
        }
    "#;
    let (val, live) = run_with_heap(src).expect("stmt-position must run");
    assert_eq!(val, Value::from_int(0));
    assert_eq!(live, 0, "stmt-position block must close its region; live = {}", live);
}

// heap (arrays, records, Refs) can ESCAPE a per-iter region
// via `break`: but they are later freed by Rc.
#[test]
fn break_heap_array_from_loop_then_freed_by_rc() {
    let src = r#"
        fn main() -> Int {
            let xs = loop { break [10, 20, 30] };
            xs[1]
        }
    "#;
    let (val, live) = run_with_heap(src).expect("break-with-array must work");
    assert_eq!(val, Value::from_int(20));
    assert_eq!(live, 0,
        "array survives break via region_forget, then Drop at main's block end \
         frees it via rc_dec; live = {}", live);
}

#[test]
fn break_ref_to_local_primitive_permitted() {
    // &i copies the int's bits into a Ref cell; the cell is region-forgotten
    // at break, so dereferencing after the loop returns the snapshot.
    let src = r#"
        fn main() -> Int {
            let r = loop {
                let i = 42;
                break &i
            };
            *r
        }
    "#;
    let v = run_source(src).expect("break-&-primitive must work");
    assert_eq!(v, Value::from_int(42));
}

#[test]
fn return_heap_array_from_inside_loop_permitted() {
    let src = r#"
        fn produce() -> Int {
            loop {
                let xs = [7, 8, 9];
                return xs[2]
            }
        }

        fn main() -> Int { produce() }
    "#;
    let v = run_source(src).expect("return-with-array-derived-int must work");
    assert_eq!(v, Value::from_int(9));
}

#[test]
fn return_from_inside_user_region_pops_user_region() {
    let src = r#"
        fn produce() -> Int {
            region {
                region {
                    return 11
                }
            }
        }

        fn main() -> Int { produce() }
    "#;
    let (v, live) = run_with_heap(src).expect("return inside user region must work");
    assert_eq!(v, Value::from_int(11));
    assert_eq!(live, 0, "user regions must be unwound by return; live = {}", live);
}

#[test]
fn break_inside_user_region_inside_loop_pops_both() {
    // loop { region { break } } — break must pop the user region AND the
    // loop's per-iter region. Verifies unified counter.
    let src = r#"
        fn main() -> Int {
            loop {
                region {
                    break 5
                }
            }
        }
    "#;
    let (v, live) = run_with_heap(src).expect("break across user region must work");
    assert_eq!(v, Value::from_int(5));
    assert_eq!(live, 0, "both regions must unwind on break; live = {}", live);
}

// re-running must not carry over heap from a prior module
#[test]
fn region_table_cleared_across_evals() {
    let ast = parse_source(r#"
        fn main() -> Int {
            let mut i = 0;
            while i < 10 {
                let r = &i;
                i = i + 1;
            };
            i
        }
    "#);
    let mut compiler = Compiler::new();
    let module = compiler.compile_module(&ast).expect("compile");
    let mut vm = VirtualMachine::new();
    for run_idx in 0..3 {
        let v = vm.run_module(&module).expect("each run must succeed");
        assert_eq!(v, Value::from_int(10));
        assert_eq!(vm.heap_live_count(), 0,
            "run #{}: heap must drain between runs; live = {}",
            run_idx, vm.heap_live_count());
    }
}

// Abnormal exits unwind block-binders, not just regions.
#[test]
fn break_drops_move_typed_binders_in_path() {
    let src = r#"
        fn main() -> Int {
            let r = loop {
                let s = "scratch";
                break 7
            };
            r
        }
    "#;
    let (val, live) = run_with_heap(src).expect("break with move-typed binder must run");
    assert_eq!(val, Value::from_int(7));
    assert_eq!(live, 0, "string binder must be Dropped on break; live = {}", live);
}

#[test]
fn return_drops_move_typed_binders_in_path() {
    let src = r#"
        fn produce() -> Int {
            let s = "scratch";
            return 11
        }

        fn main() -> Int { produce() }
    "#;
    let (val, live) = run_with_heap(src).expect("return must drop block binders");
    assert_eq!(val, Value::from_int(11));
    assert_eq!(live, 0, "string must be Dropped on return; live = {}", live);
}

// Continue must Drop binders too, otherwise iterating leaks one box per round.
#[test]
fn continue_drops_move_typed_binders_per_iter() {
    let src = r#"
        fn main() -> Int {
            let mut acc = 0;
            let mut i = 0;
            while i < 50 {
                let s = "iter-local";
                i = i + 1;
                if i % 2 == 0 { continue };
                acc = acc + 1;
            };
            acc
        }
    "#;
    let (val, live) = run_with_heap(src).expect("continue with binder must run");
    assert_eq!(val, Value::from_int(25));
    assert_eq!(live, 0, "string per iter must be Dropped on continue; live = {}", live);
}

#[test]
fn break_drops_all_move_typed_binders_in_layer() {
    let src = r#"
        fn main() -> Int {
            let r = loop {
                let a = "first";
                let b = "second";
                let c = "third";
                break 9
            };
            r
        }
    "#;
    let (val, live) = run_with_heap(src).expect("break with multiple binders must run");
    assert_eq!(val, Value::from_int(9));
    assert_eq!(live, 0, "every binder in layer must be Dropped; live = {}", live);
}

// Closure boxes own a heap env cell via env_slot/env_gen. BoxPool::dec_cascade
// rc_decs that cell when the closure's box reaches rc=0; without the cascade
// the env cell would linger until the next eval clears the whole heap.
#[test]
fn closure_box_cascade_frees_env_cell() {
    let src = r#"
        fn main() -> Int {
            let outer = 10;
            let f = |y| outer + y;
            f(5)
        }
    "#;
    let (val, live) = run_with_heap(src).expect("closure with capture must run");
    assert_eq!(val, Value::from_int(15));
    assert_eq!(live, 0,
        "closure box must cascade-dec its env_slot on reclaim; live = {}", live);
}

// Closure with multiple captures: env cell holds two fields. Cascade must
// still reclaim the whole env (rc_dec on the env handle recurses through
// the cell's contents).
#[test]
fn closure_multi_capture_env_freed() {
    let src = r#"
        fn main() -> Int {
            let a = 1;
            let b = 2;
            let f = |c| a + b + c;
            f(3)
        }
    "#;
    let (val, live) = run_with_heap(src).expect("multi-capture closure must run");
    assert_eq!(val, Value::from_int(6));
    assert_eq!(live, 0,
        "multi-capture env cell must be reclaimed via cascade; live = {}", live);
}
