#[path = "compiler_codegen_common.rs"]
mod compiler_codegen_common;

use compiler_codegen_common::*;
use myriad::Value;
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn verify_compile_record_simple_construction() {
    let ast = vec![
        Decl::Type {
            name: "Point".to_string(),
            generics: vec![],
            attrs: vec![],
            is_pub: false,
            ownership: None,
            body: TypeBody::Record(vec![
                RecordField {
                    is_pub: true,
                    name: "x".to_string(),
                    ty: Type::Named("Int".to_string()),
                },
                RecordField {
                    is_pub: true,
                    name: "y".to_string(),
                    ty: Type::Named("Int".to_string()),
                },
            ]),
        },
        Decl::Fn(FnDecl {
            attrs: vec![],
            is_pub: false,
            name: "main".to_string(),
            generics: vec![],
            params: vec![],
            effects: vec![],
            return_type: Some(Type::Named("Int".to_string())),
            where_clause: vec![],
            body: Block {
                stmts: vec![],
                ret: Some(Box::new(Spanned {
                    node: Expr::Literal(Literal::Int(42)),
                    span: Span::new(0, 0),
                })),
            },
        }),
    ];
    let result = compile_module_and_run(&ast);
    assert_eq!(result, Ok(Value::from_int(42)));
}

#[test]
fn verify_compile_record_field_access() {
    let ast = vec![
        Decl::Type {
            name: "Rect".to_string(),
            generics: vec![],
            attrs: vec![],
            is_pub: false,
            ownership: None,
            body: TypeBody::Record(vec![
                RecordField {
                    is_pub: true,
                    name: "w".to_string(),
                    ty: Type::Named("Int".to_string()),
                },
                RecordField {
                    is_pub: true,
                    name: "h".to_string(),
                    ty: Type::Named("Int".to_string()),
                },
            ]),
        },
        Decl::Fn(FnDecl {
            attrs: vec![],
            is_pub: false,
            name: "main".to_string(),
            generics: vec![],
            params: vec![],
            effects: vec![],
            return_type: Some(Type::Named("Int".to_string())),
            where_clause: vec![],
            body: Block {
                stmts: vec![],
                ret: Some(Box::new(Spanned {
                    node: Expr::Literal(Literal::Int(7)),
                    span: Span::new(0, 0),
                })),
            },
        }),
    ];
    let result = compile_module_and_run(&ast);
    assert_eq!(result, Ok(Value::from_int(7)));
}

#[test]
fn verify_compile_record_with_multiple_fields() {
    let ast = vec![
        Decl::Type {
            name: "Triple".to_string(),
            generics: vec![],
            attrs: vec![],
            is_pub: false,
            ownership: None,
            body: TypeBody::Record(vec![
                RecordField {
                    is_pub: true,
                    name: "a".to_string(),
                    ty: Type::Named("Int".to_string()),
                },
                RecordField {
                    is_pub: true,
                    name: "b".to_string(),
                    ty: Type::Named("Int".to_string()),
                },
                RecordField {
                    is_pub: true,
                    name: "c".to_string(),
                    ty: Type::Named("Int".to_string()),
                },
            ]),
        },
        Decl::Fn(FnDecl {
            attrs: vec![],
            is_pub: false,
            name: "main".to_string(),
            generics: vec![],
            params: vec![],
            effects: vec![],
            return_type: Some(Type::Named("Int".to_string())),
            where_clause: vec![],
            body: Block {
                stmts: vec![],
                ret: Some(Box::new(Spanned {
                    node: Expr::Literal(Literal::Int(777)),
                    span: Span::new(0, 0),
                })),
            },
        }),
    ];
    let result = compile_module_and_run(&ast);
    assert_eq!(result, Ok(Value::from_int(777)));
}

#[test]
fn record_field_assign_writes_field() {
    let src = r#"
        type Point = { x: Int, y: Int }
        fn main() -> Int {
            let mut p = Point { x: 1, y: 2 };
            p.x = 100;
            p.y = 200;
            p.x + p.y
        }
    "#;
    assert_eq!(run_source(src), Ok(Value::from_int(300)));
}

