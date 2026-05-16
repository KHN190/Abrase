#[path = "compiler_codegen_common.rs"]
mod compiler_codegen_common;

use compiler_codegen_common::*;
use abrase::vm::Value;

#[test]
fn verify_compile_match_literal_int_with_wildcard() {
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
                node: Expr::Match {
                    scrutinee: Box::new(Spanned {
                        node: Expr::Literal(Literal::Int(5)),
                        span: Span::new(0, 0),
                    }),
                    arms: vec![
                        MatchArm {
                            pattern: Spanned {
                                node: Pattern::Literal(Literal::Int(5)),
                                span: Span::new(0, 0),
                            },
                            guard: None,
                            body: Spanned {
                                node: Expr::Literal(Literal::Int(10)),
                                span: Span::new(0, 0),
                            },
                        },
                        MatchArm {
                            pattern: Spanned {
                                node: Pattern::Wildcard,
                                span: Span::new(0, 0),
                            },
                            guard: None,
                            body: Spanned {
                                node: Expr::Literal(Literal::Int(0)),
                                span: Span::new(0, 0),
                            },
                        },
                    ],
                },
                span: Span::new(0, 0),
            })),
        },
    })];

    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::from_int(10));
}

#[test]
fn verify_compile_match_wildcard_only() {
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
                node: Expr::Match {
                    scrutinee: Box::new(Spanned {
                        node: Expr::Literal(Literal::Int(99)),
                        span: Span::new(0, 0),
                    }),
                    arms: vec![
                        MatchArm {
                            pattern: Spanned {
                                node: Pattern::Wildcard,
                                span: Span::new(0, 0),
                            },
                            guard: None,
                            body: Spanned {
                                node: Expr::Literal(Literal::Int(42)),
                                span: Span::new(0, 0),
                            },
                        },
                    ],
                },
                span: Span::new(0, 0),
            })),
        },
    })];

    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::from_int(42));
}

#[test]
fn verify_compile_match_bind_pattern() {
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
                node: Expr::Match {
                    scrutinee: Box::new(Spanned {
                        node: Expr::Literal(Literal::Int(5)),
                        span: Span::new(0, 0),
                    }),
                    arms: vec![
                        MatchArm {
                            pattern: Spanned {
                                node: Pattern::Bind("x".to_string()),
                                span: Span::new(0, 0),
                            },
                            guard: None,
                            body: Spanned {
                                node: Expr::Binary {
                                    op: BinaryOp::Add,
                                    left: Box::new(Spanned {
                                        node: Expr::Identifier("x".to_string()),
                                        span: Span::new(0, 0),
                                    }),
                                    right: Box::new(Spanned {
                                        node: Expr::Literal(Literal::Int(10)),
                                        span: Span::new(0, 0),
                                    }),
                                },
                                span: Span::new(0, 0),
                            },
                        },
                    ],
                },
                span: Span::new(0, 0),
            })),
        },
    })];

    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::from_int(15));
}

#[test]
fn verify_compile_match_bool_patterns() {
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
                node: Expr::Match {
                    scrutinee: Box::new(Spanned {
                        node: Expr::Literal(Literal::Bool(true)),
                        span: Span::new(0, 0),
                    }),
                    arms: vec![
                        MatchArm {
                            pattern: Spanned {
                                node: Pattern::Literal(Literal::Bool(true)),
                                span: Span::new(0, 0),
                            },
                            guard: None,
                            body: Spanned {
                                node: Expr::Literal(Literal::Int(1)),
                                span: Span::new(0, 0),
                            },
                        },
                        MatchArm {
                            pattern: Spanned {
                                node: Pattern::Wildcard,
                                span: Span::new(0, 0),
                            },
                            guard: None,
                            body: Spanned {
                                node: Expr::Literal(Literal::Int(0)),
                                span: Span::new(0, 0),
                            },
                        },
                    ],
                },
                span: Span::new(0, 0),
            })),
        },
    })];

    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::from_int(1));
}

#[test]
fn verify_compile_match_non_exhaustive_errors() {
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
                node: Expr::Match {
                    scrutinee: Box::new(Spanned {
                        node: Expr::Literal(Literal::Int(5)),
                        span: Span::new(0, 0),
                    }),
                    arms: vec![
                        MatchArm {
                            pattern: Spanned {
                                node: Pattern::Literal(Literal::Int(1)),
                                span: Span::new(0, 0),
                            },
                            guard: None,
                            body: Spanned {
                                node: Expr::Literal(Literal::Int(10)),
                                span: Span::new(0, 0),
                            },
                        },
                    ],
                },
                span: Span::new(0, 0),
            })),
        },
    })];

    let result = compiler.compile(&ast);
    assert!(result.is_err(), "Expected error for non-exhaustive match");
}

