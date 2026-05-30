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

#[test]
fn array_indexed_assign_writes_slot() {
    let src = "fn main() -> Int { let mut a = [0; 4]; a[2] = 42; a[2] }";
    assert_eq!(run_source(src), Ok(Value::from_int(42)));
}

#[test]
fn array_indexed_assign_independent_slots() {
    let src = "fn main() -> Int { let mut a = [0; 4]; a[0] = 10; a[1] = 20; a[2] = 30; a[3] = 40; a[0] + a[1] + a[2] + a[3] }";
    assert_eq!(run_source(src), Ok(Value::from_int(100)));
}

#[test]
fn array_indexed_assign_overwrites_initial_value() {
    let src = "fn main() -> Int { let mut a = [1, 2, 3]; a[1] = 99; a[0] + a[1] + a[2] }";
    assert_eq!(run_source(src), Ok(Value::from_int(103)));
}

#[test]
fn array_indexed_assign_rejects_immutable_binding() {
    let src = "fn main() -> Int { let a = [0; 3]; a[0] = 1; a[0] }";
    let err = run_source(src).unwrap_err();
    assert!(err.contains("immutable") || err.contains("mut"),
        "expected immutable-binding error, got: {}", err);
}

#[test]
fn array_indexed_assign_in_place_through_mut_borrow() {
    let src = r#"
        fn fill(xs: &mut Array<Int>, i: Int, n: Int) -> Int {
            if i >= n { 0 } else { (*xs)[i] = i * i; fill(xs, i + 1, n) }
        }
        fn main() -> Int {
            let mut a = [0; 5];
            fill(&mut a, 0, 5);
            a[0] + a[1] + a[2] + a[3] + a[4]
        }
    "#;
    assert_eq!(run_source(src), Ok(Value::from_int(0 + 1 + 4 + 9 + 16)));
}

#[test]
fn float_arith_on_element_indexed_from_array_returning_fn() {
    let src = r#"
        fn make() -> Array<Float> { [1.0, 2.0, 3.0] }
        fn main() -> Float { let v = make(); v[0] + v[1] }
    "#;
    assert_eq!(run_source(src), Ok(Value::from_float(3.0)));
}

#[test]
fn float_sub_assign_on_var_from_array_returning_fn() {
    let src = r#"
        fn make() -> Array<Float> { [1.0, 2.0, 3.0] }
        fn main() -> Float {
            let v = make();
            let mut x = v[0];
            x = x - 0.5;
            x
        }
    "#;
    let result = run_source(src).unwrap();
    assert!(!result.as_float().is_nan(), "got NaN: integer Sub emitted instead of FSub");
    assert_eq!(result, Value::from_float(0.5));
}

#[test]
fn float_mul_assign_on_var_from_array_returning_fn() {
    let src = r#"
        fn make() -> Array<Float> { [1.5, 2.5, 3.5] }
        fn main() -> Float {
            let v = make();
            let mut x = v[0];
            x = x * 2.0;
            x
        }
    "#;
    let result = run_source(src).unwrap();
    assert!(!result.as_float().is_nan(), "got NaN: integer Mul emitted instead of FMul");
    assert_eq!(result, Value::from_float(3.0));
}