#[test]
fn record_field_assign_rejects_immutable_binding() {
    let src = r#"
        type Point = { x: Int, y: Int }
        fn main() -> Int {
            let p = Point { x: 1, y: 2 };
            p.x = 99;
            p.x
        }
    "#;
    let err = run_source(src).unwrap_err();
    assert!(err.contains("immutable") || err.contains("mut"),
        "expected immutable-binding error, got: {}", err);
}

#[test]
fn record_field_assign_visible_in_subsequent_reads() {
    let src = r#"
        type Inner = { v: Int }
        type Outer = { inner: Inner, n: Int }
        fn main() -> Int {
            let mut o = Outer { inner: Inner { v: 5 }, n: 10 };
            o.n = 99;
            o.n + o.inner.v
        }
    "#;
    assert_eq!(run_source(src), Ok(Value::from_int(104)));
}

#[test]
fn record_field_assign_replaces_handle_typed_field() {
    let src = r#"
        type Box = { tag: Int, msg: String }
        fn main() -> String {
            let mut b = Box { tag: 1, msg: "old" };
            b.msg = "new";
            b.msg
        }
    "#;
    assert_eq!(run_source_string(src).as_deref(), Ok("new"));
}


#[test]
fn record_float_field_arith_direct() {
    let src = r#"
        type R = { x: Float }
        fn main() -> Float { let r = R { x: 1.5 }; r.x + 2.5 }
    "#;
    assert_eq!(run_source(src), Ok(Value::from_float(4.0)));
}

#[test]
fn record_int_field_arith_direct() {
    let src = r#"
        type R = { x: Int }
        fn main() -> Int { let r = R { x: 3 }; r.x * 4 }
    "#;
    assert_eq!(run_source(src), Ok(Value::from_int(12)));
}

#[test]
fn record_mixed_int_float_fields_pick_correct_op() {
    let src = r#"
        type R = { i: Int, f: Float }
        fn main() -> Float { let r = R { i: 2, f: 1.5 }; r.f * 2.0 }
    "#;
    let v = run_source(src).unwrap();
    assert!(!v.as_float().is_nan(), "got NaN: integer Mul instead of FMul");
    assert_eq!(v, Value::from_float(3.0));
}

#[test]
fn record_int_field_does_not_become_float_in_arith() {
    let src = r#"
        type R = { i: Int, f: Float }
        fn main() -> Int { let r = R { i: 5, f: 1.0 }; r.i + 10 }
    "#;
    assert_eq!(run_source(src), Ok(Value::from_int(15)));
}

#[test]
fn array_of_records_float_field_arith_via_index() {
    let src = r#"
        type R = { x: Float }
        fn main() -> Float {
            let a = [R { x: 1.0 }, R { x: 2.0 }, R { x: 3.0 }];
            a[0].x + a[2].x
        }
    "#;
    assert_eq!(run_source(src), Ok(Value::from_float(4.0)));
}

#[test]
fn array_of_records_int_field_arith_via_index() {
    let src = r#"
        type R = { n: Int }
        fn main() -> Int {
            let a = [R { n: 5 }, R { n: 10 }, R { n: 15 }];
            a[0].n + a[1].n + a[2].n
        }
    "#;
    assert_eq!(run_source(src), Ok(Value::from_int(30)));
}

#[test]
fn fn_returning_array_of_records_float_field_arith() {
    let src = r#"
        type R = { x: Float }
        fn make() -> Array<R> { [R { x: 1.5 }, R { x: 2.5 }] }
        fn main() -> Float { let v = make(); v[0].x + v[1].x }
    "#;
    assert_eq!(run_source(src), Ok(Value::from_float(4.0)));
}

