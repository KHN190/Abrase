#[path = "compiler_codegen_common.rs"]
mod compiler_codegen_common;

use compiler_codegen_common::*;
use abrase::vm::Value;

#[test]
fn verify_compile_array_literal_construction() {
    let ast = vec![Decl::Fn(FnDecl {
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
                node: Expr::Literal(Literal::Int(3)),
                span: Span::new(0, 0),
            })),
        },
    })];
    let result = compile_module_and_run(&ast);
    assert_eq!(result, Ok(Value::Int(3)));
}

#[test]
fn verify_compile_array_repeat_construction() {
    let ast = vec![Decl::Fn(FnDecl {
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
                node: Expr::Literal(Literal::Int(10)),
                span: Span::new(0, 0),
            })),
        },
    })];
    let result = compile_module_and_run(&ast);
    assert_eq!(result, Ok(Value::Int(10)));
}

#[test]
fn verify_compile_array_indexing_constant() {
    let ast = vec![Decl::Fn(FnDecl {
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
                node: Expr::Literal(Literal::Int(20)),
                span: Span::new(0, 0),
            })),
        },
    })];
    let result = compile_module_and_run(&ast);
    assert_eq!(result, Ok(Value::Int(20)));
}

#[test]
fn verify_compile_nested_record_in_array() {
    let ast = vec![
        Decl::Type {
            name: "Pos".to_string(),
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
                    node: Expr::Literal(Literal::Int(44)),
                    span: Span::new(0, 0),
                })),
            },
        }),
    ];
    let result = compile_module_and_run(&ast);
    assert_eq!(result, Ok(Value::Int(44)));
}

#[test]
fn verify_compile_array_of_records_type() {
    let ast = vec![
        Decl::Type {
            name: "Elem".to_string(),
            generics: vec![],
            attrs: vec![],
            is_pub: false,
            ownership: None,
            body: TypeBody::Record(vec![
                RecordField {
                    is_pub: true,
                    name: "val".to_string(),
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
                    node: Expr::Literal(Literal::Int(88)),
                    span: Span::new(0, 0),
                })),
            },
        }),
    ];
    let result = compile_module_and_run(&ast);
    assert_eq!(result, Ok(Value::Int(88)));
}

#[test]
fn verify_compile_array_of_variants_type() {
    let ast = vec![
        Decl::Type {
            name: "Val".to_string(),
            generics: vec![],
            attrs: vec![],
            is_pub: false,
            ownership: None,
            body: TypeBody::Variant(vec![
                VariantCase::Unit("A".to_string()),
                VariantCase::Unit("B".to_string()),
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
                    node: Expr::Literal(Literal::Int(55)),
                    span: Span::new(0, 0),
                })),
            },
        }),
    ];
    let result = compile_module_and_run(&ast);
    assert_eq!(result, Ok(Value::Int(55)));
}
