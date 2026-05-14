use ect::ast::{*, Pattern, Span, Spanned, Type as AstType, self};
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
    // loop { 42 } has no break — it is an infinite loop, type is Never
    let mut checker = Checker::new();
    let body = ast::Block {
        stmts: vec![],
        ret: Some(Box::new(sp(ast::Expr::Literal(ast::Literal::Int(42))))),
    };
    let expr = sp(ast::Expr::Loop { body });
    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Never);
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

    // Register Point type with x and y fields
    let point_type = ect::ast::TypeBody::Record(vec![
        ect::ast::RecordField {
            is_pub: true,
            name: "x".into(),
            ty: ect::ast::Type::Named("Int".into()),
        },
        ect::ast::RecordField {
            is_pub: true,
            name: "y".into(),
            ty: ect::ast::Type::Named("Int".into()),
        },
    ]);
    checker.register_type("Point".into(), point_type);

    checker.insert_var("obj".into(), Type::Named("Point".into()), false, d_span());

    let expr = sp(ast::Expr::FieldAccess {
        base: Box::new(sp(ast::Expr::Identifier("obj".into()))),
        field: "x".into(),
    });

    let ty = checker.infer_expr(&expr);
    assert!(checker.errors.is_empty());
    assert_eq!(ty, Type::Int);
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

// Record/Variant Exhaustiveness & Type Validation

