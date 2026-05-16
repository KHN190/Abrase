#[path = "compiler_codegen_common.rs"]
mod compiler_codegen_common;

use compiler_codegen_common::*;
use abrase::vm::Value;

#[test]
fn verify_compile_if_true_branch() {
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
                node: Expr::If {
                    condition: Box::new(Spanned {
                        node: Expr::Literal(Literal::Bool(true)),
                        span: Span::new(0, 0),
                    }),
                    consequence: Box::new(Spanned {
                        node: Expr::Literal(Literal::Int(10)),
                        span: Span::new(0, 0),
                    }),
                    alternative: Some(Box::new(Spanned {
                        node: Expr::Literal(Literal::Int(20)),
                        span: Span::new(0, 0),
                    })),
                },
                span: Span::new(0, 0),
            })),
        },
    })];

    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::from_int(10));
}

#[test]
fn verify_compile_if_false_branch() {
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
                node: Expr::If {
                    condition: Box::new(Spanned {
                        node: Expr::Literal(Literal::Bool(false)),
                        span: Span::new(0, 0),
                    }),
                    consequence: Box::new(Spanned {
                        node: Expr::Literal(Literal::Int(10)),
                        span: Span::new(0, 0),
                    }),
                    alternative: Some(Box::new(Spanned {
                        node: Expr::Literal(Literal::Int(20)),
                        span: Span::new(0, 0),
                    })),
                },
                span: Span::new(0, 0),
            })),
        },
    })];

    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::from_int(20));
}

#[test]
fn verify_compile_if_with_comparison() {
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
                node: Expr::If {
                    condition: Box::new(Spanned {
                        node: Expr::Binary {
                            op: BinaryOp::Gt,
                            left: Box::new(Spanned {
                                node: Expr::Literal(Literal::Int(5)),
                                span: Span::new(0, 0),
                            }),
                            right: Box::new(Spanned {
                                node: Expr::Literal(Literal::Int(3)),
                                span: Span::new(0, 0),
                            }),
                        },
                        span: Span::new(0, 0),
                    }),
                    consequence: Box::new(Spanned {
                        node: Expr::Literal(Literal::Int(100)),
                        span: Span::new(0, 0),
                    }),
                    alternative: Some(Box::new(Spanned {
                        node: Expr::Literal(Literal::Int(200)),
                        span: Span::new(0, 0),
                    })),
                },
                span: Span::new(0, 0),
            })),
        },
    })];

    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::from_int(100));
}

#[test]
fn verify_compile_if_without_else_true() {
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
                node: Expr::If {
                    condition: Box::new(Spanned {
                        node: Expr::Literal(Literal::Bool(true)),
                        span: Span::new(0, 0),
                    }),
                    consequence: Box::new(Spanned {
                        node: Expr::Literal(Literal::Int(42)),
                        span: Span::new(0, 0),
                    }),
                    alternative: None,
                },
                span: Span::new(0, 0),
            })),
        },
    })];

    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::from_int(42));
}

#[test]
fn verify_compile_if_without_else_false() {
    let ast = vec![Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        name: "main".to_string(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(Type::Named("Unit".to_string())),
        where_clause: vec![],
        body: Block {
            stmts: vec![],
            ret: Some(Box::new(Spanned {
                node: Expr::If {
                    condition: Box::new(Spanned {
                        node: Expr::Literal(Literal::Bool(false)),
                        span: Span::new(0, 0),
                    }),
                    consequence: Box::new(Spanned {
                        node: Expr::Literal(Literal::Int(42)),
                        span: Span::new(0, 0),
                    }),
                    alternative: None,
                },
                span: Span::new(0, 0),
            })),
        },
    })];

    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::UNIT);
}

#[test]
fn verify_compile_while_loop_simple() {
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
            stmts: vec![
                Spanned {
                    node: Stmt::Let {
                        pattern: Spanned {
                            node: Pattern::Bind("count".to_string()),
                            span: Span::new(0, 0),
                        },
                        is_mut: true,
                        ty: Some(Type::Named("Int".to_string())),
                        value: Spanned {
                            node: Expr::Literal(Literal::Int(0)),
                            span: Span::new(0, 0),
                        },
                    },
                    span: Span::new(0, 0),
                },
            ],
            ret: Some(Box::new(Spanned {
                node: Expr::While {
                    condition: Box::new(Spanned {
                        node: Expr::Literal(Literal::Bool(false)),
                        span: Span::new(0, 0),
                    }),
                    body: Block {
                        stmts: vec![],
                        ret: None,
                    },
                },
                span: Span::new(0, 0),
            })),
        },
    })];

    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::UNIT);
}

#[test]
fn verify_compile_while_with_comparison() {
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
            stmts: vec![
                Spanned {
                    node: Stmt::Let {
                        pattern: Spanned {
                            node: Pattern::Bind("i".to_string()),
                            span: Span::new(0, 0),
                        },
                        is_mut: true,
                        ty: Some(Type::Named("Int".to_string())),
                        value: Spanned {
                            node: Expr::Literal(Literal::Int(0)),
                            span: Span::new(0, 0),
                        },
                    },
                    span: Span::new(0, 0),
                },
            ],
            ret: Some(Box::new(Spanned {
                node: Expr::While {
                    condition: Box::new(Spanned {
                        node: Expr::Binary {
                            op: BinaryOp::Lt,
                            left: Box::new(Spanned {
                                node: Expr::Literal(Literal::Int(1)),
                                span: Span::new(0, 0),
                            }),
                            right: Box::new(Spanned {
                                node: Expr::Literal(Literal::Int(0)),
                                span: Span::new(0, 0),
                            }),
                        },
                        span: Span::new(0, 0),
                    }),
                    body: Block {
                        stmts: vec![],
                        ret: None,
                    },
                },
                span: Span::new(0, 0),
            })),
        },
    })];

    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::UNIT);
}

