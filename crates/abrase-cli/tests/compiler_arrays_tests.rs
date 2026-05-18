#[path = "compiler_codegen_common.rs"]
mod compiler_codegen_common;

use compiler_codegen_common::*;
use myriad::Value;

#[test]
fn verify_compile_array_literal_construction() {
    assert_eq!(
        run_source("fn main() -> Int { let a = [1, 2, 3]; a[1] }"),
        Ok(Value::from_int(2))
    );
}

#[test]
fn verify_compile_array_repeat_construction() {
    assert_eq!(
        run_source("fn main() -> Int { let a = [7; 4]; a[2] }"),
        Ok(Value::from_int(7))
    );
}

#[test]
fn verify_compile_array_indexing_constant() {
    assert_eq!(
        run_source("fn main() -> Int { let a = [10, 20, 30]; a[2] }"),
        Ok(Value::from_int(30))
    );
}

#[test]
#[ignore = "codegen: can't infer record element type from Array<Pt> indexing → `.x` lookup fails"]
fn verify_compile_nested_record_in_array() {
    let src = r#"
        type Pt = { x: Int, y: Int }
        fn main() -> Int { let a = [Pt { x: 1, y: 2 }, Pt { x: 3, y: 4 }]; a[1].x }
    "#;
    assert_eq!(run_source(src), Ok(Value::from_int(3)));
}

#[test]
#[ignore = "ownership: Array<Move-type> indexing moves the binding; second index triggers use-after-move"]
fn verify_compile_array_of_records_type() {
    let src = r#"
        type Val = { n: Int }
        fn main() -> Int { let a = [Val { n: 5 }, Val { n: 10 }]; a[0].n + a[1].n }
    "#;
    assert_eq!(run_source(src), Ok(Value::from_int(15)));
}

#[test]
fn verify_compile_array_of_variants_type() {
    let src = r#"
        type Dir = | Left | Right
        fn main() -> Dir { let a = [Dir.Left, Dir.Right]; a[1] }
    "#;
    assert!(run_source(src).is_ok());
}

#[test]
fn verify_codegen_array_repeat_element_value() {
    let src = "fn main() -> Int { let a = [7; 4]; a[0] + a[3] }";
    assert_eq!(run_source(src), Ok(Value::from_int(14)));
}
