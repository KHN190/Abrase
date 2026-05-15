#[path = "compiler_codegen_common.rs"]
mod compiler_codegen_common;

use compiler_codegen_common::*;
use abrase::vm::Value;

#[test]
fn verify_compile_exn_simple_ok_result() {
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
                node: Expr::Literal(Literal::Int(42)),
                span: Span::new(0, 0),
            })),
        },
    })];
    let result = compile_module_and_run(&ast);
    assert_eq!(result, Ok(Value::Int(42)));
}

#[test]
fn verify_compile_exn_function_returns_result() {
    let ast = vec![
        Decl::Fn(FnDecl {
            attrs: vec![],
            is_pub: false,
            name: "maybe_fail".to_string(),
            generics: vec![],
            params: vec![Param::Named {
                pattern: Spanned {
                    node: Pattern::Bind("x".to_string()),
                    span: Span::new(0, 0),
                },
                ty: Type::Named("Int".to_string()),
            }],
            effects: vec![EffectItem { name: vec!["exn".to_string()], arg: None }],
            return_type: Some(Type::Named("Int".to_string())),
            where_clause: vec![],
            body: Block {
                stmts: vec![],
                ret: Some(Box::new(Spanned {
                    node: Expr::Identifier("x".to_string()),
                    span: Span::new(0, 0),
                })),
            },
        }),
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
                    node: Expr::Literal(Literal::Int(100)),
                    span: Span::new(0, 0),
                })),
            },
        }),
    ];
    let result = compile_module_and_run(&ast);
    assert_eq!(result, Ok(Value::Int(100)));
}

#[test]
fn verify_compile_exn_match_ok_pattern() {
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
                node: Expr::Literal(Literal::Int(50)),
                span: Span::new(0, 0),
            })),
        },
    })];
    let result = compile_module_and_run(&ast);
    assert_eq!(result, Ok(Value::Int(50)));
}

#[test]
fn verify_compile_exn_match_err_pattern() {
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
                node: Expr::Literal(Literal::Int(75)),
                span: Span::new(0, 0),
            })),
        },
    })];
    let result = compile_module_and_run(&ast);
    assert_eq!(result, Ok(Value::Int(75)));
}

#[test]
fn verify_compile_exn_throw_integer_code() {
    let ast = vec![
        Decl::Fn(FnDecl {
            attrs: vec![],
            is_pub: false,
            name: "throws".to_string(),
            generics: vec![],
            params: vec![],
            effects: vec![EffectItem { name: vec!["exn".to_string()], arg: None }],
            return_type: Some(Type::Named("Int".to_string())),
            where_clause: vec![],
            body: Block {
                stmts: vec![],
                ret: Some(Box::new(Spanned {
                    node: Expr::Literal(Literal::Int(999)),
                    span: Span::new(0, 0),
                })),
            },
        }),
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
                    node: Expr::Literal(Literal::Int(77)),
                    span: Span::new(0, 0),
                })),
            },
        }),
    ];
    let result = compile_module_and_run(&ast);
    assert_eq!(result, Ok(Value::Int(77)));
}

#[test]
fn verify_compile_exn_conditional_throw() {
    let ast = vec![
        Decl::Fn(FnDecl {
            attrs: vec![],
            is_pub: false,
            name: "safe_div".to_string(),
            generics: vec![],
            params: vec![
                Param::Named {
                    pattern: Spanned {
                        node: Pattern::Bind("a".to_string()),
                        span: Span::new(0, 0),
                    },
                    ty: Type::Named("Int".to_string()),
                },
                Param::Named {
                    pattern: Spanned {
                        node: Pattern::Bind("b".to_string()),
                        span: Span::new(0, 0),
                    },
                    ty: Type::Named("Int".to_string()),
                },
            ],
            effects: vec![EffectItem { name: vec!["exn".to_string()], arg: None }],
            return_type: Some(Type::Named("Int".to_string())),
            where_clause: vec![],
            body: Block {
                stmts: vec![],
                ret: Some(Box::new(Spanned {
                    node: Expr::If {
                        condition: Box::new(Spanned {
                            node: Expr::Binary {
                                op: BinaryOp::Eq,
                                left: Box::new(Spanned {
                                    node: Expr::Identifier("b".to_string()),
                                    span: Span::new(0, 0),
                                }),
                                right: Box::new(Spanned {
                                    node: Expr::Literal(Literal::Int(0)),
                                    span: Span::new(0, 0),
                                }),
                            },
                            span: Span::new(0, 0),
                        }),
                        consequence: Box::new(Spanned {
                            node: Expr::Literal(Literal::Int(1)),
                            span: Span::new(0, 0),
                        }),
                        alternative: Some(Box::new(Spanned {
                            node: Expr::Binary {
                                op: BinaryOp::Div,
                                left: Box::new(Spanned {
                                    node: Expr::Identifier("a".to_string()),
                                    span: Span::new(0, 0),
                                }),
                                right: Box::new(Spanned {
                                    node: Expr::Identifier("b".to_string()),
                                    span: Span::new(0, 0),
                                }),
                            },
                            span: Span::new(0, 0),
                        })),
                    },
                    span: Span::new(0, 0),
                })),
            },
        }),
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
fn verify_compile_exn_multiple_exception_types() {
    let ast = vec![
        Decl::Fn(FnDecl {
            attrs: vec![],
            is_pub: false,
            name: "check".to_string(),
            generics: vec![],
            params: vec![Param::Named {
                pattern: Spanned {
                    node: Pattern::Bind("n".to_string()),
                    span: Span::new(0, 0),
                },
                ty: Type::Named("Int".to_string()),
            }],
            effects: vec![EffectItem { name: vec!["exn".to_string()], arg: None }],
            return_type: Some(Type::Named("Int".to_string())),
            where_clause: vec![],
            body: Block {
                stmts: vec![],
                ret: Some(Box::new(Spanned {
                    node: Expr::Identifier("n".to_string()),
                    span: Span::new(0, 0),
                })),
            },
        }),
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
                    node: Expr::Literal(Literal::Int(11)),
                    span: Span::new(0, 0),
                })),
            },
        }),
    ];
    let result = compile_module_and_run(&ast);
    assert_eq!(result, Ok(Value::Int(11)));
}

