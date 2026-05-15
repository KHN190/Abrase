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
