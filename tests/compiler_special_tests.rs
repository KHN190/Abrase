#[path = "compiler_codegen_common.rs"]
mod compiler_codegen_common;

use compiler_codegen_common::*;

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
fn verify_compile_string_interp_literal_only_runs() {
    let result = compile_module_and_run_string(&vec![Decl::Fn(FnDecl {
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
    })]);

    assert_eq!(result, Ok("hello".to_string()));
}

#[test]
fn verify_compile_string_interp_with_int_var_concat() {
    let main = Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        name: "main".to_string(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(Type::Named("String".to_string())),
        where_clause: vec![],
        body: Block {
            stmts: vec![Spanned {
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
            }],
            ret: Some(Box::new(Spanned {
                node: Expr::Literal(Literal::StringInterp(vec![
                    StringPart::Literal("n=".to_string()),
                    StringPart::Interp(vec!["x".to_string()]),
                ])),
                span: Span::new(0, 0),
            })),
        },
    });

    let result = compile_module_and_run_string(&vec![main]);
    assert_eq!(result, Ok("n=5".to_string()));
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
fn verify_codegen_emits_named_error_for_range() {
    let src = "fn main() -> Int { let _r = 0..5; 0 }";
    let result = run_source(src);
    assert!(result.is_err(), "expected NYI codegen error");
    let err = result.unwrap_err();
    assert!(err.contains("range expression") || err.contains("not yet implemented"),
            "expected named NYI message, got: {}", err);
}

#[test]
fn verify_codegen_emits_named_error_for_for_loop() {
    let src = "fn main() -> Int { for x in 0..5 { } 0 }";
    let result = run_source(src);
    assert!(result.is_err(), "expected error (Range is not iterable / NYI)");
    let err = result.unwrap_err();
    assert!(err.contains("for loop") || err.contains("range expression") || err.contains("not iterable"),
            "expected named NYI/iter message, got: {}", err);
}


#[test]
fn c1_expr_stmt_record_does_not_leak() {
    let src = r#"
        type Pt = { x: Int, y: Int }
        fn main() -> Int {
            Pt { x: 1, y: 2 };
            0
        }
    "#;
    let ast = parse_source(src);
    let (v, live) = compile_module_and_run_with_heap(&ast).unwrap();
    assert_eq!(v, Value::from_int(0));
    assert_eq!(live, 0, "expression-statement Pt should be dropped; heap_live_count={}", live);
}

#[test]
fn c1_expr_stmt_copy_type_emits_no_drop() {
    let src = r#"
        fn main() -> Int {
            1 + 2;
            7
        }
    "#;
    let ast = parse_source(src);
    let (v, live) = compile_module_and_run_with_heap(&ast).unwrap();
    assert_eq!(v, Value::from_int(7));
    assert_eq!(live, 0);
}

#[test]
fn verify_method_call_on_unknown_receiver_emits_method_error() {
    let src = "fn f(x: Int) -> Int { x.bogus() } fn main() -> Int { 0 }";
    let result = run_source(src);
    assert!(result.is_err(), "expected method-not-found error");
    let err = result.unwrap_err();
    assert!(err.contains("No method") || err.contains("bogus"),
            "expected method-specific error, got: {}", err);
}

