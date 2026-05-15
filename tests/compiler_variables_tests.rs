#[path = "compiler_codegen_common.rs"]
mod compiler_codegen_common;

use compiler_codegen_common::*;
use abrase::vm::Value;

#[test]
fn verify_compile_variable_binding() {
    // let x = 5; let y = x + 1; y
    let ast = parse_let_expr();
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::Int(6));
}

#[test]
fn verify_compile_multiple_variables() {
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
                            node: Pattern::Bind("a".to_string()),
                            span: Span::new(0, 0),
                        },
                        is_mut: false,
                        ty: Some(Type::Named("Int".to_string())),
                        value: Spanned {
                            node: Expr::Literal(Literal::Int(10)),
                            span: Span::new(0, 0),
                        },
                    },
                    span: Span::new(0, 0),
                },
                Spanned {
                    node: Stmt::Let {
                        pattern: Spanned {
                            node: Pattern::Bind("b".to_string()),
                            span: Span::new(0, 0),
                        },
                        is_mut: false,
                        ty: Some(Type::Named("Int".to_string())),
                        value: Spanned {
                            node: Expr::Literal(Literal::Int(20)),
                            span: Span::new(0, 0),
                        },
                    },
                    span: Span::new(0, 0),
                },
                Spanned {
                    node: Stmt::Let {
                        pattern: Spanned {
                            node: Pattern::Bind("c".to_string()),
                            span: Span::new(0, 0),
                        },
                        is_mut: false,
                        ty: Some(Type::Named("Int".to_string())),
                        value: Spanned {
                            node: Expr::Binary {
                                op: BinaryOp::Add,
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
                        },
                    },
                    span: Span::new(0, 0),
                },
            ],
            ret: Some(Box::new(Spanned {
                node: Expr::Identifier("c".to_string()),
                span: Span::new(0, 0),
            })),
        },
    })];

    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::Int(30));
}

#[test]
fn verify_compile_assignment_literal_int() {
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
                            node: Pattern::Bind("x".to_string()),
                            span: Span::new(0, 0),
                        },
                        is_mut: true,
                        ty: Some(Type::Named("Int".to_string())),
                        value: Spanned {
                            node: Expr::Literal(Literal::Int(10)),
                            span: Span::new(0, 0),
                        },
                    },
                    span: Span::new(0, 0),
                },
                Spanned {
                    node: Stmt::Expr(Spanned {
                        node: Expr::Binary {
                            op: BinaryOp::Assign,
                            left: Box::new(Spanned {
                                node: Expr::Identifier("x".to_string()),
                                span: Span::new(0, 0),
                            }),
                            right: Box::new(Spanned {
                                node: Expr::Literal(Literal::Int(42)),
                                span: Span::new(0, 0),
                            }),
                        },
                        span: Span::new(0, 0),
                    }),
                    span: Span::new(0, 0),
                },
            ],
            ret: Some(Box::new(Spanned {
                node: Expr::Identifier("x".to_string()),
                span: Span::new(0, 0),
            })),
        },
    })];

    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::Int(42));
}

#[test]
fn verify_compile_assignment_multiple() {
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
                            node: Pattern::Bind("x".to_string()),
                            span: Span::new(0, 0),
                        },
                        is_mut: true,
                        ty: Some(Type::Named("Int".to_string())),
                        value: Spanned {
                            node: Expr::Literal(Literal::Int(1)),
                            span: Span::new(0, 0),
                        },
                    },
                    span: Span::new(0, 0),
                },
                Spanned {
                    node: Stmt::Expr(Spanned {
                        node: Expr::Binary {
                            op: BinaryOp::Assign,
                            left: Box::new(Spanned {
                                node: Expr::Identifier("x".to_string()),
                                span: Span::new(0, 0),
                            }),
                            right: Box::new(Spanned {
                                node: Expr::Literal(Literal::Int(2)),
                                span: Span::new(0, 0),
                            }),
                        },
                        span: Span::new(0, 0),
                    }),
                    span: Span::new(0, 0),
                },
                Spanned {
                    node: Stmt::Expr(Spanned {
                        node: Expr::Binary {
                            op: BinaryOp::Assign,
                            left: Box::new(Spanned {
                                node: Expr::Identifier("x".to_string()),
                                span: Span::new(0, 0),
                            }),
                            right: Box::new(Spanned {
                                node: Expr::Literal(Literal::Int(3)),
                                span: Span::new(0, 0),
                            }),
                        },
                        span: Span::new(0, 0),
                    }),
                    span: Span::new(0, 0),
                },
            ],
            ret: Some(Box::new(Spanned {
                node: Expr::Identifier("x".to_string()),
                span: Span::new(0, 0),
            })),
        },
    })];

    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::Int(3));
}

#[test]
fn verify_compile_assignment_bool() {
    let ast = vec![Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        name: "main".to_string(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(Type::Named("Bool".to_string())),
        where_clause: vec![],
        body: Block {
            stmts: vec![
                Spanned {
                    node: Stmt::Let {
                        pattern: Spanned {
                            node: Pattern::Bind("flag".to_string()),
                            span: Span::new(0, 0),
                        },
                        is_mut: true,
                        ty: Some(Type::Named("Bool".to_string())),
                        value: Spanned {
                            node: Expr::Literal(Literal::Bool(true)),
                            span: Span::new(0, 0),
                        },
                    },
                    span: Span::new(0, 0),
                },
                Spanned {
                    node: Stmt::Expr(Spanned {
                        node: Expr::Binary {
                            op: BinaryOp::Assign,
                            left: Box::new(Spanned {
                                node: Expr::Identifier("flag".to_string()),
                                span: Span::new(0, 0),
                            }),
                            right: Box::new(Spanned {
                                node: Expr::Literal(Literal::Bool(false)),
                                span: Span::new(0, 0),
                            }),
                        },
                        span: Span::new(0, 0),
                    }),
                    span: Span::new(0, 0),
                },
            ],
            ret: Some(Box::new(Spanned {
                node: Expr::Identifier("flag".to_string()),
                span: Span::new(0, 0),
            })),
        },
    })];

    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn verify_compile_assignment_to_literal_errors() {
    let mut compiler = Compiler::new();
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
                node: Expr::Binary {
                    op: BinaryOp::Assign,
                    left: Box::new(Spanned {
                        node: Expr::Literal(Literal::Int(1)),
                        span: Span::new(0, 0),
                    }),
                    right: Box::new(Spanned {
                        node: Expr::Literal(Literal::Int(2)),
                        span: Span::new(0, 0),
                    }),
                },
                span: Span::new(0, 0),
            })),
        },
    })];

    let result = compiler.compile(&ast);
    assert!(result.is_err(), "Expected error for assignment to literal");
}
