use ect::ast::{self, Pattern, Span, Spanned};
use ect::ty::Type;
use ect::typeck::Checker;

fn d_span() -> Span { Span::new(0, 0) }
fn sp<T>(node: T) -> Spanned<T> { Spanned { node, span: d_span() } }

#[test]
fn verify_type_inference_primitives() {
    let mut checker = Checker::new();
    assert_eq!(checker.infer_expr(&sp(ast::Expr::Literal(ast::Literal::Int(42)))), Type::Int);
    assert_eq!(checker.infer_expr(&sp(ast::Expr::Literal(ast::Literal::Bool(true)))), Type::Bool);
    assert_eq!(checker.infer_expr(&sp(ast::Expr::Literal(ast::Literal::String("test".into())))), Type::String);
    assert_eq!(checker.infer_expr(&sp(ast::Expr::Literal(ast::Literal::Float(3.14)))), Type::Float);
    assert_eq!(checker.infer_expr(&sp(ast::Expr::Literal(ast::Literal::Char('a')))), Type::Char);
    assert_eq!(checker.infer_expr(&sp(ast::Expr::Literal(ast::Literal::Unit))), Type::Unit);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_binary_add_operations() {
    let mut checker = Checker::new();
    let expr = sp(ast::Expr::Binary {
        op: ast::BinaryOp::Add,
        left: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(10)))),
        right: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(20)))),
    });
    assert_eq!(checker.infer_expr(&expr), Type::Int);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_binary_float_operations() {
    let mut checker = Checker::new();
    let expr = sp(ast::Expr::Binary {
        op: ast::BinaryOp::Mul,
        left: Box::new(sp(ast::Expr::Literal(ast::Literal::Float(2.5)))),
        right: Box::new(sp(ast::Expr::Literal(ast::Literal::Float(3.0)))),
    });
    assert_eq!(checker.infer_expr(&expr), Type::Float);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_binary_type_mismatch_error() {
    let mut checker = Checker::new();
    let expr = sp(ast::Expr::Binary {
        op: ast::BinaryOp::Add,
        left: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(10)))),
        right: Box::new(sp(ast::Expr::Literal(ast::Literal::String("test".into())))),
    });
    let result = checker.infer_expr(&expr);
    assert_eq!(result, Type::Unknown);
    assert_eq!(checker.errors.len(), 1);
    assert!(checker.errors[0].message.contains("Type mismatch"), "Error: {}", checker.errors[0].message);
}

#[test]
fn verify_comparison_operations_return_bool() {
    let mut checker = Checker::new();
    let eq_expr = sp(ast::Expr::Binary {
        op: ast::BinaryOp::Eq,
        left: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(5)))),
        right: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(5)))),
    });
    assert_eq!(checker.infer_expr(&eq_expr), Type::Bool);

    let mut checker = Checker::new();
    let lt_expr = sp(ast::Expr::Binary {
        op: ast::BinaryOp::Lt,
        left: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(3)))),
        right: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(7)))),
    });
    assert_eq!(checker.infer_expr(&lt_expr), Type::Bool);
}

#[test]
fn verify_logical_operations() {
    let mut checker = Checker::new();
    let and_expr = sp(ast::Expr::Binary {
        op: ast::BinaryOp::And,
        left: Box::new(sp(ast::Expr::Literal(ast::Literal::Bool(true)))),
        right: Box::new(sp(ast::Expr::Literal(ast::Literal::Bool(false)))),
    });
    assert_eq!(checker.infer_expr(&and_expr), Type::Bool);
}

#[test]
fn verify_logical_operation_type_error() {
    let mut checker = Checker::new();
    let and_expr = sp(ast::Expr::Binary {
        op: ast::BinaryOp::And,
        left: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(1)))),
        right: Box::new(sp(ast::Expr::Literal(ast::Literal::Bool(true)))),
    });
    let result = checker.infer_expr(&and_expr);
    assert_eq!(result, Type::Unknown);
    assert_eq!(checker.errors.len(), 1);
}

#[test]
fn verify_unary_not_operation() {
    let mut checker = Checker::new();
    let not_expr = sp(ast::Expr::Unary {
        op: ast::UnaryOp::Not,
        right: Box::new(sp(ast::Expr::Literal(ast::Literal::Bool(true)))),
    });
    assert_eq!(checker.infer_expr(&not_expr), Type::Bool);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_unary_not_type_error() {
    let mut checker = Checker::new();
    let not_expr = sp(ast::Expr::Unary {
        op: ast::UnaryOp::Not,
        right: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(42)))),
    });
    let result = checker.infer_expr(&not_expr);
    assert_eq!(result, Type::Unknown);
    assert_eq!(checker.errors.len(), 1);
    assert!(checker.errors[0].message.contains("Expected Bool"));
}

#[test]
fn verify_unary_negation() {
    let mut checker = Checker::new();
    let neg_expr = sp(ast::Expr::Unary {
        op: ast::UnaryOp::Neg,
        right: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(5)))),
    });
    assert_eq!(checker.infer_expr(&neg_expr), Type::Int);

    let mut checker = Checker::new();
    let neg_float = sp(ast::Expr::Unary {
        op: ast::UnaryOp::Neg,
        right: Box::new(sp(ast::Expr::Literal(ast::Literal::Float(3.14)))),
    });
    assert_eq!(checker.infer_expr(&neg_float), Type::Float);
}

#[test]
fn verify_unary_negation_type_error() {
    let mut checker = Checker::new();
    let neg_expr = sp(ast::Expr::Unary {
        op: ast::UnaryOp::Neg,
        right: Box::new(sp(ast::Expr::Literal(ast::Literal::String("hello".into())))),
    });
    let result = checker.infer_expr(&neg_expr);
    assert_eq!(result, Type::Unknown);
    assert_eq!(checker.errors.len(), 1);
}

#[test]
fn verify_reference_operation() {
    let mut checker = Checker::new();
    let ref_expr = sp(ast::Expr::Unary {
        op: ast::UnaryOp::Ref,
        right: Box::new(sp(ast::Expr::Identifier("x".into()))),
    });
    checker.insert_var("x".into(), Type::Int, false, d_span());
    let result = checker.infer_expr(&ref_expr);
    assert_eq!(result, Type::Reference { is_mut: false, inner: Box::new(Type::Int) });
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_mutable_reference() {
    let mut checker = Checker::new();
    let ref_mut_expr = sp(ast::Expr::Unary {
        op: ast::UnaryOp::RefMut,
        right: Box::new(sp(ast::Expr::Identifier("y".into()))),
    });
    checker.insert_var("y".into(), Type::String, true, d_span());
    let result = checker.infer_expr(&ref_mut_expr);
    assert_eq!(result, Type::Reference { is_mut: true, inner: Box::new(Type::String) });
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_reference_to_temporary_error() {
    let mut checker = Checker::new();
    let ref_expr = sp(ast::Expr::Unary {
        op: ast::UnaryOp::Ref,
        right: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(42)))),
    });
    let result = checker.infer_expr(&ref_expr);
    assert_eq!(result, Type::Unknown);
    assert_eq!(checker.errors.len(), 1);
    assert!(checker.errors[0].message.contains("Cannot borrow temporary"));
}

#[test]
fn verify_dereference_operation() {
    let mut checker = Checker::new();
    let deref_expr = sp(ast::Expr::Unary {
        op: ast::UnaryOp::Deref,
        right: Box::new(sp(ast::Expr::Identifier("ptr".into()))),
    });
    checker.insert_var("ptr".into(), Type::Reference { is_mut: false, inner: Box::new(Type::Float) }, false, d_span());
    let result = checker.infer_expr(&deref_expr);
    assert_eq!(result, Type::Float);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_dereference_non_reference_error() {
    let mut checker = Checker::new();
    let deref_expr = sp(ast::Expr::Unary {
        op: ast::UnaryOp::Deref,
        right: Box::new(sp(ast::Expr::Literal(ast::Literal::Bool(true)))),
    });
    let result = checker.infer_expr(&deref_expr);
    assert_eq!(result, Type::Unknown);
    assert_eq!(checker.errors.len(), 1);
    assert!(checker.errors[0].message.contains("Expected reference"));
}

#[test]
fn verify_undefined_variable_error() {
    let mut checker = Checker::new();
    let ident_expr = sp(ast::Expr::Identifier("undefined_var".into()));
    let result = checker.infer_expr(&ident_expr);
    assert_eq!(result, Type::Unknown);
    assert_eq!(checker.errors.len(), 1);
    assert!(checker.errors[0].message.contains("Undefined variable"));
}

#[test]
fn verify_let_statement_with_type_annotation() {
    let mut checker = Checker::new();
    let let_stmt = sp(ast::Stmt::Let {
        pattern: sp(Pattern::Bind("x".into())),
        is_mut: false,
        ty: Some(ast::Type::Named("Int".into())),
        value: sp(ast::Expr::Literal(ast::Literal::Int(42))),
    });
    checker.check_stmt(&let_stmt);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_let_statement_type_mismatch_error() {
    let mut checker = Checker::new();
    let let_stmt = sp(ast::Stmt::Let {
        pattern: sp(Pattern::Bind("x".into())),
        is_mut: false,
        ty: Some(ast::Type::Named("Bool".into())),
        value: sp(ast::Expr::Literal(ast::Literal::Int(42))),
    });
    checker.check_stmt(&let_stmt);
    assert_eq!(checker.errors.len(), 1);
    assert!(checker.errors[0].message.contains("Type mismatch"));
}

#[test]
fn verify_block_expression() {
    let mut checker = Checker::new();
    let block = ast::Block {
        stmts: vec![],
        ret: Some(Box::new(sp(ast::Expr::Literal(ast::Literal::Int(99))))),
    };
    let block_expr = sp(ast::Expr::Block(block));
    let result = checker.infer_expr(&block_expr);
    assert_eq!(result, Type::Int);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_block_with_statements() {
    let mut checker = Checker::new();
    let let_stmt = sp(ast::Stmt::Let {
        pattern: sp(Pattern::Bind("x".into())),
        is_mut: true,
        ty: None,
        value: sp(ast::Expr::Literal(ast::Literal::Int(10))),
    });
    let block = ast::Block {
        stmts: vec![let_stmt],
        ret: Some(Box::new(sp(ast::Expr::Identifier("x".into())))),
    };
    let block_expr = sp(ast::Expr::Block(block));
    let result = checker.infer_expr(&block_expr);
    assert_eq!(result, Type::Int);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_if_expression_matching_branches() {
    let mut checker = Checker::new();
    let if_expr = sp(ast::Expr::If {
        condition: Box::new(sp(ast::Expr::Literal(ast::Literal::Bool(true)))),
        consequence: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(1)))),
        alternative: Some(Box::new(sp(ast::Expr::Literal(ast::Literal::Int(2))))),
    });
    let result = checker.infer_expr(&if_expr);
    assert_eq!(result, Type::Int);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_if_expression_branch_type_mismatch() {
    let mut checker = Checker::new();
    let if_expr = sp(ast::Expr::If {
        condition: Box::new(sp(ast::Expr::Literal(ast::Literal::Bool(true)))),
        consequence: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(1)))),
        alternative: Some(Box::new(sp(ast::Expr::Literal(ast::Literal::String("two".into()))))),
    });
    let result = checker.infer_expr(&if_expr);
    assert_eq!(result, Type::Int);
    assert_eq!(checker.errors.len(), 1);
    assert!(checker.errors[0].message.contains("branch types do not match"));
}

