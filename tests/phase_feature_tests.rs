use ect::ast::*;
use ect::compiler::Compiler;
use ect::vm::{Value, VirtualMachine};

fn compile_and_run(ast: &[Decl]) -> Result<Value, String> {
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

fn s(n: i32, c: i32) -> Span { Span::new(n as usize, c as usize) }
fn sp<T>(node: T) -> Spanned<T> { Spanned { node, span: s(0, 0) } }

mod phase_1_foundation {
    use super::*;

    #[test]
    fn feature_1_integer_literal() {
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
                ret: Some(Box::new(sp(Expr::Literal(Literal::Int(42))))),
            },
        })];
        assert_eq!(compile_and_run(&ast), Ok(Value::Int(42)));
    }

    #[test]
    fn feature_1_float_literal() {
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
                ret: Some(Box::new(sp(Expr::Literal(Literal::Float(3.14))))),
            },
        })];
        match compile_and_run(&ast) {
            Ok(Value::Float(f)) => assert!((f - 3.14).abs() < 0.001),
            other => panic!("Expected Float, got {:?}", other),
        }
    }

    #[test]
    fn feature_1_string_literal() {
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
                ret: Some(Box::new(sp(Expr::Literal(Literal::String("hello".to_string()))))),
            },
        })];
        assert_eq!(compile_and_run(&ast), Ok(Value::String("hello".to_string())));
    }

    #[test]
    fn feature_2_addition() {
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
                ret: Some(Box::new(sp(Expr::Binary {
                    op: BinaryOp::Add,
                    left: Box::new(sp(Expr::Literal(Literal::Int(3)))),
                    right: Box::new(sp(Expr::Literal(Literal::Int(4)))),
                }))),
            },
        })];
        assert_eq!(compile_and_run(&ast), Ok(Value::Int(7)));
    }

    #[test]
    fn feature_2_subtraction() {
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
                ret: Some(Box::new(sp(Expr::Binary {
                    op: BinaryOp::Sub,
                    left: Box::new(sp(Expr::Literal(Literal::Int(10)))),
                    right: Box::new(sp(Expr::Literal(Literal::Int(3)))),
                }))),
            },
        })];
        assert_eq!(compile_and_run(&ast), Ok(Value::Int(7)));
    }

    #[test]
    fn feature_2_multiplication() {
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
                ret: Some(Box::new(sp(Expr::Binary {
                    op: BinaryOp::Mul,
                    left: Box::new(sp(Expr::Literal(Literal::Int(6)))),
                    right: Box::new(sp(Expr::Literal(Literal::Int(7)))),
                }))),
            },
        })];
        assert_eq!(compile_and_run(&ast), Ok(Value::Int(42)));
    }

    #[test]
    fn feature_2_division() {
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
                ret: Some(Box::new(sp(Expr::Binary {
                    op: BinaryOp::Div,
                    left: Box::new(sp(Expr::Literal(Literal::Int(20)))),
                    right: Box::new(sp(Expr::Literal(Literal::Int(4)))),
                }))),
            },
        })];
        assert_eq!(compile_and_run(&ast), Ok(Value::Int(5)));
    }

    #[test]
    fn feature_2_modulo() {
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
                ret: Some(Box::new(sp(Expr::Binary {
                    op: BinaryOp::Mod,
                    left: Box::new(sp(Expr::Literal(Literal::Int(17)))),
                    right: Box::new(sp(Expr::Literal(Literal::Int(5)))),
                }))),
            },
        })];
        assert_eq!(compile_and_run(&ast), Ok(Value::Int(2)));
    }

}

mod phase_2_control_flow {
    use super::*;

