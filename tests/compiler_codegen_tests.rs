// Code generation, optimization, phases
use ect::ast::*;
use ect::compiler::Compiler;
use ect::vm::{Value, VirtualMachine};

fn compile_and_run(ast: &[Decl]) -> Result<Value, String> {
    let mut compiler = Compiler::new();
    let chunk = compiler.compile(ast)?;
    let mut vm = VirtualMachine::new();
    vm.run(&chunk)
}

fn compile_module_and_run(ast: &[Decl]) -> Result<Value, String> {
    let mut compiler = Compiler::new();
    let module = compiler.compile_module(ast).map_err(|errs| {
        errs.iter()
            .map(|e| format!("{:?} at {}:{}: {}", e.code, e.span.line, e.span.col, e.message))
            .collect::<Vec<_>>()
            .join("\n")
    })?;
    let mut vm = VirtualMachine::new();
    vm.run_module(&module)
}

fn parse_literal_int(n: i64) -> Vec<Decl> {
    vec![Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
        name: "main".to_string(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(Type::Named("Int".to_string())),
        where_clause: vec![],
        body: Block {
            stmts: vec![],
            ret: Some(Box::new(Spanned {
                node: Expr::Literal(Literal::Int(n)),
                span: Span::new(0, 0),
            })),
        },
    })]
}

fn parse_binary_int(left: i64, op: BinaryOp, right: i64) -> Vec<Decl> {
    vec![Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
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
                    op,
                    left: Box::new(Spanned {
                        node: Expr::Literal(Literal::Int(left)),
                        span: Span::new(0, 0),
                    }),
                    right: Box::new(Spanned {
                        node: Expr::Literal(Literal::Int(right)),
                        span: Span::new(0, 0),
                    }),
                },
                span: Span::new(0, 0),
            })),
        },
    })]
}

fn parse_arithmetic_expr() -> Vec<Decl> {
    vec![Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
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
                    op: BinaryOp::Add,
                    left: Box::new(Spanned {
                        node: Expr::Literal(Literal::Int(2)),
                        span: Span::new(0, 0),
                    }),
                    right: Box::new(Spanned {
                        node: Expr::Binary {
                            op: BinaryOp::Mul,
                            left: Box::new(Spanned {
                                node: Expr::Literal(Literal::Int(3)),
                                span: Span::new(0, 0),
                            }),
                            right: Box::new(Spanned {
                                node: Expr::Literal(Literal::Int(4)),
                                span: Span::new(0, 0),
                            }),
                        },
                        span: Span::new(0, 0),
                    }),
                },
                span: Span::new(0, 0),
            })),
        },
    })]
}

fn parse_let_expr() -> Vec<Decl> {
    vec![Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
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
                        is_mut: false,
                        ty: Some(Type::Named("Int".to_string())),
                        value: Spanned {
                            node: Expr::Literal(Literal::Int(5)),
                            span: Span::new(0, 0),
                        },
                    },
                    span: Span::new(0, 0),
                },
                Spanned {
                    node: Stmt::Let {
                        pattern: Spanned {
                            node: Pattern::Bind("y".to_string()),
                            span: Span::new(0, 0),
                        },
                        is_mut: false,
                        ty: Some(Type::Named("Int".to_string())),
                        value: Spanned {
                            node: Expr::Binary {
                                op: BinaryOp::Add,
                                left: Box::new(Spanned {
                                    node: Expr::Identifier("x".to_string()),
                                    span: Span::new(0, 0),
                                }),
                                right: Box::new(Spanned {
                                    node: Expr::Literal(Literal::Int(1)),
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
                node: Expr::Identifier("y".to_string()),
                span: Span::new(0, 0),
            })),
        },
    })]
}

// Literals & Constants
#[test]
fn verify_compile_literal_int() {
    let ast = parse_literal_int(42);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::Int(42));
}