#[test]
fn verify_compile_match_multiple_literals_with_wildcard() {
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
                node: Expr::Match {
                    scrutinee: Box::new(Spanned {
                        node: Expr::Literal(Literal::Int(2)),
                        span: Span::new(0, 0),
                    }),
                    arms: vec![
                        MatchArm {
                            pattern: Spanned {
                                node: Pattern::Literal(Literal::Int(1)),
                                span: Span::new(0, 0),
                            },
                            guard: None,
                            body: Spanned {
                                node: Expr::Literal(Literal::Int(10)),
                                span: Span::new(0, 0),
                            },
                        },
                        MatchArm {
                            pattern: Spanned {
                                node: Pattern::Literal(Literal::Int(2)),
                                span: Span::new(0, 0),
                            },
                            guard: None,
                            body: Spanned {
                                node: Expr::Literal(Literal::Int(20)),
                                span: Span::new(0, 0),
                            },
                        },
                        MatchArm {
                            pattern: Spanned {
                                node: Pattern::Literal(Literal::Int(3)),
                                span: Span::new(0, 0),
                            },
                            guard: None,
                            body: Spanned {
                                node: Expr::Literal(Literal::Int(30)),
                                span: Span::new(0, 0),
                            },
                        },
                        MatchArm {
                            pattern: Spanned {
                                node: Pattern::Wildcard,
                                span: Span::new(0, 0),
                            },
                            guard: None,
                            body: Spanned {
                                node: Expr::Literal(Literal::Int(99)),
                                span: Span::new(0, 0),
                            },
                        },
                    ],
                },
                span: Span::new(0, 0),
            })),
        },
    })];

    let result = compile_and_run(&ast);
    assert_eq!(result, Ok(Value::from_int(20)));
}

#[test]
fn verify_compile_match_nested_in_if() {
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
                        node: Expr::Match {
                            scrutinee: Box::new(Spanned {
                                node: Expr::Literal(Literal::Int(2)),
                                span: Span::new(0, 0),
                            }),
                            arms: vec![
                                MatchArm {
                                    pattern: Spanned {
                                        node: Pattern::Literal(Literal::Int(1)),
                                        span: Span::new(0, 0),
                                    },
                                    guard: None,
                                    body: Spanned {
                                        node: Expr::Literal(Literal::Int(10)),
                                        span: Span::new(0, 0),
                                    },
                                },
                                MatchArm {
                                    pattern: Spanned {
                                        node: Pattern::Literal(Literal::Int(2)),
                                        span: Span::new(0, 0),
                                    },
                                    guard: None,
                                    body: Spanned {
                                        node: Expr::Literal(Literal::Int(20)),
                                        span: Span::new(0, 0),
                                    },
                                },
                                MatchArm {
                                    pattern: Spanned {
                                        node: Pattern::Wildcard,
                                        span: Span::new(0, 0),
                                    },
                                    guard: None,
                                    body: Spanned {
                                        node: Expr::Literal(Literal::Int(99)),
                                        span: Span::new(0, 0),
                                    },
                                },
                            ],
                        },
                        span: Span::new(0, 0),
                    }),
                    alternative: None,
                },
                span: Span::new(0, 0),
            })),
        },
    })];

    let result = compile_and_run(&ast);
    assert_eq!(result, Ok(Value::from_int(20)));
}

#[test]
fn verify_compile_match_variant_pattern_unsupported() {
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
                node: Expr::Match {
                    scrutinee: Box::new(Spanned {
                        node: Expr::Literal(Literal::Int(1)),
                        span: Span::new(0, 0),
                    }),
                    arms: vec![
                        MatchArm {
                            pattern: Spanned {
                                node: Pattern::Variant {
                                    ty: vec!["Color".to_string()],
                                    args: vec![Spanned {
                                        node: Pattern::Bind("x".to_string()),
                                        span: Span::new(0, 0),
                                    }],
                                },
                                span: Span::new(0, 0),
                            },
                            guard: None,
                            body: Spanned {
                                node: Expr::Literal(Literal::Int(1)),
                                span: Span::new(0, 0),
                            },
                        },
                        MatchArm {
                            pattern: Spanned {
                                node: Pattern::Wildcard,
                                span: Span::new(0, 0),
                            },
                            guard: None,
                            body: Spanned {
                                node: Expr::Literal(Literal::Int(0)),
                                span: Span::new(0, 0),
                            },
                        },
                    ],
                },
                span: Span::new(0, 0),
            })),
        },
    })];

    let result = compile_and_run(&ast);
    assert!(result.is_err(), "Variant patterns is not supported yet");
}

#[test]
fn verify_compile_match_or_pattern_unsupported() {
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
                node: Expr::Match {
                    scrutinee: Box::new(Spanned {
                        node: Expr::Literal(Literal::Int(1)),
                        span: Span::new(0, 0),
                    }),
                    arms: vec![
                        MatchArm {
                            pattern: Spanned {
                                node: Pattern::Or(vec![
                                    Spanned {
                                        node: Pattern::Literal(Literal::Int(1)),
                                        span: Span::new(0, 0),
                                    },
                                    Spanned {
                                        node: Pattern::Literal(Literal::Int(2)),
                                        span: Span::new(0, 0),
                                    },
                                ]),
                                span: Span::new(0, 0),
                            },
                            guard: None,
                            body: Spanned {
                                node: Expr::Literal(Literal::Int(99)),
                                span: Span::new(0, 0),
                            },
                        },
                        MatchArm {
                            pattern: Spanned {
                                node: Pattern::Wildcard,
                                span: Span::new(0, 0),
                            },
                            guard: None,
                            body: Spanned {
                                node: Expr::Literal(Literal::Int(0)),
                                span: Span::new(0, 0),
                            },
                        },
                    ],
                },
                span: Span::new(0, 0),
            })),
        },
    })];

    let result = compile_and_run(&ast);
    assert!(result.is_err(), "Or patterns should is not supported");
}