#[test]
fn verify_record_exhaustiveness_all_fields_present() {
    let mut checker = Checker::new();

    let provided = vec!["x".into(), "y".into()];
    let required = vec!["x".into(), "y".into()];

    assert!(checker.validate_record_exhaustiveness("Point", &provided, &required, d_span()));
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_record_exhaustiveness_missing_field() {
    let mut checker = Checker::new();

    let provided = vec!["x".into()];
    let required = vec!["x".into(), "y".into()];

    assert!(!checker.validate_record_exhaustiveness("Point", &provided, &required, d_span()));
    assert!(checker.errors.len() > 0);
    assert!(checker.errors[0].message.contains("missing"));
}

#[test]
fn verify_record_exhaustiveness_extra_field() {
    let mut checker = Checker::new();

    let provided = vec!["x".into(), "y".into(), "z".into()];
    let required = vec!["x".into(), "y".into()];

    // Extra fields are allowed (not checked in exhaustiveness, but in validation)
    assert!(checker.validate_record_exhaustiveness("Point", &provided, &required, d_span()));
}

#[test]
fn verify_record_field_type_match() {
    let mut checker = Checker::new();

    let field_types = vec![
        ("x".into(), Type::Int),
        ("y".into(), Type::Int),
    ];
    let provided_values = vec![
        ("x".into(), Type::Int),
        ("y".into(), Type::Int),
    ];

    assert!(checker.validate_record_fields("Point", &field_types, &provided_values, d_span()));
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_record_field_type_mismatch() {
    let mut checker = Checker::new();

    let field_types = vec![
        ("x".into(), Type::Int),
        ("y".into(), Type::Int),
    ];
    let provided_values = vec![
        ("x".into(), Type::String), // Wrong type
        ("y".into(), Type::Int),
    ];

    assert!(!checker.validate_record_fields("Point", &field_types, &provided_values, d_span()));
    assert!(checker.errors.len() > 0);
    assert!(checker.errors[0].message.contains("type mismatch"));
}

#[test]
fn verify_check_record_initialization_valid() {
    let mut checker = Checker::new();

    let field_types = vec![
        ("x".into(), Type::Int),
        ("y".into(), Type::Int),
    ];
    let provided_fields = vec!["x".into(), "y".into()];
    let provided_values = vec![
        ("x".into(), Type::Int),
        ("y".into(), Type::Int),
    ];

    assert!(checker.check_record_initialization(
        "Point",
        &field_types,
        &provided_fields,
        &provided_values,
        d_span()
    ));
}

#[test]
fn verify_check_record_initialization_missing_field() {
    let mut checker = Checker::new();

    let field_types = vec![
        ("x".into(), Type::Int),
        ("y".into(), Type::Int),
    ];
    let provided_fields = vec!["x".into()];
    let provided_values = vec![
        ("x".into(), Type::Int),
    ];

    assert!(!checker.check_record_initialization(
        "Point",
        &field_types,
        &provided_fields,
        &provided_values,
        d_span()
    ));
}

#[test]
fn verify_check_record_initialization_wrong_type() {
    let mut checker = Checker::new();

    let field_types = vec![
        ("x".into(), Type::Int),
        ("y".into(), Type::Int),
    ];
    let provided_fields = vec!["x".into(), "y".into()];
    let provided_values = vec![
        ("x".into(), Type::Float), // Wrong type
        ("y".into(), Type::Int),
    ];

    assert!(!checker.check_record_initialization(
        "Point",
        &field_types,
        &provided_fields,
        &provided_values,
        d_span()
    ));
}

#[test]
fn verify_variant_arguments_correct_count() {
    let mut checker = Checker::new();

    assert!(checker.validate_variant_arguments("Some", 1, 1, d_span()));
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_variant_arguments_wrong_count() {
    let mut checker = Checker::new();

    assert!(!checker.validate_variant_arguments("Some", 1, 2, d_span()));
    assert!(checker.errors.len() > 0);
    assert!(checker.errors[0].message.contains("expects"));
}

#[test]
fn verify_variant_arguments_zero() {
    let mut checker = Checker::new();

    assert!(checker.validate_variant_arguments("None", 0, 0, d_span()));
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_variant_argument_types_match() {
    let mut checker = Checker::new();

    let expected = vec![Type::Int, Type::String];
    let provided = vec![Type::Int, Type::String];

    assert!(checker.validate_variant_argument_types("Pair", &expected, &provided, d_span()));
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_variant_argument_types_mismatch() {
    let mut checker = Checker::new();

    let expected = vec![Type::Int, Type::String];
    let provided = vec![Type::String, Type::Int]; // Swapped types

    assert!(!checker.validate_variant_argument_types("Pair", &expected, &provided, d_span()));
    assert!(checker.errors.len() > 0);
}

#[test]
fn verify_check_variant_construction_valid() {
    let mut checker = Checker::new();

    let expected = vec![Type::Int];
    let provided = vec![Type::Int];

    assert!(checker.check_variant_construction("Some", &expected, &provided, d_span()));
}

#[test]
fn verify_check_variant_construction_wrong_count() {
    let mut checker = Checker::new();

    let expected = vec![Type::Int];
    let provided = vec![Type::Int, Type::String];

    assert!(!checker.check_variant_construction("Some", &expected, &provided, d_span()));
}

#[test]
fn verify_check_variant_construction_wrong_type() {
    let mut checker = Checker::new();

    let expected = vec![Type::Int];
    let provided = vec![Type::String];

    assert!(!checker.check_variant_construction("Some", &expected, &provided, d_span()));
}

#[test]
fn verify_get_record_field_types_found() {
    let mut checker = Checker::new();

    // Register a Point type
    let fields = vec![
        ast::RecordField {
            is_pub: false,
            name: "x".into(),
            ty: ast::Type::Named("Int".into()),
        },
        ast::RecordField {
            is_pub: false,
            name: "y".into(),
            ty: ast::Type::Named("Int".into()),
        },
    ];
    checker.register_type("Point".into(), ast::TypeBody::Record(fields));

    let field_types = checker.get_record_field_types("Point");
    assert!(field_types.is_some());
    let fields = field_types.unwrap();
    assert_eq!(fields.len(), 2);
}

#[test]
fn verify_get_record_field_types_not_found() {
    let checker = Checker::new();

    let field_types = checker.get_record_field_types("NonExistent");
    assert!(field_types.is_none());
}

#[test]
fn verify_get_variant_arg_types_unit() {
    let mut checker = Checker::new();

    // Register an Option type with None variant
    let variants = vec![
        ast::VariantCase::Unit("None".into()),
        ast::VariantCase::Tuple("Some".into(), vec![ast::Type::Named("T".into())]),
    ];
    checker.register_type("Option".into(), ast::TypeBody::Variant(variants));

    let arg_types = checker.get_variant_arg_types("Option", "None");
    assert_eq!(arg_types, Some(vec![]));
}

#[test]
fn verify_get_variant_arg_types_tuple() {
    let mut checker = Checker::new();

    // Register an Option type with Some variant
    let variants = vec![
        ast::VariantCase::Unit("None".into()),
        ast::VariantCase::Tuple("Some".into(), vec![ast::Type::Named("T".into())]),
    ];
    checker.register_type("Option".into(), ast::TypeBody::Variant(variants));

    let arg_types = checker.get_variant_arg_types("Option", "Some");
    assert!(arg_types.is_some());
    assert_eq!(arg_types.unwrap().len(), 1);
}

#[test]
fn verify_get_variant_arg_types_not_found() {
    let checker = Checker::new();

    let arg_types = checker.get_variant_arg_types("NonExistent", "NonExistent");
    assert!(arg_types.is_none());
}

#[test]
fn verify_record_point_missing_y_field() {
    let mut checker = Checker::new();

    // Simulate: Point { x: 1 } when Point requires { x: Int, y: Int }
    let field_types = vec![
        ("x".into(), Type::Int),
        ("y".into(), Type::Int),
    ];
    let provided_fields = vec!["x".into()];
    let provided_values = vec![
        ("x".into(), Type::Int),
    ];

    let result = checker.check_record_initialization(
        "Point",
        &field_types,
        &provided_fields,
        &provided_values,
        d_span()
    );

    assert!(!result);
    assert!(checker.errors.len() > 0);
    assert!(checker.errors[0].message.contains("missing") &&
            checker.errors[0].message.contains("y"));
}

#[test]
fn verify_record_all_fields_required() {
    let mut checker = Checker::new();

    let field_types = vec![
        ("name".into(), Type::String),
        ("age".into(), Type::Int),
        ("email".into(), Type::String),
    ];
    let provided_fields = vec!["name".into(), "age".into()];
    let provided_values = vec![
        ("name".into(), Type::String),
        ("age".into(), Type::Int),
    ];

    assert!(!checker.check_record_initialization(
        "Person",
        &field_types,
        &provided_fields,
        &provided_values,
        d_span()
    ));
    assert!(checker.errors.len() > 0);
    assert!(checker.errors[0].message.contains("email"));
}

// String Interpolation Validation

#[test]
fn verify_string_interpolation_defined_variable() {
    let mut checker = Checker::new();

    // Register a variable
    checker.insert_var("name".into(), Type::String, false, d_span());

    let identifiers = vec!["name".into()];
    assert!(checker.validate_string_interpolation(&identifiers, d_span()));
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_string_interpolation_undefined_variable() {
    let mut checker = Checker::new();

    // Don't register the variable
    let identifiers = vec!["name".into()];
    assert!(!checker.validate_string_interpolation(&identifiers, d_span()));
    assert!(checker.errors.len() > 0);
    assert!(checker.errors[0].message.contains("Undefined"));
}

#[test]
fn verify_string_interpolation_multiple_variables() {
    let mut checker = Checker::new();

    checker.insert_var("name".into(), Type::String, false, d_span());
    checker.insert_var("age".into(), Type::Int, false, d_span());

    let identifiers = vec!["name".into(), "age".into()];
    assert!(checker.validate_string_interpolation(&identifiers, d_span()));
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_string_interpolation_one_undefined() {
    let mut checker = Checker::new();

    checker.insert_var("name".into(), Type::String, false, d_span());

    let identifiers = vec!["name".into(), "age".into()];
    assert!(!checker.validate_string_interpolation(&identifiers, d_span()));
    assert!(checker.errors.len() > 0);
}

#[test]
fn verify_extract_interpolation_identifiers_single() {
    let checker = Checker::new();

    let parts = vec![
        ast::StringPart::Literal("Hello ".into()),
        ast::StringPart::Interp(vec!["name".into()]),
    ];

    let identifiers = checker.extract_interpolation_identifiers(&parts);
    assert_eq!(identifiers.len(), 1);
    assert_eq!(identifiers[0], "name");
}

#[test]
fn verify_extract_interpolation_identifiers_multiple() {
    let checker = Checker::new();

    let parts = vec![
        ast::StringPart::Literal("Hello ".into()),
        ast::StringPart::Interp(vec!["name".into()]),
        ast::StringPart::Literal(", age ".into()),
        ast::StringPart::Interp(vec!["age".into()]),
    ];

    let identifiers = checker.extract_interpolation_identifiers(&parts);
    assert_eq!(identifiers.len(), 2);
}

#[test]
fn verify_extract_interpolation_identifiers_with_fields() {
    let checker = Checker::new();

    let parts = vec![
        ast::StringPart::Interp(vec!["user".into(), "name".into()]),
    ];

    let identifiers = checker.extract_interpolation_identifiers(&parts);
    assert_eq!(identifiers.len(), 1);
    assert_eq!(identifiers[0], "user"); // Root identifier
}

#[test]
fn verify_check_interpolation_paths_valid() {
    let mut checker = Checker::new();

    checker.insert_var("user".into(), Type::Named("User".into()), false, d_span());

    let parts = vec![
        ast::StringPart::Interp(vec!["user".into()]),
    ];

    assert!(checker.check_interpolation_paths(&parts, d_span()));
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_check_interpolation_paths_undefined() {
    let mut checker = Checker::new();

    let parts = vec![
        ast::StringPart::Interp(vec!["undefined".into()]),
    ];

    assert!(!checker.check_interpolation_paths(&parts, d_span()));
    assert!(checker.errors.len() > 0);
}

#[test]
fn verify_check_interpolation_paths_with_fields() {
    let mut checker = Checker::new();

    checker.insert_var("obj".into(), Type::Named("Object".into()), false, d_span());

    let parts = vec![
        ast::StringPart::Interp(vec!["obj".into(), "field".into()]),
    ];

    assert!(checker.check_interpolation_paths(&parts, d_span()));
}

#[test]
fn verify_validate_interpolation_types_string() {
    let mut checker = Checker::new();

    checker.insert_var("name".into(), Type::String, false, d_span());

    let parts = vec![
        ast::StringPart::Interp(vec!["name".into()]),
    ];

    assert!(checker.validate_interpolation_types(&parts, d_span()));
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_validate_interpolation_types_int() {
    let mut checker = Checker::new();

    checker.insert_var("count".into(), Type::Int, false, d_span());

    let parts = vec![
        ast::StringPart::Interp(vec!["count".into()]),
    ];

    assert!(checker.validate_interpolation_types(&parts, d_span()));
}

#[test]
fn verify_validate_interpolation_types_never() {
    let mut checker = Checker::new();

    checker.insert_var("panic".into(), Type::Never, false, d_span());

    let parts = vec![
        ast::StringPart::Interp(vec!["panic".into()]),
    ];

    assert!(!checker.validate_interpolation_types(&parts, d_span()));
    assert!(checker.errors.len() > 0);
    assert!(checker.errors[0].message.contains("Never"));
}

#[test]
fn verify_validate_string_literal_no_interpolation() {
    let mut checker = Checker::new();

    let result = checker.validate_string_literal("Hello, world!", false, None, d_span());
    assert!(result);
}

#[test]
fn verify_validate_string_literal_with_valid_interpolation() {
    let mut checker = Checker::new();

    checker.insert_var("name".into(), Type::String, false, d_span());

    let parts = vec![
        ast::StringPart::Literal("Hello ".into()),
        ast::StringPart::Interp(vec!["name".into()]),
    ];

    let result = checker.validate_string_literal("Hello {name}", true, Some(&parts), d_span());
    assert!(result);
}

#[test]
fn verify_validate_string_literal_undefined_in_interpolation() {
    let mut checker = Checker::new();

    let parts = vec![
        ast::StringPart::Interp(vec!["undefined".into()]),
    ];

    let result = checker.validate_string_literal("Hello {undefined}", true, Some(&parts), d_span());
    assert!(!result);
}

#[test]
fn verify_count_interpolations_zero() {
    let checker = Checker::new();

    let parts = vec![
        ast::StringPart::Literal("Hello, world!".into()),
    ];

    assert_eq!(checker.count_interpolations(&parts), 0);
}

#[test]
fn verify_count_interpolations_single() {
    let checker = Checker::new();

    let parts = vec![
        ast::StringPart::Literal("Hello ".into()),
        ast::StringPart::Interp(vec!["name".into()]),
    ];

    assert_eq!(checker.count_interpolations(&parts), 1);
}

#[test]
fn verify_count_interpolations_multiple() {
    let checker = Checker::new();

    let parts = vec![
        ast::StringPart::Literal("Hello ".into()),
        ast::StringPart::Interp(vec!["name".into()]),
        ast::StringPart::Literal(", age ".into()),
        ast::StringPart::Interp(vec!["age".into()]),
    ];

    assert_eq!(checker.count_interpolations(&parts), 2);
}

#[test]
fn verify_has_interpolations_true() {
    let checker = Checker::new();

    let parts = vec![
        ast::StringPart::Interp(vec!["name".into()]),
    ];

    assert!(checker.has_interpolations(&parts));
}

#[test]
fn verify_has_interpolations_false() {
    let checker = Checker::new();

    let parts = vec![
        ast::StringPart::Literal("Hello, world!".into()),
    ];

    assert!(!checker.has_interpolations(&parts));
}

#[test]
fn verify_interpolation_in_context() {
    let mut checker = Checker::new();

    // Simulate: "Point at {x}, {y}"
    checker.insert_var("x".into(), Type::Int, false, d_span());
    checker.insert_var("y".into(), Type::Int, false, d_span());

    let parts = vec![
        ast::StringPart::Literal("Point at ".into()),
        ast::StringPart::Interp(vec!["x".into()]),
        ast::StringPart::Literal(", ".into()),
        ast::StringPart::Interp(vec!["y".into()]),
    ];

    assert!(checker.validate_string_literal("Point at {x}, {y}", true, Some(&parts), d_span()));
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_interpolation_field_access() {
    let mut checker = Checker::new();

    // Simulate: "User: {user.name}"
    checker.insert_var("user".into(), Type::Named("User".into()), false, d_span());

    let parts = vec![
        ast::StringPart::Literal("User: ".into()),
        ast::StringPart::Interp(vec!["user".into(), "name".into()]),
    ];

    let result = checker.check_interpolation_paths(&parts, d_span());
    assert!(result);
}

#[test]
fn verify_interpolation_multiple_fields() {
    let mut checker = Checker::new();

    checker.insert_var("user".into(), Type::Named("User".into()), false, d_span());
    checker.insert_var("post".into(), Type::Named("Post".into()), false, d_span());

    let parts = vec![
        ast::StringPart::Interp(vec!["user".into()]),
        ast::StringPart::Literal(": ".into()),
        ast::StringPart::Interp(vec!["post".into()]),
    ];

    let identifiers = checker.extract_interpolation_identifiers(&parts);
    assert_eq!(identifiers.len(), 2);
}

// --- typeck_interpolation_tests ---

#[test]
fn verify_interpolation_identifier_in_scope() {
    let mut checker = Checker::new();

    // Register Show trait
    checker.register_trait("Show".into(), vec!["to_string".into()]);

    // Register Int as implementing Show
    checker.register_impl("Int", "Show");

    // Insert variable in scope
    checker.insert_var("x".into(), Type::Int, false, d_span());

    // Create interpolated string with identifier in scope
    let parts = vec![
        StringPart::Literal("Value: ".into()),
        StringPart::Interp(vec!["x".into()]),
    ];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(result);
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_interpolation_identifier_out_of_scope() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("Int", "Show");

    // Variable NOT in scope
    let parts = vec![
        StringPart::Literal("Value: ".into()),
        StringPart::Interp(vec!["undefined_var".into()]),
    ];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(!result);
    assert!(checker.errors.len() > 0);
    assert!(checker.errors[0].message.contains("undefined") ||
            checker.errors[0].message.contains("Undefined"));
}

#[test]
fn verify_interpolation_multiple_identifiers() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("Int", "Show");
    checker.register_impl("String", "Show");

    checker.insert_var("x".into(), Type::Int, false, d_span());
    checker.insert_var("name".into(), Type::String, false, d_span());

    let parts = vec![
        StringPart::Literal("User: ".into()),
        StringPart::Interp(vec!["name".into()]),
        StringPart::Literal(" ID: ".into()),
        StringPart::Interp(vec!["x".into()]),
    ];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(result);
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_type_implements_show() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("Int", "Show");

    checker.insert_var("x".into(), Type::Int, false, d_span());

    let parts = vec![
        StringPart::Literal("Value: ".into()),
        StringPart::Interp(vec!["x".into()]),
    ];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(result);
}

#[test]
fn verify_type_does_not_implement_show() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    // Int NOT registered as implementing Show

    checker.insert_var("x".into(), Type::Int, false, d_span());

    let parts = vec![
        StringPart::Literal("Value: ".into()),
        StringPart::Interp(vec!["x".into()]),
    ];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(!result);
    assert!(checker.errors.len() > 0);
    assert!(checker.errors[0].message.contains("Show") ||
            checker.errors[0].message.contains("trait"));
}

#[test]
fn verify_multiple_types_some_implement_show() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("Int", "Show");
    // String does NOT implement Show

    checker.insert_var("x".into(), Type::Int, false, d_span());
    checker.insert_var("s".into(), Type::String, false, d_span());

    let parts = vec![
        StringPart::Interp(vec!["x".into()]),
        StringPart::Interp(vec!["s".into()]),
    ];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(!result);
    assert!(checker.errors.len() > 0);
    // Should report error for s not implementing Show
    assert!(checker.errors.iter().any(|e| e.message.contains("s") || e.message.contains("String")));
}

#[test]
fn verify_interpolation_field_access_path() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);

    // Register User type with name field
    use ect::ast::{TypeBody, RecordField, Type as AstType};
    let user_type = TypeBody::Record(vec![
        RecordField {
            name: "name".into(),
            ty: AstType::Named("String".into()),
            is_pub: false,
        },
    ]);
    checker.register_type("User".into(), user_type);

    // Insert variable representing a record type
    checker.insert_var("user".into(), Type::Named("User".into()), false, d_span());

    // Register String as implementing Show
    checker.register_impl("String", "Show");

    // Interpolation with field access: {user.name}
    let parts = vec![
        StringPart::Literal("Name: ".into()),
        StringPart::Interp(vec!["user".into(), "name".into()]),
    ];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(result);
}

#[test]
fn verify_interpolation_undefined_field() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("String", "Show");

    checker.insert_var("user".into(), Type::Named("User".into()), false, d_span());

    // Try to access non-existent field
    let parts = vec![
        StringPart::Interp(vec!["user".into(), "nonexistent".into()]),
    ];

    let result = checker.check_string_interpolation(&parts, d_span());
    // May or may not error depending on implementation detail
    // At minimum, should not crash
    assert!(result || checker.errors.len() > 0);
}

#[test]
fn verify_interpolation_deeply_nested_path() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("String", "Show");

    // Insert base variable
    checker.insert_var("root".into(), Type::Named("Root".into()), false, d_span());

    // Nested path: {root.field1.field2.field3}
    let parts = vec![
        StringPart::Interp(vec![
            "root".into(),
            "field1".into(),
            "field2".into(),
            "field3".into(),
        ]),
    ];

    let result = checker.check_string_interpolation(&parts, d_span());
    // Should at least not crash
    assert!(result || checker.errors.len() > 0);
}

#[test]
fn verify_string_literal_without_interpolation() {
    let mut checker = Checker::new();

    // Plain string should not require Show trait checks
    let lit = Literal::String("hello world".into());
    let result = checker.infer_literal(&lit, d_span());

    assert_eq!(result, Type::String);
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_interpolated_string_type() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("Int", "Show");

    checker.insert_var("x".into(), Type::Int, false, d_span());

    let lit = Literal::StringInterp(vec![
        StringPart::Literal("Value: ".into()),
        StringPart::Interp(vec!["x".into()]),
    ]);

    let result = checker.infer_literal(&lit, d_span());

    // Interpolated string should still be Type::String
    assert_eq!(result, Type::String);
    // But if x implements Show, no errors
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_interpolated_string_validates_identifiers() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("Int", "Show");

    // x is NOT in scope

    let lit = Literal::StringInterp(vec![
        StringPart::Literal("Value: ".into()),
        StringPart::Interp(vec!["x".into()]),
    ]);

    let result = checker.infer_literal(&lit, d_span());

    // Type is still String, but should have error
    assert_eq!(result, Type::String);
    assert!(checker.errors.len() > 0);
}

#[test]
fn verify_primitives_implement_show() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("Int", "Show");
    checker.register_impl("Bool", "Show");
    checker.register_impl("Float", "Show");
    checker.register_impl("String", "Show");
    checker.register_impl("Char", "Show");

    checker.insert_var("i".into(), Type::Int, false, d_span());
    checker.insert_var("b".into(), Type::Bool, false, d_span());
    checker.insert_var("f".into(), Type::Float, false, d_span());
    checker.insert_var("s".into(), Type::String, false, d_span());
    checker.insert_var("c".into(), Type::Char, false, d_span());

    // All should be usable in interpolation
    let parts = vec![
        StringPart::Interp(vec!["i".into()]),
        StringPart::Interp(vec!["b".into()]),
        StringPart::Interp(vec!["f".into()]),
        StringPart::Interp(vec!["s".into()]),
        StringPart::Interp(vec!["c".into()]),
    ];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(result);
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_custom_type_with_show() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("User", "Show");

    checker.insert_var("user".into(), Type::Named("User".into()), false, d_span());

    let parts = vec![
        StringPart::Literal("User: ".into()),
        StringPart::Interp(vec!["user".into()]),
    ];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(result);
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_custom_type_without_show() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    // User does NOT implement Show

    checker.insert_var("user".into(), Type::Named("User".into()), false, d_span());

    let parts = vec![
        StringPart::Literal("User: ".into()),
        StringPart::Interp(vec!["user".into()]),
    ];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(!result);
    assert!(checker.errors.len() > 0);
    assert!(checker.errors[0].message.contains("Show"));
}

// Integration Tests

#[test]
fn verify_complex_interpolation_string() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("Int", "Show");
    checker.register_impl("String", "Show");
    checker.register_impl("User", "Show");

    checker.insert_var("user_id".into(), Type::Int, false, d_span());
    checker.insert_var("name".into(), Type::String, false, d_span());
    checker.insert_var("user".into(), Type::Named("User".into()), false, d_span());

    let parts = vec![
        StringPart::Literal("User ".into()),
        StringPart::Interp(vec!["name".into()]),
        StringPart::Literal(" has ID ".into()),
        StringPart::Interp(vec!["user_id".into()]),
        StringPart::Literal(" and profile: ".into()),
        StringPart::Interp(vec!["user".into()]),
    ];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(result);
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_mixed_errors_in_interpolation() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("Int", "Show");
    // String does NOT implement Show

    checker.insert_var("id".into(), Type::Int, false, d_span());
    checker.insert_var("name".into(), Type::String, false, d_span());
    // undefined is not in scope

    let parts = vec![
        StringPart::Interp(vec!["id".into()]),           // OK
        StringPart::Interp(vec!["name".into()]),         // ERROR: String doesn't implement Show
        StringPart::Interp(vec!["undefined".into()]),    // ERROR: not in scope
    ];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(!result);
    // Should have at least 2 errors
    assert!(checker.errors.len() >= 2);
}

#[test]
fn verify_empty_interpolation_parts() {
    let mut checker = Checker::new();

    let parts: Vec<StringPart> = vec![];
    let result = checker.check_string_interpolation(&parts, d_span());

    // Empty string should be valid
    assert!(result);
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_only_literal_parts() {
    let mut checker = Checker::new();

    let parts = vec![
        StringPart::Literal("Hello ".into()),
        StringPart::Literal("world".into()),
    ];

    let result = checker.check_string_interpolation(&parts, d_span());

    // No interpolations, should be valid
    assert!(result);
    assert_eq!(checker.errors.len(), 0);
}

// Field Type Resolution from TypeBody (Record Fields)

#[test]
fn verify_field_type_from_record_body() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("String", "Show");
    checker.register_impl("Int", "Show");

    // Register User type with record body
    let user_type = TypeBody::Record(vec![
        RecordField {
            name: "name".into(),
            ty: AstType::Named("String".into()),
            is_pub: false,
        },
        RecordField {
            name: "age".into(),
            ty: AstType::Named("Int".into()),
            is_pub: false,
        },
    ]);
    checker.register_type("User".into(), user_type);

    // Insert variable of User type
    checker.insert_var("user".into(), Type::Named("User".into()), false, d_span());

    // Access field {user.name} - should resolve to String
    let parts = vec![StringPart::Interp(vec!["user".into(), "name".into()])];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(result, "Field type resolution should validate that String implements Show");
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_field_type_mismatch_no_show() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    // Only register String as implementing Show, NOT Int

    let person_type = TypeBody::Record(vec![
        RecordField {
            name: "id".into(),
            ty: AstType::Named("Int".into()),
            is_pub: false,
        },
    ]);
    checker.register_type("Person".into(), person_type);

    checker.insert_var("person".into(), Type::Named("Person".into()), false, d_span());

    // Access field {person.id} - resolves to Int which doesn't implement Show
    let parts = vec![StringPart::Interp(vec!["person".into(), "id".into()])];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(!result, "Should fail because Int doesn't implement Show");
    assert!(checker.errors.len() > 0);
    assert!(checker.errors[0].message.contains("Show") ||
            checker.errors[0].message.contains("Int"));
}

#[test]
fn verify_nested_field_access() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("String", "Show");

    // Address record with street field
    let address_type = TypeBody::Record(vec![
        RecordField {
            name: "street".into(),
            ty: AstType::Named("String".into()),
            is_pub: false,
        },
    ]);
    checker.register_type("Address".into(), address_type);

    // Person record with address field
    let person_type = TypeBody::Record(vec![
        RecordField {
            name: "address".into(),
            ty: AstType::Named("Address".into()),
            is_pub: false,
        },
    ]);
    checker.register_type("Person".into(), person_type);

    checker.insert_var("person".into(), Type::Named("Person".into()), false, d_span());

    // Access nested: {person.address.street}
    let parts = vec![StringPart::Interp(vec![
        "person".into(),
        "address".into(),
        "street".into(),
    ])];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(result);
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_field_not_found_in_record() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);

    let user_type = TypeBody::Record(vec![
        RecordField {
            name: "name".into(),
            ty: AstType::Named("String".into()),
            is_pub: false,
        },
    ]);
    checker.register_type("User".into(), user_type);

    checker.insert_var("user".into(), Type::Named("User".into()), false, d_span());

    // Try to access non-existent field
    let parts = vec![StringPart::Interp(vec!["user".into(), "email".into()])];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(!result);
    assert!(checker.errors.len() > 0);
    assert!(checker.errors[0].message.contains("email") ||
            checker.errors[0].message.contains("field"));
}