#[test]
fn verify_compile_literal_bool() {
    let ast = vec![Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
        name: "main".to_string(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(Type::Named("Bool".to_string())),
        where_clause: vec![],
        body: Block {
            stmts: vec![],
            ret: Some(Box::new(Spanned {
                node: Expr::Literal(Literal::Bool(true)),
                span: Span::new(0, 0),
            })),
        },
    })];

    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn verify_compile_literal_string() {
    let ast = vec![Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
        name: "main".to_string(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(Type::Named("String".to_string())),
        where_clause: vec![],
        body: Block {
            stmts: vec![],
            ret: Some(Box::new(Spanned {
                node: Expr::Literal(Literal::String("hello".to_string())),
                span: Span::new(0, 0),
            })),
        },
    })];

    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::String("hello".to_string()));
}

#[test]
fn verify_compile_literal_float() {
    let ast = vec![Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
        name: "main".to_string(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(Type::Named("Float".to_string())),
        where_clause: vec![],
        body: Block {
            stmts: vec![],
            ret: Some(Box::new(Spanned {
                node: Expr::Literal(Literal::Float(3.14)),
                span: Span::new(0, 0),
            })),
        },
    })];

    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::Float(3.14));
}

#[test]
fn verify_compile_literal_unit() {
    let ast = vec![Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
        name: "main".to_string(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(Type::Named("Unit".to_string())),
        where_clause: vec![],
        body: Block {
            stmts: vec![],
            ret: Some(Box::new(Spanned {
                node: Expr::Literal(Literal::Unit),
                span: Span::new(0, 0),
            })),
        },
    })];

    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::Unit);
}

#[test]
fn verify_compile_literal_bool_false() {
    let ast = vec![Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
        name: "main".to_string(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(Type::Named("Bool".to_string())),
        where_clause: vec![],
        body: Block {
            stmts: vec![],
            ret: Some(Box::new(Spanned {
                node: Expr::Literal(Literal::Bool(false)),
                span: Span::new(0, 0),
            })),
        },
    })];

    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::Bool(false));
}

// Arithmetic Operations
#[test]
fn verify_compile_arithmetic_add() {
    let ast = parse_binary_int(2, BinaryOp::Add, 3);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::Int(5));
}

#[test]
fn verify_compile_arithmetic_sub() {
    let ast = parse_binary_int(10, BinaryOp::Sub, 3);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::Int(7));
}

#[test]
fn verify_compile_arithmetic_mul() {
    let ast = parse_binary_int(3, BinaryOp::Mul, 4);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::Int(12));
}

#[test]
fn verify_compile_arithmetic_div() {
    let ast = parse_binary_int(20, BinaryOp::Div, 4);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::Int(5));
}

#[test]
fn verify_compile_arithmetic_mod() {
    let ast = parse_binary_int(10, BinaryOp::Mod, 3);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::Int(1));
}

#[test]
fn verify_compile_arithmetic_respects_precedence() {
    // 2 + 3 * 4 = 2 + 12 = 14
    let ast = parse_arithmetic_expr();
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::Int(14));
}

// Variables & Assignment
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
        is_async: false,
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
fn verify_compile_pure_functional_arithmetic() {
    let test_cases = vec![
        (parse_literal_int(0), Value::Int(0)),
        (parse_literal_int(100), Value::Int(100)),
        (parse_binary_int(5, BinaryOp::Add, 3), Value::Int(8)),
        (parse_binary_int(15, BinaryOp::Sub, 7), Value::Int(8)),
        (parse_binary_int(6, BinaryOp::Mul, 7), Value::Int(42)),
        (parse_binary_int(100, BinaryOp::Div, 10), Value::Int(10)),
        (parse_binary_int(17, BinaryOp::Mod, 5), Value::Int(2)),
    ];

    for (ast, expected) in test_cases {
        let result = compile_and_run(&ast).expect("Execution failed");
        assert_eq!(result, expected, "Arithmetic test failed");
    }
}