#[test]
fn verify_compile_while_loop_with_mutation() {
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
            stmts: vec![
                Spanned {
                    node: Stmt::Let {
                        pattern: Spanned {
                            node: Pattern::Bind("i".to_string()),
                            span: Span::new(0, 0),
                        },
                        is_mut: true,
                        ty: Some(Type::Named("Int".to_string())),
                        value: Spanned {
                            node: Expr::Literal(Literal::Int(0)),
                            span: Span::new(0, 0),
                        },
                    },
                    span: Span::new(0, 0),
                },
                Spanned {
                    node: Stmt::Expr(Spanned {
                        node: Expr::While {
                            condition: Box::new(Spanned {
                                node: Expr::Binary {
                                    op: BinaryOp::Lt,
                                    left: Box::new(Spanned {
                                        node: Expr::Identifier("i".to_string()),
                                        span: Span::new(0, 0),
                                    }),
                                    right: Box::new(Spanned {
                                        node: Expr::Literal(Literal::Int(5)),
                                        span: Span::new(0, 0),
                                    }),
                                },
                                span: Span::new(0, 0),
                            }),
                            body: Block {
                                stmts: vec![
                                    Spanned {
                                        node: Stmt::Expr(Spanned {
                                            node: Expr::Binary {
                                                op: BinaryOp::Assign,
                                                left: Box::new(Spanned {
                                                    node: Expr::Identifier("i".to_string()),
                                                    span: Span::new(0, 0),
                                                }),
                                                right: Box::new(Spanned {
                                                    node: Expr::Binary {
                                                        op: BinaryOp::Add,
                                                        left: Box::new(Spanned {
                                                            node: Expr::Identifier("i".to_string()),
                                                            span: Span::new(0, 0),
                                                        }),
                                                        right: Box::new(Spanned {
                                                            node: Expr::Literal(Literal::Int(1)),
                                                            span: Span::new(0, 0),
                                                        }),
                                                    },
                                                    span: Span::new(0, 0),
                                                }),
                                            },
                                            span: Span::new(0, 0),
                                        }),
                                        span: Span::new(0, 0),
                                    },
                                ],
                                ret: None,
                            },
                        },
                        span: Span::new(0, 0),
                    }),
                    span: Span::new(0, 0),
                },
            ],
            ret: Some(Box::new(Spanned {
                node: Expr::Identifier("i".to_string()),
                span: Span::new(0, 0),
            })),
        },
    })];

    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::from_int(5));
}

#[test]
fn verify_compile_if_without_else_returns_unit() {
    let ast = vec![Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        name: "main".to_string(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(Type::Named("Unit".to_string())),
        where_clause: vec![],
        body: Block {
            stmts: vec![],
            ret: Some(Box::new(Spanned {
                node: Expr::If {
                    condition: Box::new(Spanned {
                        node: Expr::Literal(Literal::Bool(true)),
                        span: Span::new(0, 0),
                    }),
                    consequence: Box::new(Spanned {
                        node: Expr::Literal(Literal::Unit),
                        span: Span::new(0, 0),
                    }),
                    alternative: None,
                },
                span: Span::new(0, 0),
            })),
        },
    })];

    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::UNIT);
}

#[test]
fn verify_compile_while_never_executes() {
    let ast = vec![Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        name: "main".to_string(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(Type::Named("Unit".to_string())),
        where_clause: vec![],
        body: Block {
            stmts: vec![],
            ret: Some(Box::new(Spanned {
                node: Expr::While {
                    condition: Box::new(Spanned {
                        node: Expr::Literal(Literal::Bool(false)),
                        span: Span::new(0, 0),
                    }),
                    body: Block {
                        stmts: vec![],
                        ret: None,
                    },
                },
                span: Span::new(0, 0),
            })),
        },
    })];

    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::UNIT);
}

#[test]
fn verify_compile_if_non_bool_condition_truthy() {
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
                node: Expr::If {
                    condition: Box::new(Spanned {
                        node: Expr::Literal(Literal::Int(5)),
                        span: Span::new(0, 0),
                    }),
                    consequence: Box::new(Spanned {
                        node: Expr::Literal(Literal::Int(100)),
                        span: Span::new(0, 0),
                    }),
                    alternative: Some(Box::new(Spanned {
                        node: Expr::Literal(Literal::Int(200)),
                        span: Span::new(0, 0),
                    })),
                },
                span: Span::new(0, 0),
            })),
        },
    })];

    let result = compile_and_run(&ast);
    assert_eq!(result, Ok(Value::from_int(100)), "Truthy int (5) should take consequence");
}

#[test]
fn verify_compile_if_zero_condition_falsy() {
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
                node: Expr::If {
                    condition: Box::new(Spanned {
                        node: Expr::Literal(Literal::Int(0)),
                        span: Span::new(0, 0),
                    }),
                    consequence: Box::new(Spanned {
                        node: Expr::Literal(Literal::Int(100)),
                        span: Span::new(0, 0),
                    }),
                    alternative: Some(Box::new(Spanned {
                        node: Expr::Literal(Literal::Int(200)),
                        span: Span::new(0, 0),
                    })),
                },
                span: Span::new(0, 0),
            })),
        },
    })];

    let result = compile_and_run(&ast);
    assert_eq!(result, Ok(Value::from_int(200)), "Falsy int (0) should take alternative");
}