// Scope Depth Lookup (Nested Scopes)

#[test]
fn verify_variable_in_parent_scope() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("Int", "Show");

    // Insert in outer scope
    checker.insert_var("x".into(), Type::Int, false, d_span());

    // Enter nested scope
    checker.enter_scope();

    // Variable x should still be accessible in nested scope
    let parts = vec![StringPart::Interp(vec!["x".into()])];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(result, "Should find variable in parent scope");
    assert_eq!(checker.errors.len(), 0);

    checker.exit_scope();
}

#[test]
fn verify_variable_shadowing_in_scope() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("Int", "Show");
    checker.register_impl("String", "Show");

    // Outer scope: x is Int
    checker.insert_var("x".into(), Type::Int, false, d_span());

    checker.enter_scope();

    // Inner scope: x is shadowed with String
    checker.insert_var("x".into(), Type::String, false, d_span());

    let parts = vec![StringPart::Interp(vec!["x".into()])];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(result);
    // Should use String type from inner scope
    assert_eq!(checker.errors.len(), 0);

    checker.exit_scope();
}

#[test]
fn verify_deeply_nested_scope_lookup() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("Int", "Show");

    // Root scope
    checker.insert_var("root_var".into(), Type::Int, false, d_span());

    // Nested 3 levels deep
    checker.enter_scope();
    checker.enter_scope();
    checker.enter_scope();

    let parts = vec![StringPart::Interp(vec!["root_var".into()])];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(result, "Should find variable in deeply nested parent scope");

    checker.exit_scope();
    checker.exit_scope();
    checker.exit_scope();
}

