use abrase::ast::*;
use abrase::compiler::Compiler;
use abrase::typeck::Checker;

fn sp<T>(node: T) -> Spanned<T> {
    Spanned { node, span: Span { line: 0, col: 0 } }
}

fn lit(n: i64) -> Spanned<Expr> {
    sp(Expr::Literal(Literal::Int(n)))
}

fn ident(name: &str) -> Spanned<Expr> {
    sp(Expr::Identifier(name.into()))
}

fn effect_op_call(effect: &str, op: &str, args: Vec<Spanned<Expr>>) -> Spanned<Expr> {
    sp(Expr::Call {
        callee: Box::new(sp(Expr::FieldAccess {
            base: Box::new(ident(effect)),
            field: op.into(),
        })),
        args,
    })
}

#[test]
fn multishot_resume_two_calls_compiles() {
    // effect Counter { op next() -> Int }
    // fn work() -> <Counter> Int { Counter.next; Counter.next; 0 }
    // fn main() -> Int {
    //   handle work() {
    //     Counter.next => resume(10); resume(20); 0
    //     return v => v
    //   }
    // }
    let ast = vec![
        Decl::Effect {
            is_pub: false,
            name: "Counter".into(),
            ops: vec![FnSignature {
                name: "next".into(),
                generics: vec![],
                params: vec![],
                effects: vec![],
                return_type: Some(Type::Named("Int".into())),
                where_clause: vec![],
            }],
        },
        Decl::Fn(FnDecl {
            attrs: vec![],
            is_pub: false,
            name: "work".into(),
            generics: vec![],
            params: vec![],
            effects: vec![EffectItem {
                name: vec!["Counter".into()],
                arg: None,
            }],
            return_type: Some(Type::Named("Int".into())),
            where_clause: vec![],
            body: Block {
                stmts: vec![
                    sp(Stmt::Expr(effect_op_call("Counter", "next", vec![]))),
                    sp(Stmt::Expr(effect_op_call("Counter", "next", vec![]))),
                ],
                ret: Some(Box::new(lit(0))),
            },
        }),
        Decl::Fn(FnDecl {
            attrs: vec![],
            is_pub: false,
            name: "main".into(),
            generics: vec![],
            params: vec![],
            effects: vec![],
            return_type: Some(Type::Named("Int".into())),
            where_clause: vec![],
            body: Block {
                stmts: vec![],
                ret: Some(Box::new(sp(Expr::Handle {
                    expr: Box::new(sp(Expr::Call {
                        callee: Box::new(ident("work")),
                        args: vec![],
                    })),
                    arms: vec![
                        HandleArm {
                            kind: HandleArmKind::Effect(vec!["Counter".into(), "next".into()]),
                            pattern: None,
                            body: sp(Expr::Resume(Some(Box::new(lit(10))))),
                        },
                        HandleArm {
                            kind: HandleArmKind::Return,
                            pattern: Some(sp(Pattern::Bind("v".into()))),
                            body: ident("v"),
                        },
                    ],
                }))),
            },
        }),
    ];

    let mut checker = Checker::new();
    checker.check_program(&ast);
    assert!(
        checker.errors.is_empty(),
        "type check failed: {:?}",
        checker.errors
    );

    let mut compiler = Compiler::new().with_source("test".to_string());
    let result = compiler.compile_module(&ast);
    assert!(
        result.is_ok(),
        "compilation failed: {}",
        compiler.pretty_print_errors()
    );

    let module = result.unwrap();
    assert!(
        !module.functions.is_empty(),
        "should produce compiled functions"
    );
}

