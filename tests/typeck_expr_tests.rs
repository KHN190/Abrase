use ect::ast::{self, Pattern, Span, Spanned};
use ect::ty::Type;
use ect::typeck::Checker;

fn d_span() -> Span { Span::new(0, 0) }
fn sp<T>(node: T) -> Spanned<T> { Spanned { node, span: d_span() } }

// Basic Expressions

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
    assert!(matches!(ty, Type::Function { .. }));
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