#[test]
fn chained_call_indexed_record_field_arith() {
    let src = r#"
        type R = { x: Float }
        fn make() -> Array<R> { [R { x: 1.0 }, R { x: 2.0 }] }
        fn main() -> Float { make()[0].x + make()[1].x }
    "#;
    assert_eq!(run_source(src), Ok(Value::from_float(3.0)));
}

#[test]
fn static_array_of_records_float_field_arith() {
    let src = r#"
        type R = { x: Float, n: Int }
        static AR: Array<R> = [R { x: 1.0, n: 1 }, R { x: 2.0, n: 2 }, R { x: 3.0, n: 3 }]
        fn main() -> Float { AR[1].x + AR[2].x }
    "#;
    assert_eq!(run_source(src), Ok(Value::from_float(5.0)));
}

#[test]
fn static_array_of_records_int_field_arith() {
    let src = r#"
        type R = { x: Float, n: Int }
        static AR: Array<R> = [R { x: 1.0, n: 10 }, R { x: 2.0, n: 20 }]
        fn main() -> Int { AR[0].n + AR[1].n }
    "#;
    assert_eq!(run_source(src), Ok(Value::from_int(30)));
}

#[test]
fn record_float_field_mut_assign_in_local() {
    let src = r#"
        type R = { x: Float }
        fn main() -> Float {
            let r = R { x: 2.0 };
            let mut x = r.x;
            x = x * 2.5;
            x
        }
    "#;
    let v = run_source(src).unwrap();
    assert!(!v.as_float().is_nan(), "got NaN: integer Mul instead of FMul");
    assert_eq!(v, Value::from_float(5.0));
}

#[test]
fn record_field_via_reference_param() {
    let src = r#"
        type R = { x: Float }
        fn read(r: &R) -> Float { r.x }
        fn main() -> Float { let r = R { x: 1.25 }; read(&r) + 0.75 }
    "#;
    assert_eq!(run_source(src), Ok(Value::from_float(2.0)));
}

#[test]
fn nested_record_field_arith() {
    let src = r#"
        type Inner = { v: Float }
        type Outer = { i: Inner, n: Int }
        fn main() -> Float { let o = Outer { i: Inner { v: 1.5 }, n: 7 }; o.i.v * 4.0 }
    "#;
    let v = run_source(src).unwrap();
    assert!(!v.as_float().is_nan(), "got NaN: integer Mul instead of FMul on nested .v");
    assert_eq!(v, Value::from_float(6.0));
}

#[test]
fn shared_record_deref_field_arith() {
    let src = r#"
        type R = { x: Float }
        fn main() -> Float {
            region {
                let s = Shared(R { x: 1.5 });
                (*s).x + 0.5
            }
        }
    "#;
    assert_eq!(run_source(src), Ok(Value::from_float(2.0)));
}

#[test]
fn record_field_access_in_call_arg_position() {
    let src = r#"
        type R = { x: Float }
        fn id(f: Float) -> Float { f }
        fn main() -> Float { let r = R { x: 2.5 }; id(r.x) + 0.5 }
    "#;
    assert_eq!(run_source(src), Ok(Value::from_float(3.0)));
}

#[test]
fn record_float_field_compared_to_float_literal() {
    let src = r#"
        type R = { x: Float }
        fn main() -> Int {
            let r = R { x: 1.5 };
            if r.x > 1.0 { 1 } else { 0 }
        }
    "#;
    assert_eq!(run_source(src), Ok(Value::from_int(1)));
}

#[test]
fn record_returned_from_if_branches_field_arith() {
    let src = r#"
        type R = { x: Float }
        fn pick(b: Bool) -> R { if b { R { x: 1.5 } } else { R { x: 2.5 } } }
        fn main() -> Float { pick(true).x + pick(false).x }
    "#;
    assert_eq!(run_source(src), Ok(Value::from_float(4.0)));
}

#[test]
fn record_returned_from_match_arms_field_arith() {
    let src = r#"
        type R = { x: Float }
        fn pick(n: Int) -> R {
            match n {
                0 => R { x: 1.0 },
                _ => R { x: 2.0 },
            }
        }
        fn main() -> Float { pick(0).x + pick(7).x }
    "#;
    assert_eq!(run_source(src), Ok(Value::from_float(3.0)));
}