    #[test]
    fn feature_4_if_true_branch() {
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
                ret: Some(Box::new(sp(Expr::If {
                    condition: Box::new(sp(Expr::Literal(Literal::Bool(true)))),
                    consequence: Box::new(sp(Expr::Literal(Literal::Int(10)))),
                    alternative: Some(Box::new(sp(Expr::Literal(Literal::Int(20))))),
                }))),
            },
        })];
        assert_eq!(compile_and_run(&ast), Ok(Value::Int(10)));
    }

    #[test]
    fn feature_4_if_false_branch() {
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
                ret: Some(Box::new(sp(Expr::If {
                    condition: Box::new(sp(Expr::Literal(Literal::Bool(false)))),
                    consequence: Box::new(sp(Expr::Literal(Literal::Int(10)))),
                    alternative: Some(Box::new(sp(Expr::Literal(Literal::Int(20))))),
                }))),
            },
        })];
        assert_eq!(compile_and_run(&ast), Ok(Value::Int(20)));
    }

    #[test]
    fn feature_4_if_without_else() {
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
                ret: Some(Box::new(sp(Expr::If {
                    condition: Box::new(sp(Expr::Literal(Literal::Bool(true)))),
                    consequence: Box::new(sp(Expr::Literal(Literal::Unit))),
                    alternative: None,
                }))),
            },
        })];
        assert_eq!(compile_and_run(&ast), Ok(Value::Unit));
    }

    #[test]
    fn feature_6_equality_true() {
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
                ret: Some(Box::new(sp(Expr::Binary {
                    op: BinaryOp::Eq,
                    left: Box::new(sp(Expr::Literal(Literal::Int(5)))),
                    right: Box::new(sp(Expr::Literal(Literal::Int(5)))),
                }))),
            },
        })];
        assert_eq!(compile_and_run(&ast), Ok(Value::Bool(true)));
    }

    #[test]
    fn feature_6_equality_false() {
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
                ret: Some(Box::new(sp(Expr::Binary {
                    op: BinaryOp::Eq,
                    left: Box::new(sp(Expr::Literal(Literal::Int(3)))),
                    right: Box::new(sp(Expr::Literal(Literal::Int(5)))),
                }))),
            },
        })];
        assert_eq!(compile_and_run(&ast), Ok(Value::Bool(false)));
    }

    #[test]
    fn feature_6_less_than() {
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
                ret: Some(Box::new(sp(Expr::Binary {
                    op: BinaryOp::Lt,
                    left: Box::new(sp(Expr::Literal(Literal::Int(3)))),
                    right: Box::new(sp(Expr::Literal(Literal::Int(5)))),
                }))),
            },
        })];
        assert_eq!(compile_and_run(&ast), Ok(Value::Bool(true)));
    }

    #[test]
    fn feature_6_greater_than() {
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
                ret: Some(Box::new(sp(Expr::Binary {
                    op: BinaryOp::Gt,
                    left: Box::new(sp(Expr::Literal(Literal::Int(5)))),
                    right: Box::new(sp(Expr::Literal(Literal::Int(3)))),
                }))),
            },
        })];
        assert_eq!(compile_and_run(&ast), Ok(Value::Bool(true)));
    }

}

mod phase_3_functions {
    use super::*;

    #[test]
    fn feature_8_simple_function_call() {
        let ast = vec![
            Decl::Fn(FnDecl {
                attrs: vec![],
                is_pub: false,
                name: "add".to_string(),
                generics: vec![],
                params: vec![
                    Param::Named {
                        pattern: sp(Pattern::Bind("a".to_string())),
                        ty: Type::Named("Int".to_string()),
                    },
                    Param::Named {
                        pattern: sp(Pattern::Bind("b".to_string())),
                        ty: Type::Named("Int".to_string()),
                    },
                ],
                effects: vec![],
                return_type: Some(Type::Named("Int".to_string())),
                where_clause: vec![],
                body: Block {
                    stmts: vec![],
                    ret: Some(Box::new(sp(Expr::Binary {
                        op: BinaryOp::Add,
                        left: Box::new(sp(Expr::Identifier("a".to_string()))),
                        right: Box::new(sp(Expr::Identifier("b".to_string()))),
                    }))),
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
                    ret: Some(Box::new(sp(Expr::Call {
                        callee: Box::new(sp(Expr::Identifier("add".to_string()))),
                        args: vec![
                            sp(Expr::Literal(Literal::Int(3))),
                            sp(Expr::Literal(Literal::Int(4))),
                        ],
                    }))),
                },
            }),
        ];
        assert_eq!(compile_and_run(&ast), Ok(Value::Int(7)));
    }

    #[test]
    fn feature_10_recursive_factorial() {
        let ast = vec![
            Decl::Fn(FnDecl {
                attrs: vec![],
                is_pub: false,
                name: "fact".to_string(),
                generics: vec![],
                params: vec![Param::Named {
                    pattern: sp(Pattern::Bind("n".to_string())),
                    ty: Type::Named("Int".to_string()),
                }],
                effects: vec![],
                return_type: Some(Type::Named("Int".to_string())),
                where_clause: vec![],
                body: Block {
                    stmts: vec![],
                    ret: Some(Box::new(sp(Expr::If {
                        condition: Box::new(sp(Expr::Binary {
                            op: BinaryOp::Lte,
                            left: Box::new(sp(Expr::Identifier("n".to_string()))),
                            right: Box::new(sp(Expr::Literal(Literal::Int(1)))),
                        })),
                        consequence: Box::new(sp(Expr::Literal(Literal::Int(1)))),
                        alternative: Some(Box::new(sp(Expr::Binary {
                            op: BinaryOp::Mul,
                            left: Box::new(sp(Expr::Identifier("n".to_string()))),
                            right: Box::new(sp(Expr::Call {
                                callee: Box::new(sp(Expr::Identifier("fact".to_string()))),
                                args: vec![sp(Expr::Binary {
                                    op: BinaryOp::Sub,
                                    left: Box::new(sp(Expr::Identifier("n".to_string()))),
                                    right: Box::new(sp(Expr::Literal(Literal::Int(1)))),
                                })],
                            })),
                        }))),
                    }))),
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
                    ret: Some(Box::new(sp(Expr::Call {
                        callee: Box::new(sp(Expr::Identifier("fact".to_string()))),
                        args: vec![sp(Expr::Literal(Literal::Int(5)))],
                    }))),
                },
            }),
        ];
        assert_eq!(compile_and_run(&ast), Ok(Value::Int(120)));
    }
}

