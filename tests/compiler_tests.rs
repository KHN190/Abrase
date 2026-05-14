// Compiler tests: code generation, optimization, phases
use ect::ast::*;
use ect::compiler::Compiler;
use ect::vm::{Value, VirtualMachine};

fn compile_and_run(ast: &[Decl]) -> Result<Value, String> {
    let mut compiler = Compiler::new();
    let chunk = compiler.compile(ast)?;
    let mut vm = VirtualMachine::new();
    vm.run(&chunk)
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

// Phase 1: Feature 1 - Literals & Constants
#[test]
fn test_phase1_feature1_literal_int() {
    let ast = parse_literal_int(42);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::Int(42));
}

#[test]
fn test_phase1_feature1_literal_bool_true() {
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
fn test_phase1_feature1_literal_string() {
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

// Phase 1: Feature 2 - Arithmetic (Int)
#[test]
fn test_phase1_feature2_add() {
    let ast = parse_binary_int(2, BinaryOp::Add, 3);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::Int(5));
}

#[test]
fn test_phase1_feature2_sub() {
    let ast = parse_binary_int(10, BinaryOp::Sub, 3);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::Int(7));
}

#[test]
fn test_phase1_feature2_mul() {
    let ast = parse_binary_int(3, BinaryOp::Mul, 4);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::Int(12));
}

#[test]
fn test_phase1_feature2_div() {
    let ast = parse_binary_int(20, BinaryOp::Div, 4);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::Int(5));
}

#[test]
fn test_phase1_feature2_mod() {
    let ast = parse_binary_int(10, BinaryOp::Mod, 3);
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::Int(1));
}

#[test]
fn test_phase1_feature2_complex_expression() {
    // 2 + 3 * 4 = 2 + 12 = 14
    let ast = parse_arithmetic_expr();
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::Int(14));
}

// Phase 1: Feature 3 - Variables & Assignment
#[test]
fn test_phase1_feature3_let_and_use() {
    // let x = 5; let y = x + 1; y
    let ast = parse_let_expr();
    let result = compile_and_run(&ast).expect("Execution failed");
    assert_eq!(result, Value::Int(6));
}

#[test]
fn test_phase1_feature3_multiple_let() {
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

// Regression: Ensure Phase 1 milestones
#[test]
fn test_phase1_milestone_pure_arithmetic() {
    // Verify we can run pure functional arithmetic
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