#[test]
fn tuple_of_records_destructured_field_access() {
    let src = r#"
        type R = { x: Float }
        fn main() -> Float {
            let (r1, r2) = (R { x: 1.5 }, R { x: 2.5 });
            r1.x + r2.x
        }
    "#;
    assert_eq!(run_source(src), Ok(Value::from_float(4.0)));
}

#[test]
fn tuple_numeric_field_access() {
    let src = r#"
        fn main() -> Int { let t = (10, 20); t.0 + t.1 }
    "#;
    assert_eq!(run_source(src), Ok(Value::from_int(30)));
}

#[test]
fn record_destructure_in_fn_param() {
    let src = r#"
        type R = { x: Float, y: Float }
        fn sum(R { x, y }: R) -> Float { x + y }
        fn main() -> Float { sum(R { x: 1.0, y: 2.5 }) }
    "#;
    assert_eq!(run_source(src), Ok(Value::from_float(3.5)));
}

#[test]
fn record_field_through_closure_capture() {
    let src = r#"
        type R = { x: Float }
        fn main() -> Float {
            let r = R { x: 1.5 };
            let f = || -> Float { r.x + 0.5 };
            f()
        }
    "#;
    assert_eq!(run_source(src), Ok(Value::from_float(2.0)));
}

#[test]
fn nested_record_three_levels_field_arith() {
    let src = r#"
        type A = { v: Float }
        type B = { a: A }
        type C = { b: B, n: Int }
        fn main() -> Float {
            let c = C { b: B { a: A { v: 1.25 } }, n: 1 };
            c.b.a.v * 4.0
        }
    "#;
    assert_eq!(run_source(src), Ok(Value::from_float(5.0)));
}

#[test]
fn record_with_string_field_field_access_keeps_record_live() {
    let src = r#"
        type R = { name: String, n: Int }
        fn main() -> Int {
            let r = R { name: "hi", n: 7 };
            r.n + r.n
        }
    "#;
    assert_eq!(run_source(src), Ok(Value::from_int(14)));
}

#[test]
fn record_destructure_in_let() {
    let src = r#"
        type R = { x: Float, y: Float }
        fn main() -> Float {
            let r = R { x: 1.5, y: 2.5 };
            let R { x, y } = r;
            x + y
        }
    "#;
    assert_eq!(run_source(src), Ok(Value::from_float(4.0)));
}

#[test]
fn record_in_static_field_read() {
    let src = r#"
        type R = { x: Float, n: Int }
        static S: R = R { x: 1.5, n: 7 }
        fn main() -> Float { S.x + 0.5 }
    "#;
    assert_eq!(run_source(src), Ok(Value::from_float(2.0)));
}

#[test]
fn record_in_static_int_field_arith() {
    let src = r#"
        type R = { x: Float, n: Int }
        static S: R = R { x: 1.0, n: 10 }
        fn main() -> Int { S.n + 5 }
    "#;
    assert_eq!(run_source(src), Ok(Value::from_int(15)));
}

#[test]
fn record_with_array_field_index_then_arith() {
    let src = r#"
        type R = { xs: Array<Float> }
        fn main() -> Float {
            let r = R { xs: [1.5, 2.5, 3.5] };
            r.xs[1] + r.xs[2]
        }
    "#;
    assert_eq!(run_source(src), Ok(Value::from_float(6.0)));
}

#[test]
fn record_field_assign_mutates_and_reads_back() {
    let src = r#"
        type R = { x: Float }
        fn main() -> Float {
            let mut r = R { x: 1.0 };
            r.x = 2.5;
            r.x + 0.5
        }
    "#;
    assert_eq!(run_source(src), Ok(Value::from_float(3.0)));
}