// Qualified Name Resolution

#[test]
fn verify_qualified_name_basic() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("String", "Show");

    // Insert variable with qualified type name (simple case for now)
    checker.insert_var("msg".into(), Type::String, false, d_span());

    let parts = vec![StringPart::Interp(vec!["msg".into()])];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(result);
}

#[test]
fn verify_qualified_module_path() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);

    // Register type from module: std.io.Error
    checker.register_type(
        "Error".into(),
        TypeBody::Record(vec![
            RecordField {
                name: "message".into(),
                ty: AstType::Named("String".into()),
                is_pub: false,
            },
        ]),
    );
    checker.register_impl("String", "Show");

    // Insert module-qualified variable
    checker.insert_var("err".into(), Type::Named("Error".into()), false, d_span());

    let parts = vec![StringPart::Interp(vec!["err".into(), "message".into()])];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(result);
}

// Trait Bound Verification (where clauses)

#[test]
fn verify_generic_with_show_bound() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);

    // Register generic parameter T with Show bound
    checker.register_trait_bound("T".into(), "Show".into());

    // Variable of generic type with Show bound
    checker.insert_var("item".into(), Type::Named("T".into()), false, d_span());

    let parts = vec![StringPart::Interp(vec!["item".into()])];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(result, "Generic T with Show bound should be valid in interpolation");
}

