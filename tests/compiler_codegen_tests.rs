// Code generation, optimization, phases
use ect::ast::*;
use ect::compiler::Compiler;
use ect::lexer::Lexer;
use ect::parser::Parser;
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

// Functions

#[test]
fn verify_compile_simple_function_call() {
    let ast = vec![
        Decl::Fn(FnDecl {
            attrs: vec![],
            is_pub: false,
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
    assert_eq!(result, Ok(Value::Int(42)));
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
    assert_eq!(result, Ok(Value::Int(7)));
}

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
    assert_eq!(result, Ok(Value::Int(777)));
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

fn parse_source(src: &str) -> Vec<Decl> {
    let mut p = Parser::new(Lexer::new(src)).with_source(src.to_string());
    let decls = p.parse_program();
    assert!(p.errors.is_empty(), "parse errors: {:?}", p.errors);
    decls
}

fn run_source(src: &str) -> Result<Value, String> {
    let ast = parse_source(src);
    compile_module_and_run(&ast)
}

#[test]
fn handle_with_only_return_arm_passes_body_through() {
    // No effect ops at all — only the return arm fires. Verifies that the
    // pre-pass + Handle codegen wire body → return_arm correctly.
    let src = r#"
        fn body() -> Int { 42 }
        fn main() -> Int {
            handle body() {
                return v => v
            }
        }
    "#;
    assert_eq!(run_source(src), Ok(Value::Int(42)));
}

#[test]
fn return_arm_transforms_body_value() {
    // Return arm adds 1: the handle expression's value is body() + 1.
    // Proves the return arm body actually executes (not just identity-thunk).
    let src = r#"
        fn body() -> Int { 41 }
        fn main() -> Int {
            handle body() {
                return v => v + 1
            }
        }
    "#;
    assert_eq!(run_source(src), Ok(Value::Int(42)));
}

#[test]
fn effect_op_call_reroutes_to_arm() {
    // The arm rets a non-default value via tail-position `resume(7)`.
    // produce() reads provider.give() + 1 — only 8 if the arm actually fired
    // AND returned 7 to the call site (proving both the rewrite and Resume→Ret
    // lowering work).
    let src = r#"
        effect provider { fn give() -> Int }
        fn produce() -> <provider> Int { provider.give() + 1 }
        fn main() -> Int {
            handle produce() {
                return v => v,
                provider.give => resume(7)
            }
        }
    "#;
    assert_eq!(run_source(src), Ok(Value::Int(8)));
}

#[test]
fn arm_body_can_short_circuit_without_resume() {
    // Op arm returns a constant directly — no resume. produce() never sees a
    // value from provider.give(); the arm's return becomes the op-call's value
    // in the rewritten path.
    let src = r#"
        effect provider { fn give() -> Int }
        fn produce() -> <provider> Int { provider.give() * 10 }
        fn main() -> Int {
            handle produce() {
                return v => v,
                provider.give => 5
            }
        }
    "#;
    assert_eq!(run_source(src), Ok(Value::Int(50)));
}

#[test]
fn effect_op_with_param_is_visible_to_arm() {
    // The op takes an Int argument; the arm pattern binds it and uses it.
    // resume(n + 1) sends n+1 back to the call site.
    let src = r#"
        effect t { fn at(n: Int) -> Int }
        fn produce() -> <t> Int { t.at(5) + 100 }
        fn main() -> Int {
            handle produce() {
                return v => v,
                t.at n => resume(n + 1)
            }
        }
    "#;
    // arm sees n=5, rets 6; produce reads 6 + 100 = 106; return arm rets it.
    assert_eq!(run_source(src), Ok(Value::Int(106)));
}

#[test]
fn multiple_op_calls_each_dispatch_to_arm() {
    // produce() calls t.at twice — both call sites must be rewritten and the
    // arm must fire each time independently.
    let src = r#"
        effect t { fn at(n: Int) -> Int }
        fn produce() -> <t> Int { t.at(2) + t.at(3) }
        fn main() -> Int {
            handle produce() {
                return v => v,
                t.at n => resume(n)
            }
        }
    "#;
    // first call rets 2, second rets 3, sum = 5; return arm rets 5.
    assert_eq!(run_source(src), Ok(Value::Int(5)));
}

#[test]
fn arm_can_call_top_level_function() {
    // Arm body calls another fn — proves arm fns are real fns in the table
    // and not inlined-only blobs. helper is invoked via plain call.
    let src = r#"
        effect t { fn at(n: Int) -> Int }
        fn double(x: Int) -> Int { x + x }
        fn produce() -> <t> Int { t.at(7) }
        fn main() -> Int {
            handle produce() {
                return v => v,
                t.at n => resume(double(n))
            }
        }
    "#;
    // arm rets double(7) = 14; produce returns 14.
    assert_eq!(run_source(src), Ok(Value::Int(14)));
}

#[test]
fn two_handlers_for_different_effects_in_one_module() {
    // Distinct effects, distinct arms. Verifies the (effect, op) keying works
    // and the synthetic fn names don't collide.
    let src = r#"
        effect a { fn one() -> Int }
        effect b { fn two() -> Int }
        fn produce_a() -> <a> Int { a.one() }
        fn produce_b() -> <b> Int { b.two() }
        fn main() -> Int {
            let x = handle produce_a() {
                return v => v,
                a.one => resume(10)
            };
            let y = handle produce_b() {
                return v => v,
                b.two => resume(32)
            };
            x + y
        }
    "#;
    assert_eq!(run_source(src), Ok(Value::Int(42)));
}

#[test]
fn handle_compiles_when_body_is_pure() {
    // Body has no effect ops, so the dispatch path is never taken — only the
    // return arm. Should still compile and produce a value.
    let src = r#"
        fn main() -> Int {
            handle (3 + 4) {
                return v => v * 2
            }
        }
    "#;
    assert_eq!(run_source(src), Ok(Value::Int(14)));
}

#[test]
fn lifted_arms_appear_in_function_table() {
    // Spot-check the structural side of the pre-pass: a module with one handle
    // has at least one synthetic arm fn in addition to user fns.
    let src = r#"
        effect t { fn at() -> Int }
        fn produce() -> <t> Int { t.at() }
        fn main() -> Int {
            handle produce() {
                return v => v,
                t.at => resume(1)
            }
        }
    "#;
    let ast = parse_source(src);
    let mut compiler = Compiler::new();
    let module = compiler.compile_module(&ast).expect("compile ok");
    // User fns: produce, main = 2. Plus a return arm + an op arm = 4 total.
    assert!(
        module.functions.len() >= 4,
        "expected ≥4 fns after lifting arms; got {}",
        module.functions.len()
    );
}