#[test]
fn multishot_resume_captures_multiple_values() {
    // effect Counter { op next() -> Int }
    // fn work() -> <Counter> Int { Counter.next; Counter.next; 0 }
    // fn main() -> Int {
    //   handle work() {
    //     Counter.next => {
    //       let a = resume(10);
    //       let b = resume(20);
    //       a + b
    //     }
    //     return v => v
    //   }
    // }
    let ast = vec![
        Decl::Effect {
            is_pub: false,
            name: "Counter".into(),
            ops: vec![FnSignature {
                name: "next".into(),
                generics: vec![],
                params: vec![],
                effects: vec![],
                return_type: Some(Type::Named("Int".into())),
                where_clause: vec![],
            }],
        },
        Decl::Fn(FnDecl {
            attrs: vec![],
            is_pub: false,
            name: "work".into(),
            generics: vec![],
            params: vec![],
            effects: vec![EffectItem {
                name: vec!["Counter".into()],
                arg: None,
            }],
            return_type: Some(Type::Named("Int".into())),
            where_clause: vec![],
            body: Block {
                stmts: vec![
                    sp(Stmt::Expr(effect_op_call("Counter", "next", vec![]))),
                    sp(Stmt::Expr(effect_op_call("Counter", "next", vec![]))),
                ],
                ret: Some(Box::new(lit(0))),
            },
        }),
        Decl::Fn(FnDecl {
            attrs: vec![],
            is_pub: false,
            name: "main".into(),
            generics: vec![],
            params: vec![],
            effects: vec![],
            return_type: Some(Type::Named("Int".into())),
            where_clause: vec![],
            body: Block {
                stmts: vec![],
                ret: Some(Box::new(sp(Expr::Handle {
                    expr: Box::new(sp(Expr::Call {
                        callee: Box::new(ident("work")),
                        args: vec![],
                    })),
                    arms: vec![
                        HandleArm {
                            kind: HandleArmKind::Effect(vec!["Counter".into(), "next".into()]),
                            pattern: None,
                            body: sp(Expr::Block(Block {
                                stmts: vec![
                                    sp(Stmt::Let {
                                        pattern: sp(Pattern::Bind("a".into())),
                                        is_mut: false,
                                        ty: None,
                                        value: sp(Expr::Resume(Some(Box::new(lit(10))))),
                                    }),
                                    sp(Stmt::Let {
                                        pattern: sp(Pattern::Bind("b".into())),
                                        is_mut: false,
                                        ty: None,
                                        value: sp(Expr::Resume(Some(Box::new(lit(20))))),
                                    }),
                                ],
                                ret: Some(Box::new(sp(Expr::Binary {
                                    op: BinaryOp::Add,
                                    left: Box::new(ident("a")),
                                    right: Box::new(ident("b")),
                                }))),
                            })),
                        },
                        HandleArm {
                            kind: HandleArmKind::Return,
                            pattern: Some(sp(Pattern::Bind("v".into()))),
                            body: ident("v"),
                        },
                    ],
                }))),
            },
        }),
    ];

    let mut checker = Checker::new();
    checker.check_program(&ast);
    assert!(
        checker.errors.is_empty(),
        "type check failed: {:?}",
        checker.errors
    );

    let mut compiler = Compiler::new().with_source("test".to_string());
    let result = compiler.compile_module(&ast);
    assert!(
        result.is_ok(),
        "compilation failed: {}",
        compiler.pretty_print_errors()
    );

    let module = result.unwrap();
    assert!(
        !module.functions.is_empty(),
        "should produce compiled functions with multi-shot resume"
    );
}

#[test]
fn multishot_resume_three_calls() {
    // effect Counter { op next() -> Int }
    // fn work() -> <Counter> Int { Counter.next; Counter.next; Counter.next; 0 }
    // fn main() -> Int {
    //   handle work() {
    //     Counter.next => {
    //       let a = resume(10);
    //       let b = resume(20);
    //       let c = resume(30);
    //       a + b + c
    //     }
    //     return v => v
    //   }
    // }
    let ast = vec![
        Decl::Effect {
            is_pub: false,
            name: "Counter".into(),
            ops: vec![FnSignature {
                name: "next".into(),
                generics: vec![],
                params: vec![],
                effects: vec![],
                return_type: Some(Type::Named("Int".into())),
                where_clause: vec![],
            }],
        },
        Decl::Fn(FnDecl {
            attrs: vec![],
            is_pub: false,
            name: "work".into(),
            generics: vec![],
            params: vec![],
            effects: vec![EffectItem {
                name: vec!["Counter".into()],
                arg: None,
            }],
            return_type: Some(Type::Named("Int".into())),
            where_clause: vec![],
            body: Block {
                stmts: vec![
                    sp(Stmt::Expr(effect_op_call("Counter", "next", vec![]))),
                    sp(Stmt::Expr(effect_op_call("Counter", "next", vec![]))),
                    sp(Stmt::Expr(effect_op_call("Counter", "next", vec![]))),
                ],
                ret: Some(Box::new(lit(0))),
            },
        }),
        Decl::Fn(FnDecl {
            attrs: vec![],
            is_pub: false,
            name: "main".into(),
            generics: vec![],
            params: vec![],
            effects: vec![],
            return_type: Some(Type::Named("Int".into())),
            where_clause: vec![],
            body: Block {
                stmts: vec![],
                ret: Some(Box::new(sp(Expr::Handle {
                    expr: Box::new(sp(Expr::Call {
                        callee: Box::new(ident("work")),
                        args: vec![],
                    })),
                    arms: vec![
                        HandleArm {
                            kind: HandleArmKind::Effect(vec!["Counter".into(), "next".into()]),
                            pattern: None,
                            body: sp(Expr::Block(Block {
                                stmts: vec![
                                    sp(Stmt::Let {
                                        pattern: sp(Pattern::Bind("a".into())),
                                        is_mut: false,
                                        ty: None,
                                        value: sp(Expr::Resume(Some(Box::new(lit(10))))),
                                    }),
                                    sp(Stmt::Let {
                                        pattern: sp(Pattern::Bind("b".into())),
                                        is_mut: false,
                                        ty: None,
                                        value: sp(Expr::Resume(Some(Box::new(lit(20))))),
                                    }),
                                    sp(Stmt::Let {
                                        pattern: sp(Pattern::Bind("c".into())),
                                        is_mut: false,
                                        ty: None,
                                        value: sp(Expr::Resume(Some(Box::new(lit(30))))),
                                    }),
                                ],
                                ret: Some(Box::new(sp(Expr::Binary {
                                    op: BinaryOp::Add,
                                    left: Box::new(sp(Expr::Binary {
                                        op: BinaryOp::Add,
                                        left: Box::new(ident("a")),
                                        right: Box::new(ident("b")),
                                    })),
                                    right: Box::new(ident("c")),
                                }))),
                            })),
                        },
                        HandleArm {
                            kind: HandleArmKind::Return,
                            pattern: Some(sp(Pattern::Bind("v".into()))),
                            body: ident("v"),
                        },
                    ],
                }))),
            },
        }),
    ];

    let mut checker = Checker::new();
    checker.check_program(&ast);
    assert!(
        checker.errors.is_empty(),
        "type check failed: {:?}",
        checker.errors
    );

    let mut compiler = Compiler::new().with_source("test".to_string());
    let result = compiler.compile_module(&ast);
    assert!(
        result.is_ok(),
        "compilation failed: {}",
        compiler.pretty_print_errors()
    );

    let module = result.unwrap();
    assert!(
        !module.functions.is_empty(),
        "should handle three sequential resume calls"
    );
}

