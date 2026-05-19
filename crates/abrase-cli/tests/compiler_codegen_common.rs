#![allow(dead_code)]

pub use abrase::ast::*;
pub use abrase::compiler::Compiler;
pub use abrase::lexer::Lexer;
pub use abrase::parser::Parser;
pub use myriad::{Value, VirtualMachine, read_string};

pub fn compile_and_run(ast: &[Decl]) -> Result<Value, String> {
    let mut compiler = Compiler::new();
    let chunk = compiler.compile(ast)?;
    let mut vm = VirtualMachine::new();
    vm.run(&chunk)
}

pub fn compile_and_run_string(ast: &[Decl]) -> Result<String, String> {
    let mut compiler = Compiler::new();
    let chunk = compiler.compile(ast)?;
    let mut vm = VirtualMachine::new();
    let v = vm.run(&chunk)?;
    extract_string(&vm, v)
}

pub fn run_source_string(src: &str) -> Result<String, String> {
    let ast = parse_source(src);
    let mut compiler = Compiler::new();
    let module = compiler.compile_module(&ast).map_err(|errs| {
        errs.iter().map(|e| format!("{:?}: {}", e.code, e.message))
            .collect::<Vec<_>>().join("\n")
    })?;
    let mut vm = VirtualMachine::new();
    let v = vm.run_module(&module)?;
    extract_string(&vm, v)
}

pub fn extract_string(vm: &VirtualMachine, v: Value) -> Result<String, String> {
    read_string(vm.heap_ref(), v).ok_or_else(|| format!("expected String handle, got {:?}", v))
}

pub fn compile_module_and_run_string(ast: &[Decl]) -> Result<String, String> {
    let mut compiler = Compiler::new();
    let module = compiler.compile_module(ast).map_err(|errs| {
        errs.iter()
            .map(|e| format!("{:?} at {}:{}: {}", e.code, e.span.line, e.span.col, e.message))
            .collect::<Vec<_>>()
            .join("\n")
    })?;
    let mut vm = VirtualMachine::new();
    let v = vm.run_module(&module)?;
    extract_string(&vm, v)
}

pub fn compile_module_and_run(ast: &[Decl]) -> Result<Value, String> {
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

pub fn compile_module_and_run_with_heap(ast: &[Decl]) -> Result<(Value, usize), String> {
    let mut compiler = Compiler::new();
    let module = compiler.compile_module(ast).map_err(|errs| {
        errs.iter()
            .map(|e| format!("{:?} at {}:{}: {}", e.code, e.span.line, e.span.col, e.message))
            .collect::<Vec<_>>()
            .join("\n")
    })?;
    let mut vm = VirtualMachine::new();
    let v = vm.run_module(&module)?;
    Ok((v, vm.heap_live_count()))
}

pub fn parse_literal_int(n: i64) -> Vec<Decl> {
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

pub fn parse_binary_int(left: i64, op: BinaryOp, right: i64) -> Vec<Decl> {
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

pub fn parse_arithmetic_expr() -> Vec<Decl> {
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

pub fn parse_let_expr() -> Vec<Decl> {
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

pub fn parse_source(src: &str) -> Vec<Decl> {
    let mut p = Parser::new(Lexer::new(src)).with_source(src.to_string());
    let decls = p.parse_program();
    assert!(p.errors.is_empty(), "parse errors: {:?}", p.errors);
    decls
}

pub fn run_source(src: &str) -> Result<Value, String> {
    let ast = parse_source(src);
    compile_module_and_run(&ast)
}
