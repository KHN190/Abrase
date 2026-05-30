#[path = "compiler_codegen_common.rs"]
mod compiler_codegen_common;

use compiler_codegen_common::*;
use myriad::Value;

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
    assert_eq!(result, Ok(Value::from_int(5)));
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
    assert_eq!(result, Ok(Value::from_int(0)));
}

#[test]
fn verify_callreg_closure_call() {
    // A closure invocation emits CallReg (dynamic dispatch through a register).
    let src = "fn main() -> Int { let f = |y| y + 1; f(5) }";
    assert_eq!(run_source(src), Ok(Value::from_int(6)));
}

#[test]
fn verify_callreg_closure_captures_outer_binding() {
    let src = r#"fn main() -> Int { let n: Int = 10; let f = |y| n + y; f(5) }"#;
    assert_eq!(run_source(src), Ok(Value::from_int(15)));
}

#[test]
fn verify_callreg_first_class_function_passed_as_arg() {
    let src = r#"
        fn add_one(x: Int) -> Int { x + 1 }
        fn apply(f: fn(Int) -> Int, x: Int) -> Int { f(x) }
        fn main() -> Int { apply(add_one, 7) }
    "#;
    assert_eq!(run_source(src), Ok(Value::from_int(8)));
}

#[test]
fn verify_impl_method_lifted_to_synthetic_fn() {
    // impl-lift pass synthesizes Doubler__Int__double in the fn table for dispatch.
    let src = r#"
        trait Doubler {
            fn double(self) -> Int { 0 }
        }
        impl Doubler for Int {
            fn double(self) -> Int { self + 1 }
        }
        fn main() -> Int { (4).double() }
    "#;
    let ast = parse_source(src);
    let mut compiler = Compiler::new();
    let module = compiler.compile_module(&ast).expect("compile ok");
    let entry = compiler.method_dispatch.get(&("Int".to_string(), "double".to_string()));

    assert_eq!(entry, Some(&"Doubler__Int__double".to_string()),
        "method_dispatch should map (Int, double) to mangled name");

    assert!(module.functions.len() >= 2,
        "expected at least main + synthetic impl method; got {}",
        module.functions.len());
}