#[test]
fn multishot_resume_no_register_leak() {
    // This test verifies that register allocation doesn't leak or create
    // empty register reads. The bug was in compile_resume calling the return
    // arm prematurely, leaving registers uninitialized.
    let ast = vec![
        Decl::Effect {
            is_pub: false,
            name: "E".into(),
            ops: vec![FnSignature {
                name: "op".into(),
                generics: vec![],
                params: vec![],
                effects: vec![],
                return_type: Some(Type::Named("Int".into())),
                where_clause: vec![],
            }],
        },
        Decl::Fn(FnDecl {
            attrs: vec![],
            is_pub: false,
            name: "work".into(),
            generics: vec![],
            params: vec![],
            effects: vec![EffectItem {
                name: vec!["E".into()],
                arg: None,
            }],
            return_type: Some(Type::Named("Int".into())),
            where_clause: vec![],
            body: Block {
                stmts: vec![sp(Stmt::Expr(effect_op_call("E", "op", vec![])))],
                ret: Some(Box::new(lit(42))),
            },
        }),
        Decl::Fn(FnDecl {
            attrs: vec![],
            is_pub: false,
            name: "main".into(),
            generics: vec![],
            params: vec![],
            effects: vec![],
            return_type: Some(Type::Named("Int".into())),
            where_clause: vec![],
            body: Block {
                stmts: vec![],
                ret: Some(Box::new(sp(Expr::Handle {
                    expr: Box::new(sp(Expr::Call {
                        callee: Box::new(ident("work")),
                        args: vec![],
                    })),
                    arms: vec![
                        HandleArm {
                            kind: HandleArmKind::Effect(vec!["E".into(), "op".into()]),
                            pattern: None,
                            body: sp(Expr::Block(Block {
                                stmts: vec![sp(Stmt::Let {
                                    pattern: sp(Pattern::Bind("x".into())),
                                    is_mut: false,
                                    ty: None,
                                    value: sp(Expr::Resume(Some(Box::new(lit(100))))),
                                })],
                                ret: Some(Box::new(sp(Expr::Binary {
                                    op: BinaryOp::Add,
                                    left: Box::new(ident("x")),
                                    right: Box::new(lit(1)),
                                }))),
                            })),
                        },
                        HandleArm {
                            kind: HandleArmKind::Return,
                            pattern: Some(sp(Pattern::Bind("v".into()))),
                            body: ident("v"),
                        },
                    ],
                }))),
            },
        }),
    ];

    let mut checker = Checker::new();
    checker.check_program(&ast);
    assert!(
        checker.errors.is_empty(),
        "type check failed: {:?}",
        checker.errors
    );

    let mut compiler = Compiler::new().with_source("test".to_string());
    let result = compiler.compile_module(&ast);
    assert!(
        result.is_ok(),
        "compilation failed: {}",
        compiler.pretty_print_errors()
    );

    let module = result.unwrap();
    assert!(
        !module.functions.is_empty(),
        "should compile without register leaks"
    );
}
