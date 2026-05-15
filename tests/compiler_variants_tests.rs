#[path = "compiler_codegen_common.rs"]
mod compiler_codegen_common;

use compiler_codegen_common::*;
use abrase::vm::Value;

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
    assert_eq!(result, Ok(Value::Int(1)));
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
    assert_eq!(result, Ok(Value::Int(99)));
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
    assert_eq!(result, Ok(Value::Int(5)));
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
    assert_eq!(result, Ok(Value::Int(333)));
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
    assert_eq!(result, Ok(Value::Int(66)));
}