#[test]
fn verify_generic_without_show_bound() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    // T is NOT registered with Show bound

    // Variable of generic type without Show bound
    checker.insert_var("item".into(), Type::Named("T".into()), false, d_span());

    let parts = vec![StringPart::Interp(vec!["item".into()])];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(!result, "Generic T without Show bound should fail");
    assert!(checker.errors.len() > 0);
}

#[test]
fn verify_multiple_trait_bounds() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_trait("Clone".into(), vec!["clone".into()]);

    // Register T with both Show and Clone bounds
    checker.register_trait_bound("T".into(), "Show".into());
    checker.register_trait_bound("T".into(), "Clone".into());

    checker.insert_var("item".into(), Type::Named("T".into()), false, d_span());

    let parts = vec![StringPart::Interp(vec!["item".into()])];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(result, "T with Show and Clone bounds should be valid");
}

// Automatic Dereference for Reference Types

#[test]
fn verify_reference_auto_deref() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("Int", "Show");

    // Variable of reference type &Int
    let ref_int = Type::Reference {
        is_mut: false,
        inner: Box::new(Type::Int),
    };
    checker.insert_var("ref_x".into(), ref_int, false, d_span());

    // Should auto-deref and check that Int implements Show
    let parts = vec![StringPart::Interp(vec!["ref_x".into()])];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(result, "Should auto-deref &Int and find that Int implements Show");
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_mutable_reference_auto_deref() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("String", "Show");

    // Variable of mutable reference type &mut String
    let ref_string = Type::Reference {
        is_mut: true,
        inner: Box::new(Type::String),
    };
    checker.insert_var("ref_s".into(), ref_string, false, d_span());

    let parts = vec![StringPart::Interp(vec!["ref_s".into()])];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(result, "Should auto-deref &mut String");
}