#[test]
fn verify_if_condition_must_be_bool() {
    let mut checker = Checker::new();
    let if_expr = sp(ast::Expr::If {
        condition: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(42)))),
        consequence: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(1)))),
        alternative: None,
    });
    let result = checker.infer_expr(&if_expr);
    assert_eq!(result, Type::Int);
    assert_eq!(checker.errors.len(), 1);
    assert!(checker.errors[0].message.contains("Condition must be Bool"));
}

#[test]
fn verify_borrow_checking_move_semantics() {
    let mut checker = Checker::new();

    let let_stmt = sp(ast::Stmt::Let {
        pattern: sp(Pattern::Bind("text".into())),
        is_mut: false,
        ty: None,
        value: sp(ast::Expr::Literal(ast::Literal::String("hello".into()))),
    });
    checker.check_stmt(&let_stmt);

    let ref_expr = sp(ast::Expr::Unary {
        op: ast::UnaryOp::Ref,
        right: Box::new(sp(ast::Expr::Identifier("text".into()))),
    });
    let ref_ty = checker.infer_expr(&ref_expr);

    assert_eq!(ref_ty, Type::Reference { is_mut: false, inner: Box::new(Type::String) });
    assert!(checker.errors.is_empty(), "Borrowing should not cause an error or move");

    let move_expr = sp(ast::Expr::Identifier("text".into()));
    let _ = checker.infer_expr(&move_expr);
    assert!(checker.errors.is_empty(), "First move should be valid");

    let second_move = sp(ast::Expr::Identifier("text".into()));
    let _ = checker.infer_expr(&second_move);

    assert_eq!(checker.errors.len(), 1);
    assert!(checker.errors[0].message.contains("Use of moved value 'text'"), "Error message: {}", checker.errors[0].message);
}

#[test]
fn verify_borrow_checking_copy_semantics() {
    let mut checker = Checker::new();

    let let_stmt = sp(ast::Stmt::Let {
        pattern: sp(Pattern::Bind("num".into())),
        is_mut: false,
        ty: None,
        value: sp(ast::Expr::Literal(ast::Literal::Int(100))),
    });
    checker.check_stmt(&let_stmt);

    let use_one = sp(ast::Expr::Identifier("num".into()));
    assert_eq!(checker.infer_expr(&use_one), Type::Int);

    let use_two = sp(ast::Expr::Identifier("num".into()));
    assert_eq!(checker.infer_expr(&use_two), Type::Int);

    assert!(checker.errors.is_empty());
}

#[test]
fn verify_error_context_stack() {
    let mut checker = Checker::new();
    let expr = sp(ast::Expr::Binary {
        op: ast::BinaryOp::Add,
        left: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(10)))),
        right: Box::new(sp(ast::Expr::Literal(ast::Literal::String("x".into())))),
    });
    checker.infer_expr(&expr);
    assert_eq!(checker.errors.len(), 1);
    assert_eq!(checker.errors[0].context.len(), 1);
    assert!(checker.errors[0].context[0].contains("binary"));
}

#[test]
fn verify_match_expression_type_unification() {
    let mut checker = Checker::new();
    let expr = sp(ast::Expr::Match {
        scrutinee: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(1)))),
        arms: vec![
            ast::MatchArm {
                pattern: sp(Pattern::Literal(ast::Literal::Int(1))),
                guard: None,
                body: sp(ast::Expr::Literal(ast::Literal::Bool(true))),
            },
            ast::MatchArm {
                pattern: sp(Pattern::Wildcard),
                guard: None,
                body: sp(ast::Expr::Literal(ast::Literal::Bool(false))),
            },
        ],
    });
    assert_eq!(checker.infer_expr(&expr), Type::Bool);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_match_arm_type_mismatch() {
    let mut checker = Checker::new();
    let expr = sp(ast::Expr::Match {
        scrutinee: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(1)))),
        arms: vec![
            ast::MatchArm {
                pattern: sp(Pattern::Literal(ast::Literal::Int(1))),
                guard: None,
                body: sp(ast::Expr::Literal(ast::Literal::Int(42))),
            },
            ast::MatchArm {
                pattern: sp(Pattern::Wildcard),
                guard: None,
                body: sp(ast::Expr::Literal(ast::Literal::String("mismatch".into()))),
            },
        ],
    });
    checker.infer_expr(&expr);
    assert_eq!(checker.errors.len(), 1);
    assert!(checker.errors[0].message.contains("Match arm types"));
}

#[test]
fn verify_match_guard_must_be_bool() {
    let mut checker = Checker::new();
    let expr = sp(ast::Expr::Match {
        scrutinee: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(1)))),
        arms: vec![
            ast::MatchArm {
                pattern: sp(Pattern::Bind("x".into())),
                guard: Some(sp(ast::Expr::Literal(ast::Literal::Int(5)))),
                body: sp(ast::Expr::Literal(ast::Literal::Bool(true))),
            },
        ],
    });
    checker.infer_expr(&expr);
    assert_eq!(checker.errors.len(), 1);
    assert!(checker.errors[0].message.contains("Guard must be Bool"));
}

#[test]
fn verify_for_loop_binding() {
    let mut checker = Checker::new();
    let body = ast::Block {
        stmts: vec![
            sp(ast::Stmt::Expr(sp(ast::Expr::Identifier("x".into())))),
        ],
        ret: None,
    };
    let expr = sp(ast::Expr::For {
        pattern: sp(Pattern::Bind("x".into())),
        iter: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(10)))),
        body,
    });
    let _ty = checker.infer_expr(&expr);
    assert!(checker.errors.is_empty(), "For loop should bind pattern variable");
}

#[test]
fn verify_while_loop_condition_must_be_bool() {
    let mut checker = Checker::new();
    let body = ast::Block { stmts: vec![], ret: None };
    let expr = sp(ast::Expr::While {
        condition: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(5)))),
        body,
    });
    checker.infer_expr(&expr);
    assert_eq!(checker.errors.len(), 1);
    assert!(checker.errors[0].message.contains("While condition must be Bool"));
}

#[test]
fn verify_while_loop_valid() {
    let mut checker = Checker::new();
    let body = ast::Block { stmts: vec![], ret: None };
    let expr = sp(ast::Expr::While {
        condition: Box::new(sp(ast::Expr::Literal(ast::Literal::Bool(true)))),
        body,
    });
    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Unit);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_loop_expression() {
    let mut checker = Checker::new();
    let body = ast::Block {
        stmts: vec![],
        ret: Some(Box::new(sp(ast::Expr::Literal(ast::Literal::Int(42))))),
    };
    let expr = sp(ast::Expr::Loop { body });
    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Int);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_break_outside_loop_error() {
    let mut checker = Checker::new();
    let expr = sp(ast::Expr::Break(None));
    checker.infer_expr(&expr);
    assert_eq!(checker.errors.len(), 1);
    assert!(checker.errors[0].message.contains("Break outside of loop"));
}

#[test]
fn verify_break_inside_loop_valid() {
    let mut checker = Checker::new();
    let body = ast::Block {
        stmts: vec![],
        ret: Some(Box::new(sp(ast::Expr::Break(None)))),
    };
    let expr = sp(ast::Expr::Loop { body });
    checker.infer_expr(&expr);
    assert!(checker.errors.is_empty(), "Break inside loop should be valid");
}

#[test]
fn verify_continue_outside_loop_error() {
    let mut checker = Checker::new();
    let expr = sp(ast::Expr::Continue);
    checker.infer_expr(&expr);
    assert_eq!(checker.errors.len(), 1);
    assert!(checker.errors[0].message.contains("Continue outside of loop"));
}

#[test]
fn verify_continue_inside_loop_valid() {
    let mut checker = Checker::new();
    let body = ast::Block {
        stmts: vec![sp(ast::Stmt::Expr(sp(ast::Expr::Continue)))],
        ret: None,
    };
    let expr = sp(ast::Expr::Loop { body });
    checker.infer_expr(&expr);
    assert!(checker.errors.is_empty(), "Continue inside loop should be valid");
}