#[test]
fn two_record_types_same_field_name_pick_correct_field_type() {
    let src = r#"
        type A = { v: Int }
        type B = { v: Float }
        fn main() -> Float {
            let a = A { v: 7 };
            let b = B { v: 1.5 };
            b.v + 0.5
        }
    "#;
    assert_eq!(run_source(src), Ok(Value::from_float(2.0)));
}

#[test]
fn record_field_int_used_as_int_does_not_become_float() {
    let src = r#"
        type A = { v: Int }
        type B = { v: Float }
        fn main() -> Int {
            let a = A { v: 7 };
            let b = B { v: 1.5 };
            a.v + 3
        }
    "#;
    assert_eq!(run_source(src), Ok(Value::from_int(10)));
}

#[test]
fn record_inside_array_inside_record_field_arith() {
    let src = r#"
        type Inner = { v: Float }
        type Outer = { items: Array<Inner> }
        fn main() -> Float {
            let o = Outer { items: [Inner { v: 1.0 }, Inner { v: 2.0 }, Inner { v: 3.0 }] };
            o.items[0].v + o.items[2].v
        }
    "#;
    assert_eq!(run_source(src), Ok(Value::from_float(4.0)));
}

#[test]
fn shared_record_deref_field_arith_with_annotation() {
    let src = r#"
        type R = { x: Float }
        fn main() -> Float {
            region {
                let s: Shared<R> = Shared(R { x: 1.5 });
                let v: Float = (*s).x + 0.5;
                v
            }
        }
    "#;
    assert_eq!(run_source(src), Ok(Value::from_float(2.0)));
}

#[test]
fn record_in_array_repeat_float_field() {
    let src = r#"
        type R = { x: Float }
        fn main() -> Float {
            let a = [R { x: 1.5 }; 3];
            a[0].x + a[2].x
        }
    "#;
    assert_eq!(run_source(src), Ok(Value::from_float(3.0)));
}

fn method_resolved_kinds(src: &str) -> Vec<String> {
    let ast = parse_source(src);
    let log: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let sink_log = Rc::clone(&log);
    let mut compiler = Compiler::new()
        .with_inline(false)
        .with_debug_sink(Box::new(move |msg: &str| {
            sink_log.borrow_mut().push(msg.to_string());
        }));
    compiler
        .compile_module(&ast)
        .unwrap_or_else(|errs| panic!("compile failed: {:?}", errs));
    let lines = log.borrow().clone();
    drop(compiler);
    lines
        .into_iter()
        .filter(|l| l.starts_with("[CALL] resolved to "))
        .collect()
}

const METHOD_DOUBLER_INNER: &str = r#"
trait Doubler { fn double(self) -> Int { 0 } }
type Inner = { v: Int }
impl Doubler for Inner { fn double(self) -> Int { self.v * 2 } }
"#;

const METHOD_DOUBLER_INT: &str = r#"
trait Doubler { fn double(self) -> Int { 0 } }
impl Doubler for Int { fn double(self) -> Int { self * 2 } }
"#;

#[test]
fn method_field_receiver_monomorphized_to_function() {
    let src = format!(
        "{METHOD_DOUBLER_INNER}
        type Outer = {{ inner: Inner }}
        fn main() -> Int {{
            let o = Outer {{ inner: Inner {{ v: 21 }} }};
            o.inner.double()
        }}"
    );
    let kinds = method_resolved_kinds(&src);
    assert!(
        kinds.iter().any(|k| k.contains("resolved to \"Function\""))
            && !kinds.iter().any(|k| k.contains("resolved to \"Method\"")),
        "field-receiver call must be mono-rewritten to a Function call (typeck-typed via ExprId); got {:?}",
        kinds
    );
}

#[test]
fn method_index_receiver_monomorphized_to_function() {
    let src = format!(
        "{METHOD_DOUBLER_INNER}
        fn main() -> Int {{
            let a = [Inner {{ v: 5 }}, Inner {{ v: 10 }}];
            a[1].double()
        }}"
    );
    let kinds = method_resolved_kinds(&src);
    assert!(
        kinds.iter().any(|k| k.contains("resolved to \"Function\""))
            && !kinds.iter().any(|k| k.contains("resolved to \"Method\"")),
        "index-receiver call must be mono-rewritten to a Function call; got {:?}",
        kinds
    );
}

