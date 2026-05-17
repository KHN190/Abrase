use abrase::lexer::Lexer;
use abrase::parser::Parser;
use abrase::typeck::Checker;

fn typeck(src: &str) -> Vec<String> {
    let mut parser = Parser::new(Lexer::new(src)).with_source(src.into());
    let ast = parser.parse_program();
    assert!(parser.errors.is_empty(), "parse errors: {:?}", parser.errors);
    let mut checker = Checker::new();
    checker.check_program(&ast);
    checker.errors.into_iter().map(|e| e.message).collect()
}

fn expect_escape_err(src: &str) {
    let msgs = typeck(src);
    assert!(
        msgs.iter().any(|m| m.contains("cannot escape") || m.contains("dangle")),
        "expected escape-barrier error, got: {:?}", msgs,
    );
}

fn expect_no_escape_err(src: &str) {
    let msgs = typeck(src);
    assert!(
        !msgs.iter().any(|m| m.contains("cannot escape") || m.contains("dangle")),
        "expected no escape-barrier error, got: {:?}", msgs,
    );
}

//
// The Ref cell stores a snapshot of the bits (an i48 Int, in these cases).
// Codegen emits region_forget on the break/return value before the region
// pops, so the cell itself survives the unwind. No dangling — the bits in
// the cell don't reach back into the region.
//
// These tests previously asserted rejection; that policy predates
// region_forget. See compiler_loop_region_tests.rs for the runtime side.

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
    expect_no_escape_err(src);
}

#[test]
fn loop_return_with_inner_int_ref_permitted() {
    let src = r#"
        fn main() -> Int {
            loop {
                let x = 42;
                return &x;
            };
            0
        }
    "#;
    expect_no_escape_err(src);
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
    expect_no_escape_err(src);
}

#[test]
fn for_return_with_inner_int_ref_permitted() {
    let src = r#"
        fn main() -> Int {
            for i in 0..1 {
                let x = 42;
                return &x;
            };
            0
        }
    "#;
    expect_no_escape_err(src);
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
    expect_no_escape_err(src);
}

#[test]
fn while_return_with_inner_int_ref_permitted() {
    let src = r#"
        fn main() -> Int {
            while true {
                let x = 42;
                return &x;
            };
            0
        }
    "#;
    expect_no_escape_err(src);
}


#[test]
fn loop_break_with_inner_ref_binding_rejected() {
    // r itself is bound inside the loop body; carries an &Int from `x` (also inside).
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
    expect_no_escape_err(src);
}


#[test]
fn nested_loop_inner_break_with_outer_body_local_accepted() {
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
    expect_no_escape_err(src);
}