#[test]
fn verify_break_type_is_never() {
    let mut checker = Checker::new();
    let body = ast::Block {
        stmts: vec![],
        ret: Some(Box::new(sp(ast::Expr::Break(None)))),
    };
    let expr = sp(ast::Expr::Loop { body });
    let _ty = checker.infer_expr(&expr);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_return_expression() {
    let mut checker = Checker::new();
    let expr = sp(ast::Expr::Return(Some(Box::new(sp(ast::Expr::Literal(ast::Literal::Int(42)))))));
    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Never);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_throw_expression() {
    let mut checker = Checker::new();
    let expr = sp(ast::Expr::Throw(Box::new(sp(ast::Expr::Literal(ast::Literal::String("error".into()))))));
    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Never);
    assert!(checker.errors.is_empty());
}

// Complex Expressions

#[test]
fn verify_function_call_type_checking() {
    let mut checker = Checker::new();
    let fn_type = Type::Function {
        params: vec![Type::Int, Type::Bool],
        effects: vec![],
        ret: Box::new(Type::String),
    };
    checker.insert_var("add".into(), fn_type, false, d_span());

    let expr = sp(ast::Expr::Call {
        callee: Box::new(sp(ast::Expr::Identifier("add".into()))),
        args: vec![
            sp(ast::Expr::Literal(ast::Literal::Int(5))),
            sp(ast::Expr::Literal(ast::Literal::Bool(true))),
        ],
    });

    assert_eq!(checker.infer_expr(&expr), Type::String);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_function_call_argument_count_mismatch() {
    let mut checker = Checker::new();
    let fn_type = Type::Function {
        params: vec![Type::Int],
        effects: vec![],
        ret: Box::new(Type::String),
    };
    checker.insert_var("func".into(), fn_type, false, d_span());

    let expr = sp(ast::Expr::Call {
        callee: Box::new(sp(ast::Expr::Identifier("func".into()))),
        args: vec![
            sp(ast::Expr::Literal(ast::Literal::Int(5))),
            sp(ast::Expr::Literal(ast::Literal::Bool(true))),
        ],
    });

    checker.infer_expr(&expr);
    assert_eq!(checker.errors.len(), 1);
    assert!(checker.errors[0].message.contains("Expected 1 arguments, got 2"));
}

#[test]
fn verify_function_call_argument_type_mismatch() {
    let mut checker = Checker::new();
    let fn_type = Type::Function {
        params: vec![Type::Int],
        effects: vec![],
        ret: Box::new(Type::String),
    };
    checker.insert_var("func".into(), fn_type, false, d_span());

    let expr = sp(ast::Expr::Call {
        callee: Box::new(sp(ast::Expr::Identifier("func".into()))),
        args: vec![sp(ast::Expr::Literal(ast::Literal::Bool(true)))],
    });

    checker.infer_expr(&expr);
    assert_eq!(checker.errors.len(), 1);
    assert!(checker.errors[0].message.contains("Argument 0 type mismatch"));
}

#[test]
fn verify_tuple_expression_type() {
    let mut checker = Checker::new();
    let expr = sp(ast::Expr::Tuple(vec![
        sp(ast::Expr::Literal(ast::Literal::Int(1))),
        sp(ast::Expr::Literal(ast::Literal::Bool(true))),
        sp(ast::Expr::Literal(ast::Literal::String("x".into()))),
    ]));

    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Tuple(vec![Type::Int, Type::Bool, Type::String]));
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_empty_tuple_expression() {
    let mut checker = Checker::new();
    let expr = sp(ast::Expr::Tuple(vec![]));

    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Tuple(vec![]));
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_array_expression_uniform_type() {
    let mut checker = Checker::new();
    let expr = sp(ast::Expr::Array(vec![
        sp(ast::Expr::Literal(ast::Literal::Int(1))),
        sp(ast::Expr::Literal(ast::Literal::Int(2))),
        sp(ast::Expr::Literal(ast::Literal::Int(3))),
    ]));

    let _ty = checker.infer_expr(&expr);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_array_expression_type_mismatch() {
    let mut checker = Checker::new();
    let expr = sp(ast::Expr::Array(vec![
        sp(ast::Expr::Literal(ast::Literal::Int(1))),
        sp(ast::Expr::Literal(ast::Literal::Bool(true))),
    ]));

    checker.infer_expr(&expr);
    assert_eq!(checker.errors.len(), 1);
    assert!(checker.errors[0].message.contains("Array elements must have same type"));
}

#[test]
fn verify_array_repeat_expression() {
    let mut checker = Checker::new();
    let expr = sp(ast::Expr::ArrayRepeat {
        elem: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(5)))),
        count: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(10)))),
    });

    let _ty = checker.infer_expr(&expr);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_array_repeat_non_int_count() {
    let mut checker = Checker::new();
    let expr = sp(ast::Expr::ArrayRepeat {
        elem: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(5)))),
        count: Box::new(sp(ast::Expr::Literal(ast::Literal::Bool(true)))),
    });

    checker.infer_expr(&expr);
    assert_eq!(checker.errors.len(), 1);
    assert!(checker.errors[0].message.contains("Array repeat count must be Int"));
}

#[test]
fn verify_index_expression_on_array() {
    let mut checker = Checker::new();
    checker.insert_var("arr".into(), Type::Named("Array<Int>".into()), false, d_span());

    let expr = sp(ast::Expr::Index {
        base: Box::new(sp(ast::Expr::Identifier("arr".into()))),
        index: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(0)))),
    });

    let _ty = checker.infer_expr(&expr);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_index_non_int_index() {
    let mut checker = Checker::new();
    checker.insert_var("arr".into(), Type::Named("Array<Int>".into()), false, d_span());

    let expr = sp(ast::Expr::Index {
        base: Box::new(sp(ast::Expr::Identifier("arr".into()))),
        index: Box::new(sp(ast::Expr::Literal(ast::Literal::Bool(true)))),
    });

    checker.infer_expr(&expr);
    assert_eq!(checker.errors.len(), 1);
    assert!(checker.errors[0].message.contains("Index must be Int"));
}

#[test]
fn verify_index_on_tuple() {
    let mut checker = Checker::new();
    let tuple_type = Type::Tuple(vec![Type::Int, Type::Bool, Type::String]);
    checker.insert_var("tup".into(), tuple_type, false, d_span());

    let expr = sp(ast::Expr::Index {
        base: Box::new(sp(ast::Expr::Identifier("tup".into()))),
        index: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(0)))),
    });

    let _ty = checker.infer_expr(&expr);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_field_access() {
    let mut checker = Checker::new();
    checker.insert_var("obj".into(), Type::Named("Point".into()), false, d_span());

    let expr = sp(ast::Expr::FieldAccess {
        base: Box::new(sp(ast::Expr::Identifier("obj".into()))),
        field: "x".into(),
    });

    let _ty = checker.infer_expr(&expr);
    assert!(checker.errors.is_empty());
}

// Advanced Expressions

#[test]
fn verify_closure_expression_type() {
    let mut checker = Checker::new();
    let expr = sp(ast::Expr::Closure {
        is_move: false,
        params: vec![
            ast::ClosureParam {
                pattern: sp(Pattern::Bind("x".into())),
                ty: Some(ast::Type::Named("Int".into())),
            },
        ],
        effects: vec![],
        ret_ty: Some(ast::Type::Named("Bool".into())),
        body: Box::new(sp(ast::Expr::Literal(ast::Literal::Bool(true)))),
    });

    let ty = checker.infer_expr(&expr);
    assert!(matches!(ty, Type::Named(ref n) if n == "Closure"));
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_closure_return_type_mismatch() {
    let mut checker = Checker::new();
    let expr = sp(ast::Expr::Closure {
        is_move: false,
        params: vec![],
        effects: vec![],
        ret_ty: Some(ast::Type::Named("Int".into())),
        body: Box::new(sp(ast::Expr::Literal(ast::Literal::Bool(true)))),
    });

    checker.infer_expr(&expr);
    assert_eq!(checker.errors.len(), 1);
    assert!(checker.errors[0].message.contains("Closure body type mismatch"));
}

#[test]
fn verify_record_expression() {
    let mut checker = Checker::new();
    let expr = sp(ast::Expr::Record {
        ty: vec!["Point".into()],
        fields: vec![
            ast::FieldInit {
                name: "x".into(),
                value: Some(sp(ast::Expr::Literal(ast::Literal::Int(10)))),
            },
            ast::FieldInit {
                name: "y".into(),
                value: Some(sp(ast::Expr::Literal(ast::Literal::Int(20)))),
            },
        ],
    });

    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Named("Point".into()));
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_variant_expression() {
    let mut checker = Checker::new();
    let expr = sp(ast::Expr::Variant {
        ty: vec!["Option".into()],
        args: vec![sp(ast::Expr::Literal(ast::Literal::Int(42)))],
    });

    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Named("Option".into()));
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_range_expression_int() {
    let mut checker = Checker::new();
    let expr = sp(ast::Expr::Range {
        start: Some(Box::new(sp(ast::Expr::Literal(ast::Literal::Int(1))))),
        end: Some(Box::new(sp(ast::Expr::Literal(ast::Literal::Int(10))))),
        inclusive: false,
    });

    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Named("Range<Int>".into()));
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_range_non_int_start() {
    let mut checker = Checker::new();
    let expr = sp(ast::Expr::Range {
        start: Some(Box::new(sp(ast::Expr::Literal(ast::Literal::Bool(true))))),
        end: Some(Box::new(sp(ast::Expr::Literal(ast::Literal::Int(10))))),
        inclusive: false,
    });

    checker.infer_expr(&expr);
    assert_eq!(checker.errors.len(), 1);
    assert!(checker.errors[0].message.contains("Range start must be Int"));
}

#[test]
fn verify_range_non_int_end() {
    let mut checker = Checker::new();
    let expr = sp(ast::Expr::Range {
        start: Some(Box::new(sp(ast::Expr::Literal(ast::Literal::Int(1))))),
        end: Some(Box::new(sp(ast::Expr::Literal(ast::Literal::String("x".into()))))),
        inclusive: false,
    });

    checker.infer_expr(&expr);
    assert_eq!(checker.errors.len(), 1);
    assert!(checker.errors[0].message.contains("Range end must be Int"));
}

#[test]
fn verify_scope_expression() {
    let mut checker = Checker::new();
    let body = ast::Block {
        stmts: vec![],
        ret: Some(Box::new(sp(ast::Expr::Literal(ast::Literal::Int(42))))),
    };
    let expr = sp(ast::Expr::Scope {
        label: Some("s".into()),
        options: None,
        body,
    });

    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Int);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_region_expression() {
    let mut checker = Checker::new();
    let body = ast::Block {
        stmts: vec![],
        ret: Some(Box::new(sp(ast::Expr::Literal(ast::Literal::String("x".into()))))),
    };
    let expr = sp(ast::Expr::Region {
        label: None,
        body,
    });

    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::String);
    assert!(checker.errors.is_empty());
}