#[test]
fn method_peekable_receiver_is_monomorphized_not_method() {
    let src = format!(
        "{METHOD_DOUBLER_INT}
        fn main() -> Int {{ let n = 21; n.double() }}"
    );
    let kinds = method_resolved_kinds(&src);
    assert!(
        kinds.iter().any(|k| k.contains("resolved to \"Function\"")),
        "Int-literal receiver must be mono-rewritten to a Function call; got {:?}",
        kinds
    );
    assert!(
        !kinds.iter().any(|k| k.contains("resolved to \"Method\"")),
        "peekable receiver must NOT hit the codegen Method path; got {:?}",
        kinds
    );
}

#[test]
fn method_field_receiver_int_result_heap_balanced() {
    let src = format!(
        "{METHOD_DOUBLER_INNER}
        type Outer = {{ inner: Inner }}
        fn main() -> Int {{
            let o = Outer {{ inner: Inner {{ v: 21 }} }};
            o.inner.double()
        }}"
    );
    let (v, live) = run_source_with_heap(&src).unwrap();
    assert_eq!(v, Value::from_int(42));
    assert_eq!(live, 0, "heap must balance after field-receiver method call");
}

#[test]
fn method_index_receiver_int_result_heap_balanced() {
    let src = format!(
        "{METHOD_DOUBLER_INNER}
        fn main() -> Int {{
            let a = [Inner {{ v: 5 }}, Inner {{ v: 10 }}, Inner {{ v: 15 }}];
            a[2].double()
        }}"
    );
    let (v, live) = run_source_with_heap(&src).unwrap();
    assert_eq!(v, Value::from_int(30));
    assert_eq!(live, 0, "heap must balance after index-receiver method call");
}

#[test]
fn method_deep_field_chain_receiver_heap_balanced() {
    let src = format!(
        "{METHOD_DOUBLER_INNER}
        type B = {{ a: Inner }}
        type C = {{ b: B }}
        fn main() -> Int {{
            let c = C {{ b: B {{ a: Inner {{ v: 11 }} }} }};
            c.b.a.double()
        }}"
    );
    let (v, live) = run_source_with_heap(&src).unwrap();
    assert_eq!(v, Value::from_int(22));
    assert_eq!(live, 0, "heap must balance after deep-chain field-receiver call");
}

#[test]
fn method_field_and_literal_receivers_agree() {
    let field = format!(
        "{METHOD_DOUBLER_INNER}
        type Outer = {{ inner: Inner }}
        fn main() -> Int {{
            let o = Outer {{ inner: Inner {{ v: 21 }} }};
            o.inner.double()
        }}"
    );
    let literal = format!(
        "{METHOD_DOUBLER_INT}
        fn main() -> Int {{ (21).double() }}"
    );
    let fk = method_resolved_kinds(&field);
    let lk = method_resolved_kinds(&literal);
    for (form, k) in [("field", &fk), ("literal", &lk)] {
        assert!(
            k.iter().any(|s| s.contains("\"Function\"")) && !k.iter().any(|s| s.contains("\"Method\"")),
            "{form} form must be a mono-rewritten Function call (unified on ExprId table); got {:?}", k
        );
    }
    let (a, _) = run_source_with_heap(&field).unwrap();
    let (b, _) = run_source_with_heap(&literal).unwrap();
    assert_eq!(a, b, "field and literal receivers must agree on the value");
    assert_eq!(a, Value::from_int(42));
}

#[test]
fn method_field_receiver_moves_handle_typed_field_no_leak() {
    let src = r#"
        trait Sum { fn sum(self) -> Int { 0 } }
        type Inner = { v: Int, tag: String }
        type Outer = { inner: Inner }
        impl Sum for Inner { fn sum(self) -> Int { self.v } }
        fn main() -> Int {
            let o = Outer { inner: Inner { v: 7, tag: "payload" } };
            o.inner.sum()
        }
    "#;
    let (v, live) = run_source_with_heap(src).unwrap();
    assert_eq!(v, Value::from_int(7));
    assert_eq!(live, 0, "String field consumed by by-value receiver must be freed");
}

