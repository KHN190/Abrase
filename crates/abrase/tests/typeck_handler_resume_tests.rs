use abrase::ast::{self, Block, Expr, Literal, MatchArm, Pattern, Span, Spanned, Stmt, HandleArm, HandleArmKind};
use abrase::typeck::Checker;

fn sp<T>(node: T) -> Spanned<T> {
    Spanned { node, span: Span { line: 0, col: 0 } }
}

fn lit(n: i64) -> Spanned<Expr> { sp(Expr::Literal(Literal::Int(n))) }

fn handle_with_effect_arm(body: Spanned<Expr>) -> Spanned<Expr> {
    // `handle 1 { logger.log msg => <body> }` — Effect arm only.
    sp(Expr::Handle {
        expr: Box::new(lit(1)),
        arms: vec![
            HandleArm {
                kind: HandleArmKind::Effect(vec!["logger".into(), "log".into()]),
                pattern: Some(sp(Pattern::Bind("msg".into()))),
                body,
            },
        ],
    })
}

fn check(expr: &Spanned<Expr>) -> Vec<String> {
    let mut checker = Checker::new();
    checker.infer_expr(expr);
    checker.errors.into_iter().map(|e| e.message).collect()
}

fn expect_missing_resume(expr: &Spanned<Expr>) {
    let msgs = check(expr);
    assert!(
        msgs.iter().any(|m| m.contains("must call `resume`")),
        "expected must-resume error, got: {:?}", msgs,
    );
}

fn expect_no_missing_resume(expr: &Spanned<Expr>) {
    let msgs = check(expr);
    assert!(
        !msgs.iter().any(|m| m.contains("must call `resume`")),
        "expected no must-resume error, got: {:?}", msgs,
    );
}


#[test]
fn arm_with_tail_resume_accepted() {
    let body = sp(Expr::Resume(None));
    expect_no_missing_resume(&handle_with_effect_arm(body));
}

#[test]
fn arm_block_with_tail_resume_accepted() {
    // { let x = msg; resume(()) }
    let body = sp(Expr::Block(Block {
        stmts: vec![
            sp(Stmt::Let {
                pattern: sp(Pattern::Bind("x".into())),
                is_mut: false,
                ty: None,
                value: sp(Expr::Identifier("msg".into())),
            }),
        ],
        ret: Some(Box::new(sp(Expr::Resume(None)))),
    }));
    expect_no_missing_resume(&handle_with_effect_arm(body));
}

#[test]
fn arm_with_return_accepted() {
    let body = sp(Expr::Return(Some(Box::new(lit(99)))));
    expect_no_missing_resume(&handle_with_effect_arm(body));
}

#[test]
fn arm_with_throw_accepted() {
    let body = sp(Expr::Throw(Box::new(lit(1))));
    expect_no_missing_resume(&handle_with_effect_arm(body));
}


#[test]
fn arm_if_both_branches_resume_accepted() {
    let body = sp(Expr::If {
        condition: Box::new(sp(Expr::Identifier("msg".into()))),
        consequence: Box::new(sp(Expr::Resume(None))),
        alternative: Some(Box::new(sp(Expr::Resume(None)))),
    });
    expect_no_missing_resume(&handle_with_effect_arm(body));
}


#[test]
fn arm_match_all_branches_resume_accepted() {
    let body = sp(Expr::Match {
        scrutinee: Box::new(sp(Expr::Identifier("msg".into()))),
        arms: vec![
            MatchArm {
                pattern: sp(Pattern::Wildcard),
                guard: None,
                body: sp(Expr::Resume(None)),
            },
        ],
    });
    expect_no_missing_resume(&handle_with_effect_arm(body));
}


#[test]
fn arm_with_no_resume_rejected() {
    let body = lit(42);
    expect_missing_resume(&handle_with_effect_arm(body));
}


#[test]
fn arm_if_without_else_rejected() {
    let body = sp(Expr::If {
        condition: Box::new(sp(Expr::Identifier("msg".into()))),
        consequence: Box::new(sp(Expr::Resume(None))),
        alternative: None,
    });
    expect_missing_resume(&handle_with_effect_arm(body));
}


#[test]
fn arm_if_only_one_branch_resumes_rejected() {
    let body = sp(Expr::If {
        condition: Box::new(sp(Expr::Identifier("msg".into()))),
        consequence: Box::new(sp(Expr::Resume(None))),
        alternative: Some(Box::new(lit(0))),
    });
    expect_missing_resume(&handle_with_effect_arm(body));
}


#[test]
fn arm_block_with_no_resume_anywhere_rejected() {
    // { let x = 0; 1 }  — no resume / return / throw on any sub-expression.
    let body = sp(Expr::Block(Block {
        stmts: vec![
            sp(Stmt::Let {
                pattern: sp(Pattern::Bind("x".into())),
                is_mut: false,
                ty: None,
                value: lit(0),
            }),
        ],
        ret: Some(Box::new(lit(1))),
    }));
    expect_missing_resume(&handle_with_effect_arm(body));
}

// Under the current codegen, `resume(_)` lowers to a tail `Ret`, so anything
// after it is dead code — the arm still diverges via the resume call.

#[test]
fn arm_with_resume_in_binary_operand_accepted() {
    // v + resume(())  — generator pattern; `resume` on RHS makes the
    // expression divergent under the Ret-based lowering.
    let body = sp(Expr::Binary {
        op: ast::BinaryOp::Add,
        left: Box::new(sp(Expr::Identifier("msg".into()))),
        right: Box::new(sp(Expr::Resume(None))),
    });
    expect_no_missing_resume(&handle_with_effect_arm(body));
}


#[test]
fn return_arm_without_resume_accepted() {
    // handle 1 { return v => v }
    let expr = sp(Expr::Handle {
        expr: Box::new(lit(1)),
        arms: vec![
            HandleArm {
                kind: HandleArmKind::Return,
                pattern: Some(sp(Pattern::Bind("v".into()))),
                body: sp(Expr::Identifier("v".into())),
            },
        ],
    });
    expect_no_missing_resume(&expr);
}

#[test]
fn exn_arm_without_resume_accepted() {
    // handle 1 { exn e => 0 }  — Exn kind is exempt even with non-resume body.
    let expr = sp(Expr::Handle {
        expr: Box::new(lit(1)),
        arms: vec![
            HandleArm {
                kind: HandleArmKind::Exn,
                pattern: Some(sp(Pattern::Bind("e".into()))),
                body: lit(0),
            },
        ],
    });
    expect_no_missing_resume(&expr);
}