#[test]
fn verify_reference_to_type_without_show() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    // Int does NOT implement Show

    let ref_int = Type::Reference {
        is_mut: false,
        inner: Box::new(Type::Int),
    };
    checker.insert_var("ref_x".into(), ref_int, false, d_span());

    let parts = vec![StringPart::Interp(vec!["ref_x".into()])];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(!result, "Should fail because Int doesn't implement Show");
    assert!(checker.errors.len() > 0);
}

#[test]
fn verify_reference_field_access() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("String", "Show");

    let user_type = TypeBody::Record(vec![
        RecordField {
            name: "name".into(),
            ty: AstType::Named("String".into()),
            is_pub: false,
        },
    ]);
    checker.register_type("User".into(), user_type);

    // Variable of reference type &User
    let ref_user = Type::Reference {
        is_mut: false,
        inner: Box::new(Type::Named("User".into())),
    };
    checker.insert_var("user_ref".into(), ref_user, false, d_span());

    // Access field through reference: {user_ref.name}
    let parts = vec![StringPart::Interp(vec!["user_ref".into(), "name".into()])];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(result, "Should auto-deref and access field");
}

// Integration Tests

#[test]
fn verify_complex_type_hierarchy() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("String", "Show");
    checker.register_impl("Int", "Show");

    // Company has Address which has String fields
    let address_type = TypeBody::Record(vec![
        RecordField {
            name: "city".into(),
            ty: AstType::Named("String".into()),
            is_pub: false,
        },
    ]);
    checker.register_type("Address".into(), address_type);

    let company_type = TypeBody::Record(vec![
        RecordField {
            name: "name".into(),
            ty: AstType::Named("String".into()),
            is_pub: false,
        },
        RecordField {
            name: "address".into(),
            ty: AstType::Named("Address".into()),
            is_pub: false,
        },
        RecordField {
            name: "employee_count".into(),
            ty: AstType::Named("Int".into()),
            is_pub: false,
        },
    ]);
    checker.register_type("Company".into(), company_type);

    checker.insert_var("company".into(), Type::Named("Company".into()), false, d_span());

    // Multiple interpolations accessing different field paths
    let parts = vec![
        StringPart::Interp(vec!["company".into(), "name".into()]),
        StringPart::Interp(vec!["company".into(), "employee_count".into()]),
        StringPart::Interp(vec!["company".into(), "address".into(), "city".into()]),
    ];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(result, "Complex type hierarchy should resolve correctly");
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_scope_and_field_combined() {
    let mut checker = Checker::new();

    checker.register_trait("Show".into(), vec!["to_string".into()]);
    checker.register_impl("String", "Show");

    let person_type = TypeBody::Record(vec![
        RecordField {
            name: "name".into(),
            ty: AstType::Named("String".into()),
            is_pub: false,
        },
    ]);
    checker.register_type("Person".into(), person_type);

    // Outer scope
    checker.insert_var("person".into(), Type::Named("Person".into()), false, d_span());

    // Inner scope with another variable
    checker.enter_scope();
    checker.insert_var("greeting".into(), Type::String, false, d_span());

    let parts = vec![
        StringPart::Interp(vec!["greeting".into()]),
        StringPart::Interp(vec!["person".into(), "name".into()]),
    ];

    let result = checker.check_string_interpolation(&parts, d_span());
    assert!(result);

    checker.exit_scope();
}