// Comparison Operators
#[test]
fn verify_compile_comparison_eq() {
    let ast = parse_binary_int(5, BinaryOp::Eq, 5);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn verify_compile_comparison_neq() {
    let ast = parse_binary_int(5, BinaryOp::Neq, 3);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn verify_compile_comparison_lt() {
    let ast = parse_binary_int(3, BinaryOp::Lt, 5);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn verify_compile_comparison_gt() {
    let ast = parse_binary_int(5, BinaryOp::Gt, 3);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn verify_compile_comparison_lte() {
    let ast = parse_binary_int(3, BinaryOp::Lte, 5);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn verify_compile_comparison_gte() {
    let ast = parse_binary_int(5, BinaryOp::Gte, 3);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::Bool(true));
}

// If/Else
#[test]
fn verify_compile_if_true_branch() {
    let ast = vec![Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
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
    assert_eq!(result, Value::Int(10));
}

#[test]
fn verify_compile_if_false_branch() {
    let ast = vec![Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
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
    assert_eq!(result, Value::Int(20));
}

#[test]
fn verify_compile_if_with_comparison() {
    let ast = vec![Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
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
    assert_eq!(result, Value::Int(100));
}

// If without else
#[test]
fn verify_compile_if_without_else_true() {
    let ast = vec![Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
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
    assert_eq!(result, Value::Int(42));
}

#[test]
fn verify_compile_if_without_else_false() {
    let ast = vec![Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
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
    assert_eq!(result, Value::Unit);
}

// While Loop
#[test]
fn verify_compile_while_loop_simple() {
    let ast = vec![Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
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
    assert_eq!(result, Value::Unit);
}

#[test]
fn verify_compile_while_with_comparison() {
    let ast = vec![Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
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
    assert_eq!(result, Value::Unit);
}

// F7: While loop with mutable variable
#[test]
fn verify_compile_while_loop_with_mutation() {
    let ast = vec![Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
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
    assert_eq!(result, Value::Int(5));
}

// If without else returning Unit
#[test]
fn verify_compile_if_without_else_returns_unit() {
    let ast = vec![Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
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
    assert_eq!(result, Value::Unit);
}

// While that never executes
#[test]
fn verify_compile_while_never_executes() {
    let ast = vec![Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
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
    assert_eq!(result, Value::Unit);
}

// Error cases
#[test]
fn verify_compile_undefined_variable_errors() {
    let mut compiler = Compiler::new();
    let ast = vec![Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
        name: "main".to_string(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(Type::Named("Int".to_string())),
        where_clause: vec![],
        body: Block {
            stmts: vec![],
            ret: Some(Box::new(Spanned {
                node: Expr::Identifier("undefined_var".to_string()),
                span: Span::new(0, 0),
            })),
        },
    })];

    let result = compiler.compile(&ast);
    assert!(result.is_err(), "Expected error for undefined variable");
}

#[test]
fn verify_compile_non_bind_pattern_errors() {
    let mut compiler = Compiler::new();
    let ast = vec![Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
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
                            node: Pattern::Tuple(vec![
                                Spanned {
                                    node: Pattern::Bind("x".to_string()),
                                    span: Span::new(0, 0),
                                },
                                Spanned {
                                    node: Pattern::Bind("y".to_string()),
                                    span: Span::new(0, 0),
                                },
                            ]),
                            span: Span::new(0, 0),
                        },
                        is_mut: false,
                        ty: None,
                        value: Spanned {
                            node: Expr::Literal(Literal::Int(5)),
                            span: Span::new(0, 0),
                        },
                    },
                    span: Span::new(0, 0),
                },
            ],
            ret: Some(Box::new(Spanned {
                node: Expr::Literal(Literal::Int(0)),
                span: Span::new(0, 0),
            })),
        },
    })];

    let result = compiler.compile(&ast);
    assert!(result.is_err(), "Expected error for non-bind pattern");
}

// Assignment with literals
#[test]
fn verify_compile_assignment_literal_int() {
    let ast = vec![Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
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
        is_async: false,
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
        is_async: false,
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

// Comparison false cases
#[test]
fn verify_compile_comparison_eq_false() {
    let ast = parse_binary_int(5, BinaryOp::Eq, 3);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn verify_compile_comparison_neq_false() {
    let ast = parse_binary_int(5, BinaryOp::Neq, 5);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn verify_compile_comparison_lt_false() {
    let ast = parse_binary_int(5, BinaryOp::Lt, 3);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn verify_compile_comparison_gt_false() {
    let ast = parse_binary_int(3, BinaryOp::Gt, 5);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn verify_compile_comparison_lte_false() {
    let ast = parse_binary_int(5, BinaryOp::Lte, 3);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn verify_compile_comparison_gte_false() {
    let ast = parse_binary_int(3, BinaryOp::Gte, 5);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::Bool(false));
}

// Boundary cases for lte/gte
#[test]
fn verify_compile_comparison_lte_equal() {
    let ast = parse_binary_int(5, BinaryOp::Lte, 5);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn verify_compile_comparison_gte_equal() {
    let ast = parse_binary_int(5, BinaryOp::Gte, 5);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::Bool(true));
}

// Error cases
#[test]
fn verify_compile_unsupported_literal_char_errors() {
    let mut compiler = Compiler::new();
    let ast = vec![Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
        name: "main".to_string(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(Type::Named("Char".to_string())),
        where_clause: vec![],
        body: Block {
            stmts: vec![],
            ret: Some(Box::new(Spanned {
                node: Expr::Literal(Literal::Char('a')),
                span: Span::new(0, 0),
            })),
        },
    })];

    let result = compiler.compile(&ast);
    assert!(result.is_err(), "Expected error for unsupported Char literal");
}

#[test]
fn verify_compile_unsupported_literal_string_interp_errors() {
    let mut compiler = Compiler::new();
    let ast = vec![Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
        name: "main".to_string(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(Type::Named("String".to_string())),
        where_clause: vec![],
        body: Block {
            stmts: vec![],
            ret: Some(Box::new(Spanned {
                node: Expr::Literal(Literal::StringInterp(vec![
                    StringPart::Literal("hello".to_string()),
                ])),
                span: Span::new(0, 0),
            })),
        },
    })];

    let result = compiler.compile(&ast);
    assert!(result.is_err(), "Expected error for unsupported StringInterp literal");
}

#[test]
fn verify_compile_assignment_to_literal_errors() {
    let mut compiler = Compiler::new();
    let ast = vec![Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
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

#[test]
fn verify_compile_unsupported_binary_op_and_errors() {
    let mut compiler = Compiler::new();
    let ast = vec![Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
        name: "main".to_string(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(Type::Named("Bool".to_string())),
        where_clause: vec![],
        body: Block {
            stmts: vec![],
            ret: Some(Box::new(Spanned {
                node: Expr::Binary {
                    op: BinaryOp::And,
                    left: Box::new(Spanned {
                        node: Expr::Literal(Literal::Bool(true)),
                        span: Span::new(0, 0),
                    }),
                    right: Box::new(Spanned {
                        node: Expr::Literal(Literal::Bool(false)),
                        span: Span::new(0, 0),
                    }),
                },
                span: Span::new(0, 0),
            })),
        },
    })];

    let result = compiler.compile(&ast);
    assert!(result.is_err(), "Expected error for unsupported And operator");
}

#[test]
fn verify_compile_unsupported_binary_op_or_errors() {
    let mut compiler = Compiler::new();
    let ast = vec![Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
        name: "main".to_string(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(Type::Named("Bool".to_string())),
        where_clause: vec![],
        body: Block {
            stmts: vec![],
            ret: Some(Box::new(Spanned {
                node: Expr::Binary {
                    op: BinaryOp::Or,
                    left: Box::new(Spanned {
                        node: Expr::Literal(Literal::Bool(true)),
                        span: Span::new(0, 0),
                    }),
                    right: Box::new(Spanned {
                        node: Expr::Literal(Literal::Bool(false)),
                        span: Span::new(0, 0),
                    }),
                },
                span: Span::new(0, 0),
            })),
        },
    })];

    let result = compiler.compile(&ast);
    assert!(result.is_err(), "Expected error for unsupported Or operator");
}

#[test]
fn verify_compile_unsupported_binary_op_add_assign_errors() {
    let mut compiler = Compiler::new();
    let ast = vec![Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
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
                    op: BinaryOp::AddAssign,
                    left: Box::new(Spanned {
                        node: Expr::Identifier("x".to_string()),
                        span: Span::new(0, 0),
                    }),
                    right: Box::new(Spanned {
                        node: Expr::Literal(Literal::Int(1)),
                        span: Span::new(0, 0),
                    }),
                },
                span: Span::new(0, 0),
            })),
        },
    })];

    let result = compiler.compile(&ast);
    assert!(result.is_err(), "Expected error for unsupported AddAssign operator");
}

// Match expressions - properly exhaustive patterns
#[test]
fn verify_compile_match_literal_int_with_wildcard() {
    let ast = vec![Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
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
    assert_eq!(result, Value::Int(10));
}

#[test]
fn verify_compile_match_wildcard_only() {
    let ast = vec![Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
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
    assert_eq!(result, Value::Int(42));
}

#[test]
fn verify_compile_match_bind_pattern() {
    let ast = vec![Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
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
    assert_eq!(result, Value::Int(15));
}

#[test]
fn verify_compile_match_bool_patterns() {
    let ast = vec![Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
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
    assert_eq!(result, Value::Int(1));
}

#[test]
fn verify_compile_match_non_exhaustive_errors() {
    let mut compiler = Compiler::new();
    let ast = vec![Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
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

// Match Expr

#[test]
fn verify_compile_match_multiple_literals_with_wildcard() {
    let ast = vec![Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
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
    assert_eq!(result, Ok(Value::Int(20)));
}

#[test]
fn verify_compile_match_nested_in_if() {
    let ast = vec![Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
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
    assert_eq!(result, Ok(Value::Int(20)));
}

#[test]
fn verify_compile_if_non_bool_condition_truthy() {
    let ast = vec![Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
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
    assert_eq!(result, Ok(Value::Int(100)), "Truthy int (5) should take consequence");
}

#[test]
fn verify_compile_if_zero_condition_falsy() {
    let ast = vec![Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
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
    assert_eq!(result, Ok(Value::Int(200)), "Falsy int (0) should take alternative");
}

#[test]
fn verify_compile_match_variant_pattern_unsupported() {
    let ast = vec![Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
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
        is_async: false,
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

// Functions

#[test]
fn verify_compile_simple_function_call() {
    let ast = vec![
        Decl::Fn(FnDecl {
            attrs: vec![],
            is_pub: false,
            is_async: false,
            name: "add".to_string(),
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
            effects: vec![],
            return_type: Some(Type::Named("Int".to_string())),
            where_clause: vec![],
            body: Block {
                stmts: vec![],
                ret: Some(Box::new(Spanned {
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
                })),
            },
        }),
        Decl::Fn(FnDecl {
            attrs: vec![],
            is_pub: false,
            is_async: false,
            name: "main".to_string(),
            generics: vec![],
            params: vec![],
            effects: vec![],
            return_type: Some(Type::Named("Int".to_string())),
            where_clause: vec![],
            body: Block {
                stmts: vec![],
                ret: Some(Box::new(Spanned {
                    node: Expr::Call {
                        callee: Box::new(Spanned {
                            node: Expr::Identifier("add".to_string()),
                            span: Span::new(0, 0),
                        }),
                        args: vec![
                            Spanned {
                                node: Expr::Literal(Literal::Int(2)),
                                span: Span::new(0, 0),
                            },
                            Spanned {
                                node: Expr::Literal(Literal::Int(3)),
                                span: Span::new(0, 0),
                            },
                        ],
                    },
                    span: Span::new(0, 0),
                })),
            },
        }),
    ];

    let result = compile_module_and_run(&ast);
    assert_eq!(result, Ok(Value::Int(5)));
}

#[test]
fn verify_compile_return_explicit() {
    let ast = vec![Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
        name: "main".to_string(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(Type::Named("Int".to_string())),
        where_clause: vec![],
        body: Block {
            stmts: vec![],
            ret: Some(Box::new(Spanned {
                node: Expr::Return(Some(Box::new(Spanned {
                    node: Expr::Literal(Literal::Int(42)),
                    span: Span::new(0, 0),
                }))),
                span: Span::new(0, 0),
            })),
        },
    })];

    let result = compile_and_run(&ast);
    assert_eq!(result, Ok(Value::Int(42)));
}

#[test]
fn verify_compile_undefined_function_call_errors() {
    let ast = vec![Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        is_async: false,
        name: "main".to_string(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(Type::Named("Int".to_string())),
        where_clause: vec![],
        body: Block {
            stmts: vec![],
            ret: Some(Box::new(Spanned {
                node: Expr::Call {
                    callee: Box::new(Spanned {
                        node: Expr::Identifier("unknown".to_string()),
                        span: Span::new(0, 0),
                    }),
                    args: vec![],
                },
                span: Span::new(0, 0),
            })),
        },
    })];

    let mut compiler = Compiler::new();
    let result = compiler.compile_module(&ast);
    assert!(result.is_err(), "Expected error for undefined function");
}

#[test]
fn verify_compile_recursive_function() {
    let ast = vec![
        Decl::Fn(FnDecl {
            attrs: vec![],
            is_pub: false,
            is_async: false,
            name: "countdown".to_string(),
            generics: vec![],
            params: vec![Param::Named {
                pattern: Spanned {
                    node: Pattern::Bind("n".to_string()),
                    span: Span::new(0, 0),
                },
                ty: Type::Named("Int".to_string()),
            }],
            effects: vec![],
            return_type: Some(Type::Named("Int".to_string())),
            where_clause: vec![],
            body: Block {
                stmts: vec![],
                ret: Some(Box::new(Spanned {
                    node: Expr::If {
                        condition: Box::new(Spanned {
                            node: Expr::Binary {
                                op: BinaryOp::Lte,
                                left: Box::new(Spanned {
                                    node: Expr::Identifier("n".to_string()),
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
                            node: Expr::Literal(Literal::Int(0)),
                            span: Span::new(0, 0),
                        }),
                        alternative: Some(Box::new(Spanned {
                            node: Expr::Call {
                                callee: Box::new(Spanned {
                                    node: Expr::Identifier("countdown".to_string()),
                                    span: Span::new(0, 0),
                                }),
                                args: vec![Spanned {
                                    node: Expr::Binary {
                                        op: BinaryOp::Sub,
                                        left: Box::new(Spanned {
                                            node: Expr::Identifier("n".to_string()),
                                            span: Span::new(0, 0),
                                        }),
                                        right: Box::new(Spanned {
                                            node: Expr::Literal(Literal::Int(1)),
                                            span: Span::new(0, 0),
                                        }),
                                    },
                                    span: Span::new(0, 0),
                                }],
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
            is_async: false,
            name: "main".to_string(),
            generics: vec![],
            params: vec![],
            effects: vec![],
            return_type: Some(Type::Named("Int".to_string())),
            where_clause: vec![],
            body: Block {
                stmts: vec![],
                ret: Some(Box::new(Spanned {
                    node: Expr::Call {
                        callee: Box::new(Spanned {
                            node: Expr::Identifier("countdown".to_string()),
                            span: Span::new(0, 0),
                        }),
                        args: vec![Spanned {
                            node: Expr::Literal(Literal::Int(3)),
                            span: Span::new(0, 0),
                        }],
                    },
                    span: Span::new(0, 0),
                })),
            },
        }),
    ];

    let result = compile_module_and_run(&ast);
    assert_eq!(result, Ok(Value::Int(0)));
}