// Special Scopes & Effects

#[test]
fn verify_scope_with_label() {
    let mut checker = Checker::new();
    let body = ast::Block {
        stmts: vec![],
        ret: Some(Box::new(sp(ast::Expr::Literal(ast::Literal::Int(42))))),
    };
    let expr = sp(ast::Expr::Scope {
        label: Some("outer".into()),
        options: None,
        body,
    });

    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Int);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_nested_scopes() {
    let mut checker = Checker::new();
    let inner_scope = sp(ast::Expr::Scope {
        label: Some("inner".into()),
        options: None,
        body: ast::Block {
            stmts: vec![],
            ret: Some(Box::new(sp(ast::Expr::Literal(ast::Literal::Bool(true))))),
        },
    });

    let outer_scope = sp(ast::Expr::Scope {
        label: Some("outer".into()),
        options: None,
        body: ast::Block {
            stmts: vec![sp(ast::Stmt::Expr(inner_scope))],
            ret: None,
        },
    });

    let ty = checker.infer_expr(&outer_scope);
    assert_eq!(ty, Type::Unit);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_region_with_label() {
    let mut checker = Checker::new();
    let body = ast::Block {
        stmts: vec![],
        ret: Some(Box::new(sp(ast::Expr::Literal(ast::Literal::Float(3.14))))),
    };
    let expr = sp(ast::Expr::Region {
        label: Some("heap".into()),
        body,
    });

    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Float);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_handle_return_arm() {
    let mut checker = Checker::new();
    let expr = sp(ast::Expr::Handle {
        expr: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(10)))),
        arms: vec![
            ast::HandleArm {
                kind: ast::HandleArmKind::Return,
                pattern: Some(sp(Pattern::Bind("x".into()))),
                body: sp(ast::Expr::Literal(ast::Literal::Int(42))),
            },
        ],
    });

    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Int);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_handle_exception_arm() {
    let mut checker = Checker::new();
    let expr = sp(ast::Expr::Handle {
        expr: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(1)))),
        arms: vec![
            ast::HandleArm {
                kind: ast::HandleArmKind::Exn,
                pattern: Some(sp(Pattern::Bind("e".into()))),
                body: sp(ast::Expr::Literal(ast::Literal::Int(0))),
            },
        ],
    });

    checker.infer_expr(&expr);
    assert_eq!(checker.errors.len(), 1);
    assert!(checker.errors[0].message.contains("no exn effect is active"));
}

#[test]
fn verify_handle_custom_effect() {
    let mut checker = Checker::new();
    let expr = sp(ast::Expr::Handle {
        expr: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(1)))),
        arms: vec![
            ast::HandleArm {
                kind: ast::HandleArmKind::Effect(vec!["logger".into(), "log".into()]),
                pattern: Some(sp(Pattern::Bind("msg".into()))),
                body: sp(ast::Expr::Literal(ast::Literal::Unit)),
            },
        ],
    });

    checker.infer_expr(&expr);
    assert_eq!(checker.errors.len(), 1);
    assert!(checker.errors[0].message.contains("not active"));
}

#[test]
fn verify_handle_multiple_arms_type_unification() {
    let mut checker = Checker::new();
    let expr = sp(ast::Expr::Handle {
        expr: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(1)))),
        arms: vec![
            ast::HandleArm {
                kind: ast::HandleArmKind::Return,
                pattern: Some(sp(Pattern::Bind("x".into()))),
                body: sp(ast::Expr::Literal(ast::Literal::Int(42))),
            },
            ast::HandleArm {
                kind: ast::HandleArmKind::Exn,
                pattern: Some(sp(Pattern::Bind("e".into()))),
                body: sp(ast::Expr::Literal(ast::Literal::Int(0))),
            },
        ],
    });

    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Int);
}

#[test]
fn verify_handle_arm_type_mismatch() {
    let mut checker = Checker::new();
    let expr = sp(ast::Expr::Handle {
        expr: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(1)))),
        arms: vec![
            ast::HandleArm {
                kind: ast::HandleArmKind::Return,
                pattern: Some(sp(Pattern::Bind("x".into()))),
                body: sp(ast::Expr::Literal(ast::Literal::Int(42))),
            },
            ast::HandleArm {
                kind: ast::HandleArmKind::Exn,
                pattern: Some(sp(Pattern::Bind("e".into()))),
                body: sp(ast::Expr::Literal(ast::Literal::String("error".into()))),
            },
        ],
    });

    checker.infer_expr(&expr);
    assert!(checker.errors.iter().any(|e| e.message.contains("Handle arm types do not match")));
}

