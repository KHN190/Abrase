#[path = "compiler_codegen_common.rs"]
mod compiler_codegen_common;

use compiler_codegen_common::*;
use myriad::Value;

fn run(src: &str) -> Result<Value, String> {
    let mut parser = Parser::new(Lexer::new(src)).with_source(src.into());
    let ast = parser.parse_program();
    if !parser.errors.is_empty() {
        return Err(format!("parse: {}", parser.pretty_print_errors()));
    }
    let mut compiler = Compiler::new().with_source(src.into());
    let module = compiler.compile_module(&ast)
        .map_err(|_| compiler.pretty_print_errors())?;
    let mut vm = VirtualMachine::new();
    vm.run_module(&module)
}

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

#[test]
fn tuple_construction_and_index() {
    let src = "fn main() -> Int { let t = (10, 20, 30); t[1] }";
    assert_eq!(run(src), Ok(Value::from_int(20)));
}

#[test]
fn loop_with_break_value() {
    let src = r#"
        fn main() -> Int {
            let mut i = 0;
            loop {
                if i == 5 { break i };
                i = i + 1
            }
        }
    "#;
    assert_eq!(run(src), Ok(Value::from_int(5)));
}

#[test]
fn loop_break_without_value() {
    let src = r#"
        fn main() -> Int {
            let mut i = 0;
            loop {
                if i == 3 { break };
                i = i + 1
            };
            i
        }
    "#;
    assert_eq!(run(src), Ok(Value::from_int(3)));
}

#[test]
fn break_outside_loop_rejected() {
    let src = "fn main() -> Int { break; 0 }";
    assert!(run(src).is_err());
}

#[test]
fn continue_outside_loop_rejected() {
    let src = "fn main() -> Int { continue; 0 }";
    assert!(run(src).is_err());
}

#[test]
fn for_range_exclusive_sums() {
    let src = r#"
        fn main() -> Int {
            let mut sum = 0;
            for i in 0..5 {
                sum = sum + i
            };
            sum
        }
    "#;
    assert_eq!(run(src), Ok(Value::from_int(10)));
}

#[test]
fn for_range_inclusive_sums() {
    let src = r#"
        fn main() -> Int {
            let mut sum = 0;
            for i in 1..=5 {
                sum = sum + i
            };
            sum
        }
    "#;
    assert_eq!(run(src), Ok(Value::from_int(15)));
}

#[test]
fn for_with_break() {
    let src = r#"
        fn main() -> Int {
            let mut s = 0;
            for i in 0..100 {
                if i == 4 { break };
                s = s + i
            };
            s
        }
    "#;
    assert_eq!(run(src), Ok(Value::from_int(6)));
}

#[test]
fn for_with_continue_skip_even() {
    let src = r#"
        fn main() -> Int {
            let mut s = 0;
            for i in 0..6 {
                if i % 2 == 0 { continue };
                s = s + i
            };
            s
        }
    "#;
    assert_eq!(run(src), Ok(Value::from_int(9)));
}

#[test]
fn nested_loop_inner_break_does_not_affect_outer() {
    let src = r#"
        fn main() -> Int {
            let mut outer = 0;
            for i in 0..3 {
                for j in 0..10 {
                    if j == 2 { break };
                    outer = outer + 1
                }
            };
            outer
        }
    "#;
    assert_eq!(run(src), Ok(Value::from_int(6)));
}