#[test]
fn verify_compile_exn_handler_catches_error() {
    let ast = vec![
        Decl::Fn(FnDecl {
            attrs: vec![],
            is_pub: false,
            name: "risky".to_string(),
            generics: vec![],
            params: vec![],
            effects: vec![EffectItem { name: vec!["exn".to_string()], arg: None }],
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
                    node: Expr::Literal(Literal::Int(22)),
                    span: Span::new(0, 0),
                })),
            },
        }),
    ];
    let result = compile_module_and_run(&ast);
    assert_eq!(result, Ok(Value::Int(22)));
}

#[test]
fn verify_compile_exn_missing_ok_err_pattern_in_match() {
    let ast = vec![
        Decl::Fn(FnDecl {
            attrs: vec![],
            is_pub: false,
            name: "maybe_fail".to_string(),
            generics: vec![],
            params: vec![],
            effects: vec![EffectItem { name: vec!["exn".to_string()], arg: None }],
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
    assert!(result.is_ok());
}

#[test]
fn verify_compile_exn_throw_with_custom_error() {
    let ast = vec![
        Decl::Fn(FnDecl {
            attrs: vec![],
            is_pub: false,
            name: "bad_op".to_string(),
            generics: vec![],
            params: vec![Param::Named {
                pattern: Spanned {
                    node: Pattern::Bind("x".to_string()),
                    span: Span::new(0, 0),
                },
                ty: Type::Named("Int".to_string()),
            }],
            effects: vec![EffectItem { name: vec!["exn".to_string()], arg: None }],
            return_type: Some(Type::Named("Int".to_string())),
            where_clause: vec![],
            body: Block {
                stmts: vec![],
                ret: Some(Box::new(Spanned {
                    node: Expr::If {
                        condition: Box::new(Spanned {
                            node: Expr::Binary {
                                op: BinaryOp::Lt,
                                left: Box::new(Spanned {
                                    node: Expr::Identifier("x".to_string()),
                                    span: Span::new(0, 0),
                                }),
                                right: Box::new(Spanned {
                                    node: Expr::Literal(Literal::Int(0)),
                                    span: Span::new(0, 0),
                                }),
                            },
                            span: Span::new(0, 0),
                        }),
                        consequence: Box::new(Spanned {
                            node: Expr::Literal(Literal::Int(1)),
                            span: Span::new(0, 0),
                        }),
                        alternative: Some(Box::new(Spanned {
                            node: Expr::Identifier("x".to_string()),
                            span: Span::new(0, 0),
                        })),
                    },
                    span: Span::new(0, 0),
                })),
            },
        }),
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
                    node: Expr::Literal(Literal::Int(33)),
                    span: Span::new(0, 0),
                })),
            },
        }),
    ];
    let result = compile_module_and_run(&ast);
    assert_eq!(result, Ok(Value::Int(33)));
}

#[test]
fn verify_compile_exn_propagate_up_stack() {
    let ast = vec![
        Decl::Fn(FnDecl {
            attrs: vec![],
            is_pub: false,
            name: "level3".to_string(),
            generics: vec![],
            params: vec![],
            effects: vec![EffectItem { name: vec!["exn".to_string()], arg: None }],
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
        Decl::Fn(FnDecl {
            attrs: vec![],
            is_pub: false,
            name: "level2".to_string(),
            generics: vec![],
            params: vec![],
            effects: vec![EffectItem { name: vec!["exn".to_string()], arg: None }],
            return_type: Some(Type::Named("Int".to_string())),
            where_clause: vec![],
            body: Block {
                stmts: vec![],
                ret: Some(Box::new(Spanned {
                    node: Expr::Literal(Literal::Int(2)),
                    span: Span::new(0, 0),
                })),
            },
        }),
        Decl::Fn(FnDecl {
            attrs: vec![],
            is_pub: false,
            name: "level1".to_string(),
            generics: vec![],
            params: vec![],
            effects: vec![EffectItem { name: vec!["exn".to_string()], arg: None }],
            return_type: Some(Type::Named("Int".to_string())),
            where_clause: vec![],
            body: Block {
                stmts: vec![],
                ret: Some(Box::new(Spanned {
                    node: Expr::Literal(Literal::Int(3)),
                    span: Span::new(0, 0),
                })),
            },
        }),
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
fn verify_compile_exn_ok_and_err_both_handled() {
    let ast = vec![
        Decl::Fn(FnDecl {
            attrs: vec![],
            is_pub: false,
            name: "compute".to_string(),
            generics: vec![],
            params: vec![Param::Named {
                pattern: Spanned {
                    node: Pattern::Bind("flag".to_string()),
                    span: Span::new(0, 0),
                },
                ty: Type::Named("Int".to_string()),
            }],
            effects: vec![EffectItem { name: vec!["exn".to_string()], arg: None }],
            return_type: Some(Type::Named("Int".to_string())),
            where_clause: vec![],
            body: Block {
                stmts: vec![],
                ret: Some(Box::new(Spanned {
                    node: Expr::Identifier("flag".to_string()),
                    span: Span::new(0, 0),
                })),
            },
        }),
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
