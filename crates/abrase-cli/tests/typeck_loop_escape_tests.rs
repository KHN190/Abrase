#[path = "compiler_codegen_common.rs"]
mod compiler_codegen_common;

use compiler_codegen_common::*;

fn expect_escape_err(src: &str) {
    match run_source(src) {
        Ok(v) => panic!("expected escape-barrier error, got Ok({:?})", v),
        Err(e) => assert!(
            e.contains("cannot escape") || e.contains("dangle"),
            "expected escape-barrier message, got: {}", e,
        ),
    }
}

fn expect_typeck_ok(src: &str) {
    if let Err(e) = run_source(src) {
        panic!("expected acceptance, got error: {}", e);
    }
}

//
// The Ref cell stores a snapshot of the bits (an i48 Int, in these cases).
// Codegen emits region_forget on the break/return value before the region
// pops, so the cell itself survives the unwind. No dangling — the bits in
// the cell don't reach back into the region.

#[test]
fn loop_break_with_inner_int_ref_permitted() {
    let src = r#"
        fn main() -> &Int {
            loop {
                let x = 42;
                break &x;
            }
        }
    "#;
    expect_typeck_ok(src);
}

#[test]
fn for_break_with_inner_int_ref_permitted() {
    let src = r#"
        fn main() -> Int {
            for i in 0..1 {
                let x = 42;
                break &x;
            };
            0
        }
    "#;
    expect_typeck_ok(src);
}

#[test]
fn while_break_with_inner_int_ref_permitted() {
    let src = r#"
        fn main() -> Int {
            while true {
                let x = 42;
                break &x;
            };
            0
        }
    "#;
    expect_typeck_ok(src);
}


#[test]
fn loop_break_with_inner_ref_binding_rejected() {
    // r itself is bound inside the loop body; even if its origin were outer,
    // the conservative check still rejects (origin tracking is future work).
    let src = r#"
        fn main() -> &Int {
            loop {
                let x = 42;
                let r = &x;
                break r;
            }
        }
    "#;
    expect_escape_err(src);
}


#[test]
fn loop_break_with_field_root_inside_rejected() {
    let src = r#"
        type Pt = { x: Int, y: Int }
        fn main() -> &Int {
            loop {
                let p = Pt { x: 1, y: 2 };
                break &p.x;
            }
        }
    "#;
    expect_escape_err(src);
}


#[test]
fn loop_break_with_outer_ref_accepted() {
    let src = r#"
        fn main() -> Int {
            let outer = 42;
            let r = loop { break &outer };
            *r
        }
    "#;
    expect_typeck_ok(src);
}


#[test]
fn nested_loop_inner_break_with_outer_body_local_accepted() {
    // x lives in outer for-body region; inner loop body region is deeper.
    // x outlives the inner loop, so `break &x` from the inner loop is safe.
    let src = r#"
        fn main() -> Int {
            let mut sum = 0;
            for i in 0..1 {
                let x = 5;
                let r = loop { break &x };
                sum = sum + *r;
            };
            sum
        }
    "#;
    expect_typeck_ok(src);
}

#[test]
fn return_from_loop_carrying_inner_heap_ref_rejected() {
    let src = r#"
        fn bad() -> &Array<Int> {
            loop {
                let xs = [1, 2, 3];
                return &xs;
            }
        }
        fn main() -> Int { 0 }
    "#;
    expect_escape_err(src);
}

#[test]
fn throw_from_loop_carrying_inner_heap_ref_rejected() {
    let src = r#"
        fn bad() -> <exn<Int>> Int {
            loop {
                let xs = [1, 2, 3];
                let r = &xs;
                throw r
            }
        }
        fn main() -> Int { 0 }
    "#;
    expect_escape_err(src);
}