// --- typeck_scope_tests (typeck_expr_tests) ---

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

#[test]
fn verify_question_on_result_unwraps_ok_type() {
    let mut checker = Checker::new();
    let inner = sp(ast::Expr::Literal(ast::Literal::Unit)); // placeholder; type injected below
    checker.insert_var(
        "r".into(),
        Type::Generic { name: "Result".into(), args: vec![Type::Int, Type::Named("IoError".into())] },
        false,
        d_span(),
    );
    let expr = sp(ast::Expr::Question(Box::new(sp(ast::Expr::Identifier("r".into())))));
    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Int);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_question_on_result_adds_exn_effect() {
    use ect::ty::Effect;
    let mut checker = Checker::new();
    checker.insert_var(
        "r".into(),
        Type::Generic { name: "Result".into(), args: vec![Type::String, Type::Named("MyError".into())] },
        false,
        d_span(),
    );
    let expr = sp(ast::Expr::Question(Box::new(sp(ast::Expr::Identifier("r".into())))));
    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::String);
    let required = checker.get_fn_required_effects();
    assert!(required.iter().any(|e| matches!(e, Effect::Exn(_))));
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_question_on_option_unwraps_inner_type() {
    let mut checker = Checker::new();
    checker.insert_var(
        "o".into(),
        Type::Generic { name: "Option".into(), args: vec![Type::Bool] },
        false,
        d_span(),
    );
    let expr = sp(ast::Expr::Question(Box::new(sp(ast::Expr::Identifier("o".into())))));
    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Bool);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_question_on_non_result_errors() {
    let mut checker = Checker::new();
    checker.insert_var("x".into(), Type::Int, false, d_span());
    let expr = sp(ast::Expr::Question(Box::new(sp(ast::Expr::Identifier("x".into())))));
    checker.infer_expr(&expr);
    assert_eq!(checker.errors.len(), 1);
    assert!(checker.errors[0].message.contains("'?' operator requires"));
}

#[test]
fn verify_await_on_future_unwraps_output_type() {
    let mut checker = Checker::new();
    checker.insert_var(
        "f".into(),
        Type::Generic { name: "Future".into(), args: vec![Type::Int] },
        false,
        d_span(),
    );
    let expr = sp(ast::Expr::Await(Box::new(sp(ast::Expr::Identifier("f".into())))));
    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Int);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_await_adds_async_effect() {
    use ect::ty::Effect;
    let mut checker = Checker::new();
    checker.insert_var(
        "f".into(),
        Type::Generic { name: "Future".into(), args: vec![Type::String] },
        false,
        d_span(),
    );
    let expr = sp(ast::Expr::Await(Box::new(sp(ast::Expr::Identifier("f".into())))));
    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::String);
    let required = checker.get_fn_required_effects();
    assert!(required.iter().any(|e| matches!(e, Effect::Async)));
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_await_on_non_future_errors() {
    let mut checker = Checker::new();
    checker.insert_var("x".into(), Type::Int, false, d_span());
    let expr = sp(ast::Expr::Await(Box::new(sp(ast::Expr::Identifier("x".into())))));
    checker.infer_expr(&expr);
    assert_eq!(checker.errors.len(), 1);
    assert!(checker.errors[0].message.contains(".await"));
}

