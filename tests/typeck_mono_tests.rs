use abrase::ast::*;
use abrase::compiler::Compiler;
use abrase::vm::{Value, VirtualMachine};

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
fn gp(name: &str) -> GenericParam { GenericParam { name: name.into() } }

fn fn_main_returns_int(call: Expr) -> Decl {
    Decl::Fn(FnDecl {
        attrs: vec![],
        is_pub: false,
        name: "main".into(),
        generics: vec![],
        params: vec![],
        effects: vec![],
        return_type: Some(Type::Named("Int".into())),
        where_clause: vec![],
        body: Block { stmts: vec![], ret: Some(Box::new(sp(call))) },
    })
}

fn compile_module_errors(ast: &[Decl]) -> Vec<String> {
    let mut compiler = abrase::compiler::Compiler::new();
    match compiler.compile_module(ast) {
        Ok(_) => Vec::new(),
        Err(es) => es.into_iter().map(|e| e.message).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // fn id<T>(x: T) -> T { x }
    // fn main() -> Int { id(5) }
    #[test]
    fn generic_identity_function_int() {
        let ast = vec![
            Decl::Fn(FnDecl {
                attrs: vec![],
                is_pub: false,
                name: "id".into(),
                generics: vec![gp("T")],
                params: vec![Param::Named {
                    pattern: sp(Pattern::Bind("x".into())),
                    ty: Type::Named("T".into()),
                }],
                effects: vec![],
                return_type: Some(Type::Named("T".into())),
                where_clause: vec![],
                body: Block {
                    stmts: vec![],
                    ret: Some(Box::new(sp(Expr::Identifier("x".into())))),
                },
            }),
            fn_main_returns_int(Expr::Call {
                callee: Box::new(sp(Expr::Identifier("id".into()))),
                args: vec![sp(Expr::Literal(Literal::Int(5)))],
            }),
        ];
        assert_eq!(compile_and_run(&ast), Ok(Value::Int(5)));
    }

    // Two specialisations of `id` coexist: id(5) and id(true), summed via Int branch.
    #[test]
    fn generic_specializes_for_different_types() {
        let id_fn = Decl::Fn(FnDecl {
            attrs: vec![],
            is_pub: false,
            name: "id".into(),
            generics: vec![gp("T")],
            params: vec![Param::Named {
                pattern: sp(Pattern::Bind("x".into())),
                ty: Type::Named("T".into()),
            }],
            effects: vec![],
            return_type: Some(Type::Named("T".into())),
            where_clause: vec![],
            body: Block {
                stmts: vec![],
                ret: Some(Box::new(sp(Expr::Identifier("x".into())))),
            },
        });
        // main: if id(true) { id(10) } else { 0 }  -> 10
        let main_fn = fn_main_returns_int(Expr::If {
            condition: Box::new(sp(Expr::Call {
                callee: Box::new(sp(Expr::Identifier("id".into()))),
                args: vec![sp(Expr::Literal(Literal::Bool(true)))],
            })),
            consequence: Box::new(sp(Expr::Call {
                callee: Box::new(sp(Expr::Identifier("id".into()))),
                args: vec![sp(Expr::Literal(Literal::Int(10)))],
            })),
            alternative: Some(Box::new(sp(Expr::Literal(Literal::Int(0))))),
        });
        let ast = vec![id_fn, main_fn];
        assert_eq!(compile_and_run(&ast), Ok(Value::Int(10)));
    }

    // fn outer<T>(x: T) -> T { inner(x) }
    // fn inner<T>(x: T) -> T { x }
    // fn main() -> Int { outer(7) }
    #[test]
    fn generic_infers_through_transitive_calls() {
        let inner_fn = Decl::Fn(FnDecl {
            attrs: vec![],
            is_pub: false,
            name: "inner".into(),
            generics: vec![gp("T")],
            params: vec![Param::Named {
                pattern: sp(Pattern::Bind("x".into())),
                ty: Type::Named("T".into()),
            }],
            effects: vec![],
            return_type: Some(Type::Named("T".into())),
            where_clause: vec![],
            body: Block {
                stmts: vec![],
                ret: Some(Box::new(sp(Expr::Identifier("x".into())))),
            },
        });
        let outer_fn = Decl::Fn(FnDecl {
            attrs: vec![],
            is_pub: false,
            name: "outer".into(),
            generics: vec![gp("T")],
            params: vec![Param::Named {
                pattern: sp(Pattern::Bind("x".into())),
                ty: Type::Named("T".into()),
            }],
            effects: vec![],
            return_type: Some(Type::Named("T".into())),
            where_clause: vec![],
            body: Block {
                stmts: vec![],
                ret: Some(Box::new(sp(Expr::Call {
                    callee: Box::new(sp(Expr::Identifier("inner".into()))),
                    args: vec![sp(Expr::Identifier("x".into()))],
                }))),
            },
        });
        let main_fn = fn_main_returns_int(Expr::Call {
            callee: Box::new(sp(Expr::Identifier("outer".into()))),
            args: vec![sp(Expr::Literal(Literal::Int(7)))],
        });
        let ast = vec![inner_fn, outer_fn, main_fn];
        assert_eq!(compile_and_run(&ast), Ok(Value::Int(7)));
    }

    // fn cnt<T>(x: T, n: Int) -> Int { if n <= 0 { 0 } else { 1 + cnt(x, n - 1) } }
    // fn main() -> Int { cnt(42, 3) }
    // Exercises recursion through a generic specialization (cnt__Int) — the
    // recursive call gets rewritten to the mangled name.
    #[test]
    fn generic_function_recurses_with_specialization() {
        let cnt = Decl::Fn(FnDecl {
            attrs: vec![],
            is_pub: false,
            name: "cnt".into(),
            generics: vec![gp("T")],
            params: vec![
                Param::Named {
                    pattern: sp(Pattern::Bind("x".into())),
                    ty: Type::Named("T".into()),
                },
                Param::Named {
                    pattern: sp(Pattern::Bind("n".into())),
                    ty: Type::Named("Int".into()),
                },
            ],
            effects: vec![],
            return_type: Some(Type::Named("Int".into())),
            where_clause: vec![],
            body: Block {
                stmts: vec![],
                ret: Some(Box::new(sp(Expr::If {
                    condition: Box::new(sp(Expr::Binary {
                        op: BinaryOp::Lte,
                        left: Box::new(sp(Expr::Identifier("n".into()))),
                        right: Box::new(sp(Expr::Literal(Literal::Int(0)))),
                    })),
                    consequence: Box::new(sp(Expr::Literal(Literal::Int(0)))),
                    alternative: Some(Box::new(sp(Expr::Binary {
                        op: BinaryOp::Add,
                        left: Box::new(sp(Expr::Literal(Literal::Int(1)))),
                        right: Box::new(sp(Expr::Call {
                            callee: Box::new(sp(Expr::Identifier("cnt".into()))),
                            args: vec![
                                sp(Expr::Identifier("x".into())),
                                sp(Expr::Binary {
                                    op: BinaryOp::Sub,
                                    left: Box::new(sp(Expr::Identifier("n".into()))),
                                    right: Box::new(sp(Expr::Literal(Literal::Int(1)))),
                                }),
                            ],
                        })),
                    }))),
                }))),
            },
        });
        let main_fn = fn_main_returns_int(Expr::Call {
            callee: Box::new(sp(Expr::Identifier("cnt".into()))),
            args: vec![
                sp(Expr::Literal(Literal::Int(42))),
                sp(Expr::Literal(Literal::Int(3))),
            ],
        });
        let ast = vec![cnt, main_fn];
        assert_eq!(compile_and_run(&ast), Ok(Value::Int(3)));
    }

    // fn make<T>(x: Int) -> Int { x }  -- T is unused, so id type-arg cannot be inferred
    // fn main() -> Int { make(5) }
    #[test]
    fn generic_type_inference_fails_when_unused() {
        let make_fn = Decl::Fn(FnDecl {
            attrs: vec![],
            is_pub: false,
            name: "make".into(),
            generics: vec![gp("T")],
            params: vec![Param::Named {
                pattern: sp(Pattern::Bind("x".into())),
                ty: Type::Named("Int".into()),
            }],
            effects: vec![],
            return_type: Some(Type::Named("Int".into())),
            where_clause: vec![],
            body: Block {
                stmts: vec![],
                ret: Some(Box::new(sp(Expr::Identifier("x".into())))),
            },
        });
        let main_fn = fn_main_returns_int(Expr::Call {
            callee: Box::new(sp(Expr::Identifier("make".into()))),
            args: vec![sp(Expr::Literal(Literal::Int(5)))],
        });
        let ast = vec![make_fn, main_fn];
        let errs = compile_module_errors(&ast);
        assert!(
            errs.iter().any(|m| m.contains("Cannot infer type parameter")
                && m.contains("'T'")
                && m.contains("'make'")),
            "expected inference-failure error mentioning T and make, got: {:?}",
            errs,
        );
    }

    // Feature 22: trait dispatch via static monomorphisation.
    //
    // Uses a `Doubler` trait so the tests do not collide with the four
    // reserved trait names (`Show`/`Eq`/`Ord`/`Clone`).

    fn doubler_trait(return_ty: Type) -> Decl {
        Decl::Trait {
            is_pub: false,
            name: "Doubler".into(),
            generics: vec![],
            where_clause: vec![],
            items: vec![TraitItem::Required(FnSignature {
                name: "double".into(),
                generics: vec![],
                params: vec![Param::SelfVal],
                effects: vec![],
                return_type: Some(return_ty),
                where_clause: vec![],
            })],
        }
    }

    fn impl_doubler_for(target: &str, return_ty: Type, body: Block) -> Decl {
        Decl::Impl {
            generics: vec![],
            trait_name: Some(vec!["Doubler".into()]),
            for_type: Type::Named(target.into()),
            where_clause: vec![],
            methods: vec![FnDecl {
                attrs: vec![],
                is_pub: false,
                name: "double".into(),
                generics: vec![],
                params: vec![Param::SelfVal],
                effects: vec![],
                return_type: Some(return_ty),
                where_clause: vec![],
                body,
            }],
        }
    }

    #[test]
    fn feature_22_static_dispatch_to_int_impl() {
        let trait_decl = doubler_trait(Type::Named("Int".into()));
        let body = Block {
            stmts: vec![],
            ret: Some(Box::new(sp(Expr::Binary {
                op: BinaryOp::Mul,
                left: Box::new(sp(Expr::Identifier("self".into()))),
                right: Box::new(sp(Expr::Literal(Literal::Int(2)))),
            }))),
        };
        let impl_decl = impl_doubler_for("Int", Type::Named("Int".into()), body);
        let main_fn = fn_main_returns_int(Expr::Call {
            callee: Box::new(sp(Expr::FieldAccess {
                base: Box::new(sp(Expr::Literal(Literal::Int(5)))),
                field: "double".into(),
            })),
            args: vec![],
        });
        let ast = vec![trait_decl, impl_decl, main_fn];
        assert_eq!(compile_and_run(&ast), Ok(Value::Int(10)));
    }

    #[test]
    fn feature_22_trait_method_used_in_generic_body() {
        let trait_decl = doubler_trait(Type::Named("Int".into()));
        let body = Block {
            stmts: vec![],
            ret: Some(Box::new(sp(Expr::Binary {
                op: BinaryOp::Add,
                left: Box::new(sp(Expr::Identifier("self".into()))),
                right: Box::new(sp(Expr::Literal(Literal::Int(1)))),
            }))),
        };
        let impl_decl = impl_doubler_for("Int", Type::Named("Int".into()), body);
        // fn p<T>(x: T) -> Int where T: Doubler { x.double() }
        let p_fn = Decl::Fn(FnDecl {
            attrs: vec![],
            is_pub: false,
            name: "p".into(),
            generics: vec![gp("T")],
            params: vec![Param::Named {
                pattern: sp(Pattern::Bind("x".into())),
                ty: Type::Named("T".into()),
            }],
            effects: vec![],
            return_type: Some(Type::Named("Int".into())),
            where_clause: vec![WhereBound {
                ty: Type::Named("T".into()),
                bounds: vec![vec!["Doubler".into()]],
            }],
            body: Block {
                stmts: vec![],
                ret: Some(Box::new(sp(Expr::Call {
                    callee: Box::new(sp(Expr::FieldAccess {
                        base: Box::new(sp(Expr::Identifier("x".into()))),
                        field: "double".into(),
                    })),
                    args: vec![],
                }))),
            },
        });
        let main_fn = fn_main_returns_int(Expr::Call {
            callee: Box::new(sp(Expr::Identifier("p".into()))),
            args: vec![sp(Expr::Literal(Literal::Int(41)))],
        });
        let ast = vec![trait_decl, impl_decl, p_fn, main_fn];
        assert_eq!(compile_and_run(&ast), Ok(Value::Int(42)));
    }

    #[test]
    fn feature_22_two_impl_types_dispatch_separately() {
        // trait Doubler { fn double(self) -> Int }
        // impl Doubler for Int  { fn double(self) -> Int { self * 10 } }
        // impl Doubler for Bool { fn double(self) -> Int { if self { 1 } else { 0 } } }
        // fn main() -> Int { (5).double() + (true).double() }   // 50 + 1 = 51
        let trait_decl = doubler_trait(Type::Named("Int".into()));
        let int_body = Block {
            stmts: vec![],
            ret: Some(Box::new(sp(Expr::Binary {
                op: BinaryOp::Mul,
                left: Box::new(sp(Expr::Identifier("self".into()))),
                right: Box::new(sp(Expr::Literal(Literal::Int(10)))),
            }))),
        };
        let bool_body = Block {
            stmts: vec![],
            ret: Some(Box::new(sp(Expr::If {
                condition: Box::new(sp(Expr::Identifier("self".into()))),
                consequence: Box::new(sp(Expr::Literal(Literal::Int(1)))),
                alternative: Some(Box::new(sp(Expr::Literal(Literal::Int(0))))),
            }))),
        };
        let impl_int = impl_doubler_for("Int", Type::Named("Int".into()), int_body);
        let impl_bool = impl_doubler_for("Bool", Type::Named("Int".into()), bool_body);
        let main_fn = fn_main_returns_int(Expr::Binary {
            op: BinaryOp::Add,
            left: Box::new(sp(Expr::Call {
                callee: Box::new(sp(Expr::FieldAccess {
                    base: Box::new(sp(Expr::Literal(Literal::Int(5)))),
                    field: "double".into(),
                })),
                args: vec![],
            })),
            right: Box::new(sp(Expr::Call {
                callee: Box::new(sp(Expr::FieldAccess {
                    base: Box::new(sp(Expr::Literal(Literal::Bool(true)))),
                    field: "double".into(),
                })),
                args: vec![],
            })),
        });
        let ast = vec![trait_decl, impl_int, impl_bool, main_fn];
        assert_eq!(compile_and_run(&ast), Ok(Value::Int(51)));
    }
}