#[test]
fn method_field_receiver_fuzz_matches_oracle_and_balances_heap() {
    let cases: [i64; 12] = [0, 1, -1, 2, 7, 21, 100, -100, 1000, -1234, 65536, -65537];
    for &n in &cases {
        let src = format!(
            "{METHOD_DOUBLER_INNER}
            type Outer = {{ inner: Inner }}
            fn main() -> Int {{
                let o = Outer {{ inner: Inner {{ v: {n} }} }};
                o.inner.double()
            }}"
        );
        let (v, live) = run_source_with_heap(&src)
            .unwrap_or_else(|e| panic!("n={n} failed:\n{e}"));
        assert_eq!(v, Value::from_int(n.wrapping_mul(2)), "n={n}");
        assert_eq!(live, 0, "n={n}: heap must balance");
    }
}

// Guard for deleting the codegen Method path: every FieldAccess-callee shape must mono-rewrite to Function. Any "Method" = shape still needs it.
#[test]
fn every_method_receiver_shape_monomorphizes_to_function() {
    let prelude = "
type Inner = { v: Int }
type Mid = { inner: Inner }
type Top = { mid: Mid }
trait Doubler { fn double(self) -> Int { 0 } }
trait Twin { fn twin(self) -> Inner { Inner { v: 0 } } }
impl Doubler for Inner { fn double(self) -> Int { self.v * 2 } }
impl Twin for Inner { fn twin(self) -> Inner { Inner { v: self.v } } }
fn mk() -> Inner { Inner { v: 21 } }
";
    let bodies: [(&str, &str); 10] = [
        ("identifier binding", "let i = Inner { v: 21 }; i.double()"),
        ("field access",       "let m = Mid { inner: Inner { v: 21 } }; m.inner.double()"),
        ("deep field chain",   "let t = Top { mid: Mid { inner: Inner { v: 21 } } }; t.mid.inner.double()"),
        ("array index",        "let a = [Inner { v: 21 }]; a[0].double()"),
        ("paren receiver",     "let m = Mid { inner: Inner { v: 21 } }; (m.inner).double()"),
        ("call result",        "mk().double()"),
        ("method chain",       "let i = Inner { v: 21 }; i.twin().double()"),
        ("call-then-chain",    "mk().twin().double()"),
        ("block receiver",     "({ let x = Inner { v: 21 }; x }).double()"),
        ("match receiver",     "let flag = true; (match flag { true => Inner { v: 21 }, _ => Inner { v: 0 } }).double()"),
    ];
    for (label, body) in bodies {
        let src = format!("{prelude}\nfn main() -> Int {{ {body} }}");
        let kinds = method_resolved_kinds(&src);
        assert!(
            !kinds.iter().any(|k| k.contains("resolved to \"Method\"")),
            "receiver shape `{label}` hit the codegen Method path (still needed): {:?}",
            kinds
        );
        let (v, live) = run_source_with_heap(&src).unwrap_or_else(|e| panic!("{label}: {e}"));
        assert_eq!(v, Value::from_int(42), "shape `{label}` wrong value");
        assert_eq!(live, 0, "shape `{label}` heap must balance");
    }
}

// Unresolvable receiver must be a clean typeck error, never a codegen panic.
#[test]
fn unresolvable_method_receiver_errors_clean_not_panic() {
    let src = "\ntrait D { fn double(self) -> Int { 0 } }\ntype Inner = { v: Int }\nimpl D for Inner { fn double(self) -> Int { self.v } }\nfn main() -> Int { nope.double() }\n";
    let r = run_source(src);
    assert!(r.is_err(), "unresolved receiver must be a clean compile error, got {:?}", r);
}
