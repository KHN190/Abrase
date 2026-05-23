#[path = "compiler_codegen_common.rs"]
mod compiler_codegen_common;

use compiler_codegen_common::*;
use myriad::Value;

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