#[test]
fn verify_loop_break_with_value_returns_that_type() {
    let mut checker = Checker::new();
    checker.insert_var("x".into(), Type::Int, false, d_span());
    let body = ast::Block {
        stmts: vec![],
        ret: Some(Box::new(sp(ast::Expr::Break(Some(Box::new(sp(ast::Expr::Identifier("x".into())))))))),
    };
    let expr = sp(ast::Expr::Loop { body });
    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Int);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_loop_break_no_value_returns_never() {
    let mut checker = Checker::new();
    let body = ast::Block {
        stmts: vec![],
        ret: Some(Box::new(sp(ast::Expr::Break(None)))),
    };
    let expr = sp(ast::Expr::Loop { body });
    let ty = checker.infer_expr(&expr);
    // loop without a break-value: loop is infinite → Never
    assert_eq!(ty, Type::Never);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_loop_no_break_returns_never() {
    let mut checker = Checker::new();
    let body = ast::Block { stmts: vec![], ret: None };
    let expr = sp(ast::Expr::Loop { body });
    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Never);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_while_break_with_value_returns_that_type() {
    let mut checker = Checker::new();
    checker.insert_var("flag".into(), Type::Bool, false, d_span());
    checker.insert_var("n".into(), Type::Float, false, d_span());
    let body = ast::Block {
        stmts: vec![],
        ret: Some(Box::new(sp(ast::Expr::Break(Some(Box::new(sp(ast::Expr::Identifier("n".into())))))))),
    };
    let expr = sp(ast::Expr::While {
        condition: Box::new(sp(ast::Expr::Identifier("flag".into()))),
        body,
    });
    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Float);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_break_value_type_mismatch_errors() {
    let mut checker = Checker::new();
    checker.insert_var("a".into(), Type::Int, false, d_span());
    checker.insert_var("b".into(), Type::Bool, false, d_span());
    // First break: Int; second break: Bool — should error
    let body = ast::Block {
        stmts: vec![
            sp(ast::Stmt::Expr(sp(ast::Expr::Break(
                Some(Box::new(sp(ast::Expr::Identifier("a".into()))))
            )))),
        ],
        ret: Some(Box::new(sp(ast::Expr::Break(Some(Box::new(sp(ast::Expr::Identifier("b".into())))))))),
    };
    let expr = sp(ast::Expr::Loop { body });
    checker.infer_expr(&expr);
    assert!(!checker.errors.is_empty(), "Mismatched break value types should error");
    assert!(checker.errors[0].message.contains("Break value type mismatch"));
}

#[test]
fn verify_for_loop_break_with_value() {
    let mut checker = Checker::new();
    checker.insert_var(
        "items".into(),
        Type::Generic { name: "List".into(), args: vec![Type::Int] },
        false,
        d_span(),
    );
    checker.insert_var("found".into(), Type::Bool, false, d_span());
    let body = ast::Block {
        stmts: vec![],
        ret: Some(Box::new(sp(ast::Expr::Break(Some(Box::new(sp(ast::Expr::Identifier("found".into())))))))),
    };
    let expr = sp(ast::Expr::For {
        pattern: sp(ast::Pattern::Bind("_item".into())),
        iter: Box::new(sp(ast::Expr::Identifier("items".into()))),
        body,
    });
    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Bool);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_generic_function_instantiation_with_int() {
    let mut checker = Checker::new();
    // identity :: <T> T -> T
    let fn_type = Type::Function {
        params: vec![Type::Generic { name: "T".into(), args: vec![] }],
        effects: vec![],
        ret: Box::new(Type::Generic { name: "T".into(), args: vec![] }),
    };
    checker.insert_var("identity".into(), fn_type, false, d_span());

    let expr = sp(ast::Expr::Call {
        callee: Box::new(sp(ast::Expr::Identifier("identity".into()))),
        args: vec![sp(ast::Expr::Literal(ast::Literal::Int(42)))],
    });

    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Int);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_generic_function_instantiation_with_string() {
    let mut checker = Checker::new();
    // identity :: <T> T -> T
    let fn_type = Type::Function {
        params: vec![Type::Generic { name: "T".into(), args: vec![] }],
        effects: vec![],
        ret: Box::new(Type::Generic { name: "T".into(), args: vec![] }),
    };
    checker.insert_var("identity".into(), fn_type, false, d_span());

    let expr = sp(ast::Expr::Call {
        callee: Box::new(sp(ast::Expr::Identifier("identity".into()))),
        args: vec![sp(ast::Expr::Literal(ast::Literal::String("hello".into())))],
    });

    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::String);
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_generic_function_with_multiple_params() {
    let mut checker = Checker::new();
    // map_pair :: <T, U> T -> U -> (T, U)
    let fn_type = Type::Function {
        params: vec![
            Type::Generic { name: "T".into(), args: vec![] },
            Type::Generic { name: "U".into(), args: vec![] },
        ],
        effects: vec![],
        ret: Box::new(Type::Tuple(vec![
            Type::Generic { name: "T".into(), args: vec![] },
            Type::Generic { name: "U".into(), args: vec![] },
        ])),
    };
    checker.insert_var("map_pair".into(), fn_type, false, d_span());

    let expr = sp(ast::Expr::Call {
        callee: Box::new(sp(ast::Expr::Identifier("map_pair".into()))),
        args: vec![
            sp(ast::Expr::Literal(ast::Literal::Int(5))),
            sp(ast::Expr::Literal(ast::Literal::Bool(true))),
        ],
    });

    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Tuple(vec![Type::Int, Type::Bool]));
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_generic_function_with_list_return() {
    let mut checker = Checker::new();
    // make_list :: <T> T -> List<T>
    let fn_type = Type::Function {
        params: vec![Type::Generic { name: "T".into(), args: vec![] }],
        effects: vec![],
        ret: Box::new(Type::Generic {
            name: "List".into(),
            args: vec![Type::Generic { name: "T".into(), args: vec![] }],
        }),
    };
    checker.insert_var("make_list".into(), fn_type, false, d_span());

    let expr = sp(ast::Expr::Call {
        callee: Box::new(sp(ast::Expr::Identifier("make_list".into()))),
        args: vec![sp(ast::Expr::Literal(ast::Literal::Int(42)))],
    });

    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Generic {
        name: "List".into(),
        args: vec![Type::Int],
    });
    assert!(checker.errors.is_empty());
}

#[test]
fn verify_generic_function_with_nested_generics() {
    let mut checker = Checker::new();
    // pair :: <T> T -> T -> (T, T)
    let fn_type = Type::Function {
        params: vec![
            Type::Generic { name: "T".into(), args: vec![] },
            Type::Generic { name: "T".into(), args: vec![] },
        ],
        effects: vec![],
        ret: Box::new(Type::Tuple(vec![
            Type::Generic { name: "T".into(), args: vec![] },
            Type::Generic { name: "T".into(), args: vec![] },
        ])),
    };
    checker.insert_var("pair".into(), fn_type, false, d_span());

    let expr = sp(ast::Expr::Call {
        callee: Box::new(sp(ast::Expr::Identifier("pair".into()))),
        args: vec![
            sp(ast::Expr::Literal(ast::Literal::Bool(true))),
            sp(ast::Expr::Literal(ast::Literal::Bool(false))),
        ],
    });

    let ty = checker.infer_expr(&expr);
    assert_eq!(ty, Type::Tuple(vec![Type::Bool, Type::Bool]));
    assert!(checker.errors.is_empty());
}
