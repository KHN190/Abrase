#[path = "compiler_codegen_common.rs"]
mod compiler_codegen_common;

use compiler_codegen_common::*;
use myriad::Value;

#[test]
fn verify_compile_variant_unit_construction() {
    let ast = vec![
        Decl::Type {
            name: "Status".to_string(),
            generics: vec![],
            attrs: vec![],
            is_pub: false,
            ownership: None,
            body: TypeBody::Variant(vec![
                VariantCase::Unit("Ok".to_string()),
                VariantCase::Unit("Error".to_string()),
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
                    node: Expr::Literal(Literal::Int(1)),
                    span: Span::new(0, 0),
                })),
            },
        }),
    ];
    let result = compile_module_and_run(&ast);
    assert_eq!(result, Ok(Value::from_int(1)));
}

#[test]
fn verify_compile_variant_tuple_construction() {
    let ast = vec![
        Decl::Type {
            name: "Result".to_string(),
            generics: vec![],
            attrs: vec![],
            is_pub: false,
            ownership: None,
            body: TypeBody::Variant(vec![
                VariantCase::Tuple("Some".to_string(), vec![Type::Named("Int".to_string())]),
                VariantCase::Unit("None".to_string()),
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
                    node: Expr::Literal(Literal::Int(99)),
                    span: Span::new(0, 0),
                })),
            },
        }),
    ];
    let result = compile_module_and_run(&ast);
    assert_eq!(result, Ok(Value::from_int(99)));
}

#[test]
fn verify_compile_variant_pattern_match_unit() {
    let ast = vec![
        Decl::Type {
            name: "Bool".to_string(),
            generics: vec![],
            attrs: vec![],
            is_pub: false,
            ownership: None,
            body: TypeBody::Variant(vec![
                VariantCase::Unit("True".to_string()),
                VariantCase::Unit("False".to_string()),
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
                    node: Expr::Literal(Literal::Int(5)),
                    span: Span::new(0, 0),
                })),
            },
        }),
    ];
    let result = compile_module_and_run(&ast);
    assert_eq!(result, Ok(Value::from_int(5)));
}

#[test]
fn verify_compile_variant_with_multiple_fields() {
    let ast = vec![
        Decl::Type {
            name: "Triple".to_string(),
            generics: vec![],
            attrs: vec![],
            is_pub: false,
            ownership: None,
            body: TypeBody::Variant(vec![
                VariantCase::Tuple(
                    "Triple".to_string(),
                    vec![
                        Type::Named("Int".to_string()),
                        Type::Named("Int".to_string()),
                        Type::Named("Int".to_string()),
                    ],
                ),
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
                    node: Expr::Literal(Literal::Int(333)),
                    span: Span::new(0, 0),
                })),
            },
        }),
    ];
    let result = compile_module_and_run(&ast);
    assert_eq!(result, Ok(Value::from_int(333)));
}

#[test]
fn verify_compile_variant_record_variant() {
    let ast = vec![
        Decl::Type {
            name: "Tagged".to_string(),
            generics: vec![],
            attrs: vec![],
            is_pub: false,
            ownership: None,
            body: TypeBody::Variant(vec![
                VariantCase::Record(
                    "Data".to_string(),
                    vec![
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
                    ],
                ),
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
                    node: Expr::Literal(Literal::Int(66)),
                    span: Span::new(0, 0),
                })),
            },
        }),
    ];
    let result = compile_module_and_run(&ast);
    assert_eq!(result, Ok(Value::from_int(66)));
}

const COLOR: &str = "type Color = Red | Green | Blue ";

#[test]
fn variant_eq_same_case_true() {
    let src = format!("{COLOR} fn main() -> Bool {{ let a = Red; let b = Red; a == b }}");
    assert_eq!(run_source(&src).expect("run"), Value::from_bool(true));
}

#[test]
fn variant_eq_different_case_false() {
    let src = format!("{COLOR} fn main() -> Bool {{ let a = Red; let b = Green; a == b }}");
    assert_eq!(run_source(&src).expect("run"), Value::from_bool(false));
}

#[test]
fn variant_neq_same_case_false() {
    let src = format!("{COLOR} fn main() -> Bool {{ let a = Blue; let b = Blue; a != b }}");
    assert_eq!(run_source(&src).expect("run"), Value::from_bool(false));
}

#[test]
fn variant_neq_different_case_true() {
    let src = format!("{COLOR} fn main() -> Bool {{ let a = Red; let b = Blue; a != b }}");
    assert_eq!(run_source(&src).expect("run"), Value::from_bool(true));
}

#[test]
fn variant_eq_inline_ctor_temporaries() {
    let src = format!("{COLOR} fn main() -> Bool {{ Green == Green }}");
    assert_eq!(run_source(&src).expect("run"), Value::from_bool(true));
}

#[test]
fn variant_eq_drives_if_logic() {
    let src = format!("{COLOR} fn main() -> Int {{ let x = Green; if x == Green {{ 1 }} else {{ 0 }} }}");
    assert_eq!(run_source(&src).expect("run"), Value::from_int(1));
}

#[test]
fn variant_eq_temporaries_no_leak() {
    let src = format!("{COLOR} fn main() -> Int {{ if Red == Blue {{ 1 }} else {{ 0 }} }}");
    let (v, live) = run_source_with_heap(&src).expect("run");
    assert_eq!(v, Value::from_int(0));
    assert_eq!(live, 0, "variant temporaries must be freed: {live} live");
}

#[test]
fn variant_eq_binding_no_leak() {
    let src = format!("{COLOR} fn main() -> Int {{ let a = Red; let b = Red; if a == b {{ 7 }} else {{ 0 }} }}");
    let (v, live) = run_source_with_heap(&src).expect("run");
    assert_eq!(v, Value::from_int(7));
    assert_eq!(live, 0, "bound variants must be freed: {live} live");
}

#[test]
fn variant_eq_operand_reused_after_compare() {
    let src = "type C = Red | Green fn f(c: C) -> Int { match c { Red => 0, Green => 1 } } fn main() -> Int { let c = Green; if c == Green { f(c) } else { 0 } }";
    let (v, live) = run_source_with_heap(src).expect("run");
    assert_eq!(v, Value::from_int(1), "== must borrow variant operand, allowing later use");
    assert_eq!(live, 0, "{live} live");
}