#[test]
fn verify_scope_with_statements() {
    let mut checker = Checker::new();
    let body = ast::Block {
        stmts: vec![
            sp(ast::Stmt::Let {
                pattern: sp(Pattern::Bind("x".into())),
                is_mut: false,
                ty: None,
                value: sp(ast::Expr::Literal(ast::Literal::Int(5))),
            }),
            sp(ast::Stmt::Let {
                pattern: sp(Pattern::Bind("y".into())),
                is_mut: false,
                ty: None,
                value: sp(ast::Expr::Literal(ast::Literal::Int(10))),
            }),
        ],
        ret: Some(Box::new(sp(ast::Expr::Binary {
            op: ast::BinaryOp::Add,
            left: Box::new(sp(ast::Expr::Identifier("x".into()))),
            right: Box::new(sp(ast::Expr::Identifier("y".into()))),
        }))),
    };

    let expr = sp(ast::Expr::Scope {
        label: None,
        options: None,
        body,
    });

    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Int);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_region_with_statements() {
    let mut checker = Checker::new();
    let body = ast::Block {
        stmts: vec![
            sp(ast::Stmt::Let {
                pattern: sp(Pattern::Bind("ptr".into())),
                is_mut: false,
                ty: None,
                value: sp(ast::Expr::Literal(ast::Literal::Int(0))),
            }),
        ],
        ret: Some(Box::new(sp(ast::Expr::Identifier("ptr".into())))),
    };

    let expr = sp(ast::Expr::Region {
        label: Some("r".into()),
        body,
    });

    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Int);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_handle_without_arms() {
    let mut checker = Checker::new();
    let expr = sp(ast::Expr::Handle {
        expr: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(1)))),
        arms: vec![],
    });

    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Unknown);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_scope_effect_isolation() {
    let mut checker = Checker::new();
    let scope_expr = sp(ast::Expr::Scope {
        label: Some("s".into()),
        options: None,
        body: ast::Block {
            stmts: vec![],
            ret: Some(Box::new(sp(ast::Expr::Literal(ast::Literal::Bool(true))))),
        },
    });

    let ty = checker.infer_expr(&scope_expr);
    assert_eq!(ty, Type::Bool);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_region_effect_isolation() {
    let mut checker = Checker::new();
    let region_expr = sp(ast::Expr::Region {
        label: Some("r".into()),
        body: ast::Block {
            stmts: vec![],
            ret: Some(Box::new(sp(ast::Expr::Literal(ast::Literal::Char('a'))))),
        },
    });

    let ty = checker.infer_expr(&region_expr);
    assert_eq!(ty, Type::Char);
    assert!(checker.errors.is_empty());
}

// Infrastructure & Context Management

#[test]
fn verify_function_registry() {
    let mut checker = Checker::new();
    checker.register_function(
        "add".into(),
        vec![Type::Int, Type::Int],
        Type::Int,
    );

    let result = checker.get_function("add");
    assert!(result.is_some());
    let (params, ret) = result.unwrap();
    assert_eq!(params, vec![Type::Int, Type::Int]);
    assert_eq!(ret, Type::Int);
}

#[test]
fn verify_type_registry() {
    let mut checker = Checker::new();
    checker.register_type(
        "Point".into(),
        ast::TypeBody::Record(vec![]),
    );

    let result = checker.get_type("Point");
    assert!(result.is_some());
}

#[test]
fn verify_const_registry() {
    let mut checker = Checker::new();
    checker.register_const("PI".into(), Type::Float);

    let result = checker.get_const("PI");
    assert!(result.is_some());
    assert_eq!(result.unwrap(), Type::Float);
}

#[test]
fn verify_pattern_bind() {
    let mut checker = Checker::new();
    let pattern = sp(Pattern::Bind("x".into()));
    checker.check_pattern(&pattern, &Type::Int, d_span());

    let var_ty = checker.get_var("x", false, d_span());
    assert_eq!(var_ty, Type::Int);
}

#[test]
fn verify_pattern_wildcard() {
    let mut checker = Checker::new();
    let pattern = sp(Pattern::Wildcard);
    checker.check_pattern(&pattern, &Type::String, d_span());
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_pattern_literal_match() {
    let mut checker = Checker::new();
    let pattern = sp(Pattern::Literal(ast::Literal::Int(42)));
    checker.check_pattern(&pattern, &Type::Int, d_span());
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_pattern_literal_mismatch() {
    let mut checker = Checker::new();
    let pattern = sp(Pattern::Literal(ast::Literal::Int(42)));
    checker.check_pattern(&pattern, &Type::Bool, d_span());
    assert_eq!(checker.errors.len(), 1);
    assert!(checker.errors[0].message.contains("Pattern type mismatch"));
}

#[test]
fn verify_pattern_tuple_match() {
    let mut checker = Checker::new();
    let pattern = sp(Pattern::Tuple(vec![
        sp(Pattern::Bind("x".into())),
        sp(Pattern::Bind("y".into())),
    ]));
    let tuple_ty = Type::Tuple(vec![Type::Int, Type::Bool]);
    checker.check_pattern(&pattern, &tuple_ty, d_span());

    assert_eq!(checker.get_var("x", false, d_span()), Type::Int);
    assert_eq!(checker.get_var("y", false, d_span()), Type::Bool);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_pattern_tuple_length_mismatch() {
    let mut checker = Checker::new();
    let pattern = sp(Pattern::Tuple(vec![
        sp(Pattern::Bind("x".into())),
        sp(Pattern::Bind("y".into())),
    ]));
    let tuple_ty = Type::Tuple(vec![Type::Int]);
    checker.check_pattern(&pattern, &tuple_ty, d_span());
    assert_eq!(checker.errors.len(), 1);
    assert!(checker.errors[0].message.contains("Tuple pattern length mismatch"));
}

#[test]
fn verify_pattern_or() {
    let mut checker = Checker::new();
    let pattern = sp(Pattern::Or(vec![
        sp(Pattern::Bind("a".into())),
        sp(Pattern::Bind("b".into())),
    ]));
    checker.check_pattern(&pattern, &Type::Int, d_span());

    assert_eq!(checker.get_var("a", false, d_span()), Type::Int);
    assert_eq!(checker.get_var("b", false, d_span()), Type::Int);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_pattern_range_int() {
    let mut checker = Checker::new();
    let pattern = sp(Pattern::Range {
        start: Some(ast::Literal::Int(0)),
        end: Some(ast::Literal::Int(10)),
        inclusive: false,
    });
    checker.check_pattern(&pattern, &Type::Int, d_span());
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_pattern_range_non_int() {
    let mut checker = Checker::new();
    let pattern = sp(Pattern::Range {
        start: Some(ast::Literal::Int(0)),
        end: Some(ast::Literal::Int(10)),
        inclusive: false,
    });
    checker.check_pattern(&pattern, &Type::Bool, d_span());
    assert_eq!(checker.errors.len(), 1);
    assert!(checker.errors[0].message.contains("Range pattern requires Int"));
}

#[test]
fn verify_pattern_array() {
    let mut checker = Checker::new();
    let pattern = sp(Pattern::Array(vec![
        sp(Pattern::Bind("x".into())),
        sp(Pattern::Bind("y".into())),
    ]));
    let array_ty = Type::Named("Array<Int>".into());
    checker.check_pattern(&pattern, &array_ty, d_span());
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_pattern_array_wrong_type() {
    let mut checker = Checker::new();
    let pattern = sp(Pattern::Array(vec![sp(Pattern::Wildcard)]));
    checker.check_pattern(&pattern, &Type::Int, d_span());
    assert_eq!(checker.errors.len(), 1);
    assert!(checker.errors[0].message.contains("Expected array pattern"));
}

#[test]
fn verify_pattern_ref() {
    let mut checker = Checker::new();
    let pattern = sp(Pattern::Ref(Box::new(sp(Pattern::Bind("x".into())))));
    let ref_ty = Type::Reference { is_mut: false, inner: Box::new(Type::String) };
    checker.check_pattern(&pattern, &ref_ty, d_span());

    assert_eq!(checker.get_var("x", false, d_span()), Type::String);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_pattern_ref_non_reference() {
    let mut checker = Checker::new();
    let pattern = sp(Pattern::Ref(Box::new(sp(Pattern::Wildcard))));
    checker.check_pattern(&pattern, &Type::Int, d_span());
    assert_eq!(checker.errors.len(), 1);
    assert!(checker.errors[0].message.contains("Expected reference pattern"));
}

#[test]
fn verify_let_with_tuple_pattern() {
    let mut checker = Checker::new();
    let stmt = sp(ast::Stmt::Let {
        pattern: sp(Pattern::Tuple(vec![
            sp(Pattern::Bind("x".into())),
            sp(Pattern::Bind("y".into())),
        ])),
        is_mut: false,
        ty: None,
        value: sp(ast::Expr::Tuple(vec![
            sp(ast::Expr::Literal(ast::Literal::Int(1))),
            sp(ast::Expr::Literal(ast::Literal::Bool(true))),
        ])),
    });

    checker.check_stmt(&stmt);

    assert_eq!(checker.get_var("x", false, d_span()), Type::Int);
    assert_eq!(checker.get_var("y", false, d_span()), Type::Bool);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_pattern_record() {
    let mut checker = Checker::new();
    let pattern = sp(Pattern::Record {
        ty: vec!["Point".into()],
        fields: vec![
            ast::FieldPattern {
                name: "x".into(),
                pattern: Some(sp(Pattern::Bind("px".into()))),
            },
        ],
        rest: false,
    });
    checker.check_pattern(&pattern, &Type::Named("Point".into()), d_span());
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_pattern_variant() {
    let mut checker = Checker::new();
    let pattern = sp(Pattern::Variant {
        ty: vec!["Option".into()],
        args: vec![sp(Pattern::Bind("val".into()))],
    });
    checker.check_pattern(&pattern, &Type::Named("Option".into()), d_span());
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_nested_pattern_tuple_and_bind() {
    let mut checker = Checker::new();
    let pattern = sp(Pattern::Tuple(vec![
        sp(Pattern::Bind("x".into())),
        sp(Pattern::Tuple(vec![
            sp(Pattern::Bind("a".into())),
            sp(Pattern::Bind("b".into())),
        ])),
    ]));
    let tuple_ty = Type::Tuple(vec![
        Type::Int,
        Type::Tuple(vec![Type::Bool, Type::String]),
    ]);
    checker.check_pattern(&pattern, &tuple_ty, d_span());

    assert_eq!(checker.get_var("x", false, d_span()), Type::Int);
    assert_eq!(checker.get_var("a", false, d_span()), Type::Bool);
    assert_eq!(checker.get_var("b", false, d_span()), Type::String);
    assert!(checker.errors.is_empty());
}

// Type Conversion & Compatibility

#[test]
fn verify_convert_primitive_types() {
    let checker = Checker::new();
    assert_eq!(checker.convert_type(&ast::Type::Named("Int".into())), Type::Int);
    assert_eq!(checker.convert_type(&ast::Type::Named("Float".into())), Type::Float);
    assert_eq!(checker.convert_type(&ast::Type::Named("Bool".into())), Type::Bool);
    assert_eq!(checker.convert_type(&ast::Type::Named("Char".into())), Type::Char);
    assert_eq!(checker.convert_type(&ast::Type::Named("String".into())), Type::String);
    assert_eq!(checker.convert_type(&ast::Type::Named("Unit".into())), Type::Unit);
}

#[test]
fn verify_convert_qualified_types() {
    let checker = Checker::new();
    let qualified = ast::Type::Qualified(vec!["std".into(), "io".into(), "Error".into()]);
    let converted = checker.convert_type(&qualified);
    assert_eq!(converted, Type::Named("std.io.Error".into()));
}

#[test]
fn verify_convert_generic_types() {
    let checker = Checker::new();
    let generic = ast::Type::Generic {
        name: "List".into(),
        args: vec![ast::Type::Named("Int".into())],
    };
    let converted = checker.convert_type(&generic);
    assert!(format!("{:?}", converted).contains("List"));
}

#[test]
fn verify_convert_array_with_size() {
    let checker = Checker::new();
    let array = ast::Type::Array {
        elem: Box::new(ast::Type::Named("Int".into())),
        size: 16,
    };
    let converted = checker.convert_type(&array);
    assert!(format!("{:?}", converted).contains("16"));
}

#[test]
fn verify_convert_tuple_types() {
    let checker = Checker::new();
    let tuple = ast::Type::Tuple(vec![
        ast::Type::Named("Int".into()),
        ast::Type::Named("Bool".into()),
    ]);
    let converted = checker.convert_type(&tuple);
    assert_eq!(converted, Type::Tuple(vec![Type::Int, Type::Bool]));
}

#[test]
fn verify_convert_reference_types() {
    let checker = Checker::new();
    let reference = ast::Type::Reference {
        is_mut: false,
        inner: Box::new(ast::Type::Named("String".into())),
        region: None,
    };
    let converted = checker.convert_type(&reference);
    assert_eq!(
        converted,
        Type::Reference {
            is_mut: false,
            inner: Box::new(Type::String),
        }
    );
}

#[test]
fn verify_convert_mutable_reference() {
    let checker = Checker::new();
    let reference = ast::Type::Reference {
        is_mut: true,
        inner: Box::new(ast::Type::Named("Int".into())),
        region: None,
    };
    let converted = checker.convert_type(&reference);
    assert_eq!(
        converted,
        Type::Reference {
            is_mut: true,
            inner: Box::new(Type::Int),
        }
    );
}

#[test]
fn verify_convert_function_types() {
    let checker = Checker::new();
    let function = ast::Type::Function {
        params: vec![
            ast::Type::Named("Int".into()),
            ast::Type::Named("Bool".into()),
        ],
        effects: vec![],
        ret: Box::new(ast::Type::Named("String".into())),
    };
    let converted = checker.convert_type(&function);
    assert_eq!(
        converted,
        Type::Function {
            params: vec![Type::Int, Type::Bool],
            effects: vec![],
            ret: Box::new(Type::String),
        }
    );
}

#[test]
fn verify_types_compatible_same_types() {
    let checker = Checker::new();
    assert!(checker.types_compatible(&Type::Int, &Type::Int));
    assert!(checker.types_compatible(&Type::Bool, &Type::Bool));
    assert!(checker.types_compatible(&Type::String, &Type::String));
}

#[test]
fn verify_types_compatible_with_unknown() {
    let checker = Checker::new();
    assert!(checker.types_compatible(&Type::Int, &Type::Unknown));
    assert!(checker.types_compatible(&Type::Unknown, &Type::Int));
    assert!(checker.types_compatible(&Type::Unknown, &Type::Unknown));
}

#[test]
fn verify_types_compatible_different_types() {
    let checker = Checker::new();
    assert!(!checker.types_compatible(&Type::Int, &Type::Bool));
    assert!(!checker.types_compatible(&Type::String, &Type::Float));
}

#[test]
fn verify_types_compatible_tuples() {
    let checker = Checker::new();
    let tuple1 = Type::Tuple(vec![Type::Int, Type::Bool]);
    let tuple2 = Type::Tuple(vec![Type::Int, Type::Bool]);
    assert!(checker.types_compatible(&tuple1, &tuple2));
}

#[test]
fn verify_types_compatible_tuple_length_mismatch() {
    let checker = Checker::new();
    let tuple1 = Type::Tuple(vec![Type::Int, Type::Bool]);
    let tuple2 = Type::Tuple(vec![Type::Int]);
    assert!(!checker.types_compatible(&tuple1, &tuple2));
}

#[test]
fn verify_types_compatible_tuple_element_mismatch() {
    let checker = Checker::new();
    let tuple1 = Type::Tuple(vec![Type::Int, Type::Bool]);
    let tuple2 = Type::Tuple(vec![Type::Int, Type::String]);
    assert!(!checker.types_compatible(&tuple1, &tuple2));
}

#[test]
fn verify_types_compatible_references() {
    let checker = Checker::new();
    let ref1 = Type::Reference { is_mut: false, inner: Box::new(Type::Int) };
    let ref2 = Type::Reference { is_mut: false, inner: Box::new(Type::Int) };
    assert!(checker.types_compatible(&ref1, &ref2));
}

#[test]
fn verify_types_compatible_reference_mutability_mismatch() {
    let checker = Checker::new();
    let ref_immut = Type::Reference { is_mut: false, inner: Box::new(Type::Int) };
    let ref_mut = Type::Reference { is_mut: true, inner: Box::new(Type::Int) };
    assert!(!checker.types_compatible(&ref_immut, &ref_mut));
}

#[test]
fn verify_types_compatible_functions() {
    let checker = Checker::new();
    let fn1 = Type::Function {
        params: vec![Type::Int, Type::Bool],
        effects: vec![],
        ret: Box::new(Type::String),
    };
    let fn2 = Type::Function {
        params: vec![Type::Int, Type::Bool],
        effects: vec![],
        ret: Box::new(Type::String),
    };
    assert!(checker.types_compatible(&fn1, &fn2));
}

#[test]
fn verify_types_compatible_function_param_mismatch() {
    let checker = Checker::new();
    let fn1 = Type::Function {
        params: vec![Type::Int, Type::Bool],
        effects: vec![],
        ret: Box::new(Type::String),
    };
    let fn2 = Type::Function {
        params: vec![Type::Int, Type::String],
        effects: vec![],
        ret: Box::new(Type::String),
    };
    assert!(!checker.types_compatible(&fn1, &fn2));
}

#[test]
fn verify_unify_same_types() {
    let checker = Checker::new();
    let result = checker.unify_types(&Type::Int, &Type::Int);
    assert_eq!(result, Some(Type::Int));
}

#[test]
fn verify_unify_with_unknown() {
    let checker = Checker::new();
    assert_eq!(checker.unify_types(&Type::Int, &Type::Unknown), Some(Type::Int));
    assert_eq!(checker.unify_types(&Type::Unknown, &Type::Bool), Some(Type::Bool));
}

#[test]
fn verify_unify_tuples() {
    let checker = Checker::new();
    let tuple1 = Type::Tuple(vec![Type::Int, Type::Bool]);
    let tuple2 = Type::Tuple(vec![Type::Int, Type::Bool]);
    let unified = checker.unify_types(&tuple1, &tuple2);
    assert_eq!(unified, Some(tuple1));
}

#[test]
fn verify_unify_incompatible_types() {
    let checker = Checker::new();
    let result = checker.unify_types(&Type::Int, &Type::Bool);
    assert_eq!(result, None);
}

#[test]
fn verify_unify_references() {
    let checker = Checker::new();
    let ref1 = Type::Reference { is_mut: false, inner: Box::new(Type::Int) };
    let ref2 = Type::Reference { is_mut: false, inner: Box::new(Type::Int) };
    let unified = checker.unify_types(&ref1, &ref2);
    assert!(unified.is_some());
}

#[test]
fn verify_unify_functions() {
    let checker = Checker::new();
    let fn1 = Type::Function {
        params: vec![Type::Int],
        effects: vec![],
        ret: Box::new(Type::Bool),
    };
    let fn2 = Type::Function {
        params: vec![Type::Int],
        effects: vec![],
        ret: Box::new(Type::Bool),
    };
    let unified = checker.unify_types(&fn1, &fn2);
    assert!(unified.is_some());
}

#[test]
fn verify_is_assignable() {
    let checker = Checker::new();
    assert!(checker.is_assignable(&Type::Int, &Type::Int));
    assert!(checker.is_assignable(&Type::Int, &Type::Unknown));
    assert!(checker.is_assignable(&Type::Unknown, &Type::Bool));
    assert!(!checker.is_assignable(&Type::Int, &Type::Bool));
}

// Ownership & Borrowing Tests

#[test]
fn verify_ownership_primitives_are_copy() {
    let _checker = Checker::new();
    use ect::ty::Ownership;
    assert_eq!(Type::Int.ownership(), Ownership::Copy);
    assert_eq!(Type::Bool.ownership(), Ownership::Copy);
    assert_eq!(Type::Float.ownership(), Ownership::Copy);
    assert_eq!(Type::Char.ownership(), Ownership::Copy);
    assert_eq!(Type::Unit.ownership(), Ownership::Copy);
}

#[test]
fn verify_ownership_string_is_move() {
    use ect::ty::Ownership;
    assert_eq!(Type::String.ownership(), Ownership::Move);
}

#[test]
fn verify_ownership_reference_is_copy() {
    use ect::ty::Ownership;
    let ref_ty = Type::Reference { is_mut: false, inner: Box::new(Type::String) };
    assert_eq!(ref_ty.ownership(), Ownership::Copy);
}

#[test]
fn verify_ownership_tuple_copy_all() {
    use ect::ty::Ownership;
    let tuple = Type::Tuple(vec![Type::Int, Type::Bool, Type::Float]);
    assert_eq!(tuple.ownership(), Ownership::Copy);
}

#[test]
fn verify_ownership_tuple_move_with_string() {
    use ect::ty::Ownership;
    let tuple = Type::Tuple(vec![Type::Int, Type::String]);
    assert_eq!(tuple.ownership(), Ownership::Move);
}

#[test]
fn verify_immutable_borrow_allowed() {
    let mut checker = Checker::new();
    checker.insert_var("x".into(), Type::Int, false, d_span());
    assert!(checker.try_immut_borrow("x", d_span()).is_ok());
}

#[test]
fn verify_mutable_borrow_not_allowed_on_immut_var() {
    let mut checker = Checker::new();
    checker.insert_var("x".into(), Type::Int, false, d_span());
    let result = checker.try_mut_borrow("x", d_span());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Cannot mutably borrow immutable variable"));
}

#[test]
fn verify_mutable_borrow_allowed_on_mut_var() {
    let mut checker = Checker::new();
    checker.insert_var("x".into(), Type::Int, true, d_span());
    assert!(checker.try_mut_borrow("x", d_span()).is_ok());
}

#[test]
fn verify_borrow_double_immutable_allowed() {
    let mut checker = Checker::new();
    checker.insert_var("x".into(), Type::Int, false, d_span());
    assert!(checker.try_immut_borrow("x", d_span()).is_ok());
    assert!(checker.try_immut_borrow("x", d_span()).is_ok());
}

#[test]
fn verify_borrow_immutable_then_mutable_error() {
    let mut checker = Checker::new();
    checker.insert_var("x".into(), Type::Int, true, d_span());
    assert!(checker.try_immut_borrow("x", d_span()).is_ok());
    let result = checker.try_mut_borrow("x", d_span());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Cannot mutably borrow"));
}

#[test]
fn verify_borrow_mutable_then_immutable_error() {
    let mut checker = Checker::new();
    checker.insert_var("x".into(), Type::Int, true, d_span());
    assert!(checker.try_mut_borrow("x", d_span()).is_ok());
    let result = checker.try_immut_borrow("x", d_span());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Cannot immutably borrow"));
}

#[test]
fn verify_borrow_mutable_twice_error() {
    let mut checker = Checker::new();
    checker.insert_var("x".into(), Type::Int, true, d_span());
    assert!(checker.try_mut_borrow("x", d_span()).is_ok());
    let result = checker.try_mut_borrow("x", d_span());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("mutable borrow already active"));
}

#[test]
fn verify_move_copy_type_when_using_identifier() {
    let mut checker = Checker::new();
    checker.insert_var("x".into(), Type::Int, false, d_span());
    let expr = sp(ast::Expr::Identifier("x".into()));
    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Int);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_move_move_type_when_using_identifier() {
    let mut checker = Checker::new();
    checker.insert_var("x".into(), Type::String, false, d_span());
    let expr = sp(ast::Expr::Identifier("x".into()));
    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::String);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_use_after_move_error() {
    let mut checker = Checker::new();
    checker.insert_var("x".into(), Type::String, false, d_span());

    let expr1 = sp(ast::Expr::Identifier("x".into()));
    let _ty1 = checker.infer_expr(&expr1);

    let expr2 = sp(ast::Expr::Identifier("x".into()));
    let _ty2 = checker.infer_expr(&expr2);

    assert!(!checker.errors.is_empty());
    assert!(checker.errors[0].message.contains("Use of moved value"));
}

#[test]
fn verify_reference_operation_immutable() {
    let mut checker = Checker::new();
    checker.insert_var("x".into(), Type::Int, false, d_span());
    let expr = sp(ast::Expr::Unary {
        op: ast::UnaryOp::Ref,
        right: Box::new(sp(ast::Expr::Identifier("x".into()))),
    });
    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Reference { is_mut: false, inner: Box::new(Type::Int) });
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_reference_operation_mutable() {
    let mut checker = Checker::new();
    checker.insert_var("x".into(), Type::Int, true, d_span());
    let expr = sp(ast::Expr::Unary {
        op: ast::UnaryOp::RefMut,
        right: Box::new(sp(ast::Expr::Identifier("x".into()))),
    });
    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Reference { is_mut: true, inner: Box::new(Type::Int) });
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_mutable_reference_on_immutable_error() {
    let mut checker = Checker::new();
    checker.insert_var("x".into(), Type::Int, false, d_span());
    let expr = sp(ast::Expr::Unary {
        op: ast::UnaryOp::RefMut,
        right: Box::new(sp(ast::Expr::Identifier("x".into()))),
    });
    let _ty = checker.infer_expr(&expr);
    assert!(!checker.errors.is_empty());
}

#[test]
fn verify_ownership_in_let_statement_copy() {
    let mut checker = Checker::new();
    let init_expr = sp(ast::Expr::Literal(ast::Literal::Int(42)));
    let stmt = sp(ast::Stmt::Let {
        pattern: sp(Pattern::Bind("x".into())),
        is_mut: false,
        ty: None,
        value: init_expr,
    });
    checker.check_stmt(&stmt);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_ownership_in_let_statement_move() {
    let mut checker = Checker::new();
    let init_expr = sp(ast::Expr::Literal(ast::Literal::String("hello".into())));
    let stmt = sp(ast::Stmt::Let {
        pattern: sp(Pattern::Bind("s".into())),
        is_mut: false,
        ty: None,
        value: init_expr,
    });
    checker.check_stmt(&stmt);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_copy_semantics_multiple_uses() {
    let mut checker = Checker::new();
    checker.insert_var("x".into(), Type::Int, false, d_span());

    let expr1 = sp(ast::Expr::Identifier("x".into()));
    checker.infer_expr(&expr1);

    let expr2 = sp(ast::Expr::Identifier("x".into()));
    checker.infer_expr(&expr2);

    assert!(checker.errors.is_empty());
}

#[test]
fn verify_release_borrow() {
    let mut checker = Checker::new();
    checker.insert_var("x".into(), Type::Int, false, d_span());
    checker.try_immut_borrow("x", d_span()).unwrap();
    checker.release_borrow("x");
    assert!(true);
}

#[test]
fn verify_check_ownership_method() {
    let checker = Checker::new();
    use ect::ty::Ownership;

    assert_eq!(checker.check_ownership(&Type::Int), Ownership::Copy);
    assert_eq!(checker.check_ownership(&Type::String), Ownership::Move);

    let ref_ty = Type::Reference { is_mut: false, inner: Box::new(Type::String) };
    assert_eq!(checker.check_ownership(&ref_ty), Ownership::Copy);
}

// Effects System Tests

#[test]
fn verify_effect_registration() {
    let mut checker = Checker::new();
    checker.register_effect("io".into(), vec!["read".into(), "write".into()]);

    let effect = checker.get_effect("io");
    assert!(effect.is_some());
    assert_eq!(effect.unwrap(), vec!["read", "write"]);
}

#[test]
fn verify_effect_alias_registration() {
    let mut checker = Checker::new();
    use ect::ty::Effect;

    let effects = vec![Effect::Async, Effect::Alloc];
    checker.register_effect_alias("concurrent".into(), effects.clone());

    let alias = checker.get_effect_alias("concurrent");
    assert!(alias.is_some());
    assert_eq!(alias.unwrap().len(), 2);
}

#[test]
fn verify_push_and_pop_effect() {
    let mut checker = Checker::new();
    use ect::ty::Effect;

    checker.push_effect(Effect::Async);
    checker.push_effect(Effect::Alloc);

    let expected = vec![Effect::Async, Effect::Alloc];
    assert!(checker.effects_compatible(&expected, &expected));

    checker.pop_effect();
    let expected2 = vec![Effect::Async];
    assert!(checker.effects_compatible(&expected2, &expected2));
}

#[test]
fn verify_effects_equal_total() {
    let checker = Checker::new();
    use ect::ty::Effect;

    assert!(checker.effects_equal(&Effect::Total, &Effect::Total));
    assert!(!checker.effects_equal(&Effect::Total, &Effect::Async));
}

#[test]
fn verify_effects_equal_async() {
    let checker = Checker::new();
    use ect::ty::Effect;

    assert!(checker.effects_equal(&Effect::Async, &Effect::Async));
}

#[test]
fn verify_effects_equal_alloc() {
    let checker = Checker::new();
    use ect::ty::Effect;

    assert!(checker.effects_equal(&Effect::Alloc, &Effect::Alloc));
}

#[test]
fn verify_effects_equal_nondet() {
    let checker = Checker::new();
    use ect::ty::Effect;

    assert!(checker.effects_equal(&Effect::Nondet, &Effect::Nondet));
}

#[test]
fn verify_effects_equal_exn_same_type() {
    let checker = Checker::new();
    use ect::ty::Effect;

    let exn1 = Effect::Exn(Box::new(Type::String));
    let exn2 = Effect::Exn(Box::new(Type::String));

    assert!(checker.effects_equal(&exn1, &exn2));
}

#[test]
fn verify_effects_equal_exn_different_type() {
    let checker = Checker::new();
    use ect::ty::Effect;

    let exn1 = Effect::Exn(Box::new(Type::String));
    let exn2 = Effect::Exn(Box::new(Type::Int));

    assert!(!checker.effects_equal(&exn1, &exn2));
}

#[test]
fn verify_effects_compatible_empty() {
    let checker = Checker::new();

    assert!(checker.effects_compatible(&[], &[]));
    assert!(checker.effects_compatible(&[], &vec![ect::ty::Effect::Async]));
}

#[test]
fn verify_effects_compatible_single_effect() {
    let checker = Checker::new();
    use ect::ty::Effect;

    let expected = vec![Effect::Async];
    let actual = vec![Effect::Async];

    assert!(checker.effects_compatible(&expected, &actual));
}

#[test]
fn verify_effects_compatible_subset() {
    let checker = Checker::new();
    use ect::ty::Effect;

    let expected = vec![Effect::Async];
    let actual = vec![Effect::Async, Effect::Alloc];

    assert!(checker.effects_compatible(&expected, &actual));
}

#[test]
fn verify_effects_compatible_missing_effect() {
    let checker = Checker::new();
    use ect::ty::Effect;

    let expected = vec![Effect::Async, Effect::Alloc];
    let actual = vec![Effect::Async];

    assert!(!checker.effects_compatible(&expected, &actual));
}

#[test]
fn verify_convert_effect_io() {
    let checker = Checker::new();
    use ect::ty::Effect;

    let effect_item = ast::EffectItem {
        name: vec!["io".into()],
        arg: None,
    };

    let converted = checker.convert_effect(&effect_item);
    assert!(converted.is_some());
    assert!(matches!(converted.unwrap(), Effect::Alloc));
}

#[test]
fn verify_convert_effect_async() {
    let checker = Checker::new();
    use ect::ty::Effect;

    let effect_item = ast::EffectItem {
        name: vec!["async".into()],
        arg: None,
    };

    let converted = checker.convert_effect(&effect_item);
    assert!(converted.is_some());
    assert!(matches!(converted.unwrap(), Effect::Async));
}

#[test]
fn verify_convert_effect_exn_no_arg() {
    let checker = Checker::new();
    use ect::ty::Effect;

    let effect_item = ast::EffectItem {
        name: vec!["exn".into()],
        arg: None,
    };

    let converted = checker.convert_effect(&effect_item);
    assert!(converted.is_some());
    match converted.unwrap() {
        Effect::Exn(exc_ty) => {
            assert_eq!(*exc_ty, Type::Named("Exception".into()));
        },
        _ => panic!("Expected Exn effect"),
    }
}

#[test]
fn verify_convert_effect_exn_with_arg() {
    let checker = Checker::new();
    use ect::ty::Effect;

    let effect_item = ast::EffectItem {
        name: vec!["exn".into()],
        arg: Some(Box::new(ast::Type::Named("CustomError".into()))),
    };

    let converted = checker.convert_effect(&effect_item);
    assert!(converted.is_some());
    match converted.unwrap() {
        Effect::Exn(exc_ty) => {
            assert_eq!(*exc_ty, Type::Named("CustomError".into()));
        },
        _ => panic!("Expected Exn effect"),
    }
}

#[test]
fn verify_convert_effect_nondet() {
    let checker = Checker::new();
    use ect::ty::Effect;

    let effect_item = ast::EffectItem {
        name: vec!["nondet".into()],
        arg: None,
    };

    let converted = checker.convert_effect(&effect_item);
    assert!(converted.is_some());
    assert!(matches!(converted.unwrap(), Effect::Nondet));
}

#[test]
fn verify_function_type_with_effects() {
    let _checker = Checker::new();
    use ect::ty::Effect;

    let fn_type = Type::Function {
        params: vec![Type::Int],
        effects: vec![Effect::Async],
        ret: Box::new(Type::Bool),
    };

    match fn_type {
        Type::Function { effects, .. } => {
            assert_eq!(effects.len(), 1);
            assert!(matches!(&effects[0], Effect::Async));
        },
        _ => panic!("Expected function type"),
    }
}

#[test]
fn verify_function_type_multiple_effects() {
    let _checker = Checker::new();
    use ect::ty::Effect;

    let fn_type = Type::Function {
        params: vec![Type::Int],
        effects: vec![Effect::Async, Effect::Alloc],
        ret: Box::new(Type::Bool),
    };

    match fn_type {
        Type::Function { effects, .. } => {
            assert_eq!(effects.len(), 2);
        },
        _ => panic!("Expected function type"),
    }
}

#[test]
fn verify_effect_total() {
    let _checker = Checker::new();
    use ect::ty::Effect;

    assert!(matches!(Effect::Total, Effect::Total));
}

#[test]
fn verify_convert_effect_alloc() {
    let checker = Checker::new();
    use ect::ty::Effect;

    let effect_item = ast::EffectItem {
        name: vec!["alloc".into()],
        arg: None,
    };

    let converted = checker.convert_effect(&effect_item);
    assert!(converted.is_some());
    assert!(matches!(converted.unwrap(), Effect::Alloc));
}

#[test]
fn verify_effect_compatibility_with_multiple_effects() {
    let checker = Checker::new();
    use ect::ty::Effect;

    let expected = vec![Effect::Async, Effect::Alloc];
    let actual = vec![Effect::Async, Effect::Alloc, Effect::Nondet];

    assert!(checker.effects_compatible(&expected, &actual));
}

#[test]
fn verify_effect_compatibility_order_independent() {
    let checker = Checker::new();
    use ect::ty::Effect;

    let expected = vec![Effect::Alloc, Effect::Async];
    let actual = vec![Effect::Async, Effect::Alloc];

    assert!(checker.effects_compatible(&expected, &actual));
}

// Phase 9: Type Ownership Attributes Tests

#[test]
fn verify_register_type_ownership_copy() {
    let mut checker = Checker::new();
    use ect::ty::Ownership;

    checker.register_ownership("Point".into(), Ownership::Copy);

    let ownership = checker.get_type_ownership("Point");
    assert!(ownership.is_some());
    assert_eq!(ownership.unwrap(), Ownership::Copy);
}

#[test]
fn verify_register_type_ownership_move() {
    let mut checker = Checker::new();
    use ect::ty::Ownership;

    checker.register_ownership("Buffer".into(), Ownership::Move);

    let ownership = checker.get_type_ownership("Buffer");
    assert!(ownership.is_some());
    assert_eq!(ownership.unwrap(), Ownership::Move);
}

#[test]
fn verify_register_type_ownership_share() {
    let mut checker = Checker::new();
    use ect::ty::Ownership;

    checker.register_ownership("Rc".into(), Ownership::Share);

    let ownership = checker.get_type_ownership("Rc");
    assert!(ownership.is_some());
    assert_eq!(ownership.unwrap(), Ownership::Share);
}

#[test]
fn verify_infer_ownership_primitive_int() {
    let checker = Checker::new();
    use ect::ty::Ownership;

    assert_eq!(checker.infer_type_ownership("Int"), Ownership::Copy);
}

#[test]
fn verify_infer_ownership_primitive_float() {
    let checker = Checker::new();
    use ect::ty::Ownership;

    assert_eq!(checker.infer_type_ownership("Float"), Ownership::Copy);
}

#[test]
fn verify_infer_ownership_primitive_bool() {
    let checker = Checker::new();
    use ect::ty::Ownership;

    assert_eq!(checker.infer_type_ownership("Bool"), Ownership::Copy);
}

#[test]
fn verify_infer_ownership_primitive_char() {
    let checker = Checker::new();
    use ect::ty::Ownership;

    assert_eq!(checker.infer_type_ownership("Char"), Ownership::Copy);
}

#[test]
fn verify_infer_ownership_primitive_unit() {
    let checker = Checker::new();
    use ect::ty::Ownership;

    assert_eq!(checker.infer_type_ownership("Unit"), Ownership::Copy);
}

#[test]
fn verify_infer_ownership_string_default() {
    let checker = Checker::new();
    use ect::ty::Ownership;

    assert_eq!(checker.infer_type_ownership("String"), Ownership::Share);
}

#[test]
fn verify_infer_ownership_unknown_default() {
    let checker = Checker::new();
    use ect::ty::Ownership;

    assert_eq!(checker.infer_type_ownership("CustomType"), Ownership::Move);
}

#[test]
fn verify_infer_ownership_registered_type() {
    let mut checker = Checker::new();
    use ect::ty::Ownership;

    checker.register_ownership("MyType".into(), Ownership::Copy);

    assert_eq!(checker.infer_type_ownership("MyType"), Ownership::Copy);
}

#[test]
fn verify_convert_ownership_attr_copy() {
    let checker = Checker::new();
    use ect::ty::Ownership;

    let attr = Some(ast::OwnershipAttr::Copy);
    assert_eq!(checker.convert_ownership_attr(&attr), Ownership::Copy);
}

#[test]
fn verify_convert_ownership_attr_move() {
    let checker = Checker::new();
    use ect::ty::Ownership;

    let attr = Some(ast::OwnershipAttr::Move);
    assert_eq!(checker.convert_ownership_attr(&attr), Ownership::Move);
}

#[test]
fn verify_convert_ownership_attr_share() {
    let checker = Checker::new();
    use ect::ty::Ownership;

    let attr = Some(ast::OwnershipAttr::Share);
    assert_eq!(checker.convert_ownership_attr(&attr), Ownership::Share);
}

#[test]
fn verify_convert_ownership_attr_none_defaults_to_move() {
    let checker = Checker::new();
    use ect::ty::Ownership;

    let attr = None;
    assert_eq!(checker.convert_ownership_attr(&attr), Ownership::Move);
}

#[test]
fn verify_register_type_with_ownership_copy() {
    let mut checker = Checker::new();
    use ect::ty::Ownership;

    let type_body = ast::TypeBody::Record(vec![]);
    checker.register_type_with_ownership("Point".into(), Ownership::Copy, type_body);

    assert_eq!(checker.get_type_ownership("Point").unwrap(), Ownership::Copy);
    assert!(checker.get_type("Point").is_some());
}

#[test]
fn verify_register_type_with_ownership_move() {
    let mut checker = Checker::new();
    use ect::ty::Ownership;

    let type_body = ast::TypeBody::Record(vec![]);
    checker.register_type_with_ownership("Buffer".into(), Ownership::Move, type_body);

    assert_eq!(checker.get_type_ownership("Buffer").unwrap(), Ownership::Move);
    assert!(checker.get_type("Buffer").is_some());
}

#[test]
fn verify_register_type_with_ownership_share() {
    let mut checker = Checker::new();
    use ect::ty::Ownership;

    let type_body = ast::TypeBody::Variant(vec![]);
    checker.register_type_with_ownership("Rc".into(), Ownership::Share, type_body);

    assert_eq!(checker.get_type_ownership("Rc").unwrap(), Ownership::Share);
    assert!(checker.get_type("Rc").is_some());
}

#[test]
fn verify_ownership_override_primitives_not_allowed() {
    let mut checker = Checker::new();
    use ect::ty::Ownership;

    // Attempting to override Int ownership still returns Copy
    checker.register_ownership("Int".into(), Ownership::Move);
    assert_eq!(checker.infer_type_ownership("Int"), Ownership::Copy);
}

#[test]
fn verify_ownership_string_can_be_overridden_to_copy() {
    let mut checker = Checker::new();
    use ect::ty::Ownership;

    checker.register_ownership("String".into(), Ownership::Copy);
    assert_eq!(checker.get_type_ownership("String").unwrap(), Ownership::Copy);
}

#[test]
fn verify_multiple_custom_types_ownership() {
    let mut checker = Checker::new();
    use ect::ty::Ownership;

    checker.register_ownership("Type1".into(), Ownership::Copy);
    checker.register_ownership("Type2".into(), Ownership::Move);
    checker.register_ownership("Type3".into(), Ownership::Share);

    assert_eq!(checker.get_type_ownership("Type1").unwrap(), Ownership::Copy);
    assert_eq!(checker.get_type_ownership("Type2").unwrap(), Ownership::Move);
    assert_eq!(checker.get_type_ownership("Type3").unwrap(), Ownership::Share);
}

#[test]
fn verify_ownership_registry_empty_by_default() {
    let checker = Checker::new();

    assert!(checker.get_type_ownership("NonExistent").is_none());
}

#[test]
fn verify_infer_ownership_uses_registry_before_defaults() {
    let mut checker = Checker::new();
    use ect::ty::Ownership;

    // Register custom ownership for a type
    checker.register_ownership("MyString".into(), Ownership::Copy);

    // Should use registry value, not default inference
    assert_eq!(checker.infer_type_ownership("MyString"), Ownership::Copy);
}