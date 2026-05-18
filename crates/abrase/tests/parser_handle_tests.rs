use abrase::ast::*;
use abrase::lexer::Lexer;
use abrase::parser::{Parser, Precedence};

fn parse_errs(input: &str) -> Vec<String> {
    let mut p = Parser::new(Lexer::new(input));
    let _ = p.parse_program();
    p.errors.into_iter().map(|e| e.message).collect()
}

fn parse_program_no_errors(input: &str) -> Vec<Decl> {
    let mut p = Parser::new(Lexer::new(input));
    let decls = p.parse_program();
    assert!(p.errors.is_empty(), "unexpected parser errors: {:?}", p.errors);
    decls
}

fn fn_body_expr(input: &str) -> Expr {
    let mut p = Parser::new(Lexer::new(input));
    let decls = p.parse_program();
    assert!(p.errors.is_empty(), "parse errors: {:?}", p.errors.iter().map(|e| &e.message).collect::<Vec<_>>());
    let fn_decl = decls.into_iter().find_map(|d| match d {
        Decl::Fn(f) => Some(f),
        _ => None,
    }).expect("expected a function declaration");
    fn_decl.body.ret.map(|b| b.node).or_else(|| {
        fn_decl.body.stmts.last().and_then(|s| match &s.node {
            Stmt::Expr(e) => Some(e.node.clone()),
            _ => None,
        })
    }).expect("expected an expression in fn body")
}

// --- resume expression ---

#[test]
fn test_expr_resume_no_arg() {
    let input = "resume()";
    let mut parser = Parser::new(Lexer::new(input));
    let expr = parser.parse_expr(Precedence::Lowest);
    if let Expr::Resume(arg) = expr.node {
        assert!(arg.is_none());
    } else {
        panic!("Expected Resume expression, got {:?}", expr.node);
    }
}

#[test]
fn test_expr_resume_with_arg() {
    let input = "resume(42)";
    let mut parser = Parser::new(Lexer::new(input));
    let expr = parser.parse_expr(Precedence::Lowest);
    if let Expr::Resume(Some(_)) = expr.node {
        // ok
    } else {
        panic!("Expected Resume expression with arg, got {:?}", expr.node);
    }
}

#[test]
fn test_resume_expr_no_arg_in_function() {
    let input = "fn f() -> Int { resume() }";
    let mut p = Parser::new(Lexer::new(input));
    let decls = p.parse_program();
    assert!(p.errors.is_empty(), "unexpected errors: {:?}", p.errors);
    assert_eq!(decls.len(), 1);
    if let Decl::Fn(fn_decl) = &decls[0] {
        let expr = fn_decl.body.ret.as_ref().expect("expected return expr");
        assert!(matches!(expr.node, Expr::Resume(None)));
    } else { panic!("expected Fn"); }
}

#[test]
fn test_resume_expr_with_arg_in_function() {
    let input = "fn f() -> Int { resume(5) }";
    let mut p = Parser::new(Lexer::new(input));
    let decls = p.parse_program();
    assert!(p.errors.is_empty(), "unexpected errors: {:?}", p.errors);
    assert_eq!(decls.len(), 1);
    if let Decl::Fn(fn_decl) = &decls[0] {
        let expr = fn_decl.body.ret.as_ref().expect("expected return expr");
        if let Expr::Resume(Some(arg)) = &expr.node {
            assert!(matches!(arg.node, Expr::Literal(Literal::Int(5))));
        } else { panic!("expected Resume with Some arg"); }
    } else { panic!("expected Fn"); }
}

#[test]
fn test_resume_with_complex_arg() {
    let input = "fn f() -> Int { resume(x + 1) }";
    let mut p = Parser::new(Lexer::new(input));
    let decls = p.parse_program();
    assert!(p.errors.is_empty(), "unexpected errors: {:?}", p.errors);
    if let Decl::Fn(fn_decl) = &decls[0] {
        let expr = fn_decl.body.ret.as_ref().expect("expected return expr");
        if let Expr::Resume(Some(arg)) = &expr.node {
            assert!(matches!(arg.node, Expr::Binary { op: BinaryOp::Add, .. }));
        } else { panic!("expected Resume with binary expr arg"); }
    } else { panic!("expected Fn"); }
}

#[test]
fn test_resume_paren_required() {
    let errs = parse_errs("fn f() -> Int { resume }");
    assert!(!errs.is_empty(), "resume without parens should fail");
}

#[test]
fn test_resume_closing_paren_required() {
    let errs = parse_errs("fn f() -> Int { resume(1 }");
    assert!(!errs.is_empty(), "resume with missing closing paren should fail");
}

// --- handle expression (basic) ---

#[test]
fn test_expr_handle_single_return_arm() {
    let input = "handle foo { return => 0 }";
    let mut parser = Parser::new(Lexer::new(input));
    let expr = parser.parse_expr(Precedence::Lowest);
    if let Expr::Handle { expr: _, arms } = expr.node {
        assert_eq!(arms.len(), 1);
        assert!(matches!(arms[0].kind, HandleArmKind::Return));
    } else {
        panic!("Expected Handle expression");
    }
}

#[test]
fn test_expr_handle_return_and_exn_comma() {
    let input = "handle computation { return x => x, exn e => 0 }";
    let mut parser = Parser::new(Lexer::new(input));
    let expr = parser.parse_expr(Precedence::Lowest);
    if let Expr::Handle { expr: _, arms } = expr.node {
        assert_eq!(arms.len(), 2);
        assert!(matches!(arms[0].kind, HandleArmKind::Return));
        assert!(matches!(arms[1].kind, HandleArmKind::Exn));
    } else {
        panic!("Expected Handle expression");
    }
}

// --- handle arm separator (the fixed case) ---

#[test]
fn test_handle_arms_no_comma_both_parsed() {
    let decls = parse_program_no_errors(
        "fn main() -> Int { handle f() { return v => v exn e => 0 } }",
    );
    let fn_decl = decls.into_iter().find_map(|d| match d { Decl::Fn(f) => Some(f), _ => None }).unwrap();
    let ret = fn_decl.body.ret.unwrap();
    let Expr::Handle { arms, .. } = ret.node else { panic!("expected Handle"); };
    assert_eq!(arms.len(), 2, "both arms must be parsed without comma");
    assert!(matches!(arms[0].kind, HandleArmKind::Return));
    assert!(matches!(arms[1].kind, HandleArmKind::Exn));
}

// --- full coverage: primes_gen.abe pattern ---

#[test]
fn test_handle_return_then_effect_no_comma() {
    // exact primes_gen.abe shape: return arm followed by effect arm, no comma
    let body = fn_body_expr(
        "fn f() -> Int { handle g() { return _ => 0 gen.yield v => 1 } }",
    );
    let Expr::Handle { arms, .. } = body else { panic!("expected Handle"); };
    assert_eq!(arms.len(), 2, "return + effect arm must both be parsed");
    assert!(matches!(arms[0].kind, HandleArmKind::Return));
    let HandleArmKind::Effect(ref path) = arms[1].kind else {
        panic!("expected Effect arm kind, got {:?}", arms[1].kind);
    };
    assert_eq!(path, &["gen".to_string(), "yield".to_string()]);
}

#[test]
fn test_handle_effect_arm_kind_path() {
    // verify Effect(["gen","yield"]) kind stored in arm
    let body = fn_body_expr(
        "fn f() -> Int { handle g() { gen.yield v => 0 } }",
    );
    let Expr::Handle { arms, .. } = body else { panic!("expected Handle"); };
    assert_eq!(arms.len(), 1);
    let HandleArmKind::Effect(ref path) = arms[0].kind else {
        panic!("expected Effect arm kind, got {:?}", arms[0].kind);
    };
    assert_eq!(path, &["gen".to_string(), "yield".to_string()]);
}

#[test]
fn test_handle_effect_arm_with_resume_in_body() {
    // v + resume(()) — resume used inside arm body
    let body = fn_body_expr(
        "fn f() -> Int { handle g() { gen.yield v => v + resume(()) } }",
    );
    let Expr::Handle { arms, .. } = body else { panic!("expected Handle"); };
    assert_eq!(arms.len(), 1);
    assert!(matches!(arms[0].kind, HandleArmKind::Effect(_)));
    let Expr::Binary { op: BinaryOp::Add, right, .. } = &arms[0].body.node else {
        panic!("expected Add binary in arm body, got {:?}", arms[0].body.node);
    };
    assert!(matches!(right.node, Expr::Resume(Some(_))),
        "right side of + must be resume(...), got {:?}", right.node);
}

#[test]
fn test_handle_three_arms_no_comma() {
    // return + exn + effect, no separators between any of them
    let body = fn_body_expr(
        "fn f() -> Int { handle g() { return _ => 0 exn e => 1 gen.yield v => 2 } }",
    );
    let Expr::Handle { arms, .. } = body else { panic!("expected Handle"); };
    assert_eq!(arms.len(), 3, "all three arms must be parsed");
    assert!(matches!(arms[0].kind, HandleArmKind::Return));
    assert!(matches!(arms[1].kind, HandleArmKind::Exn));
    assert!(matches!(arms[2].kind, HandleArmKind::Effect(_)));
}

#[test]
fn test_handle_effect_block_body_then_return() {
    // effect arm with block body, followed by return arm — no comma
    let body = fn_body_expr(
        "fn f() -> Int { handle g() { gen.yield v => { v } return _ => 0 } }",
    );
    let Expr::Handle { arms, .. } = body else { panic!("expected Handle"); };
    assert_eq!(arms.len(), 2);
    assert!(matches!(arms[0].kind, HandleArmKind::Effect(_)));
    assert!(matches!(arms[0].body.node, Expr::Block(_)));
    assert!(matches!(arms[1].kind, HandleArmKind::Return));
}

#[test]
fn test_handle_return_block_body_then_effect() {
    // return arm with block body, followed by effect arm — no comma
    let body = fn_body_expr(
        "fn f() -> Int { handle g() { return _ => { 0 } gen.yield v => 1 } }",
    );
    let Expr::Handle { arms, .. } = body else { panic!("expected Handle"); };
    assert_eq!(arms.len(), 2);
    assert!(matches!(arms[0].kind, HandleArmKind::Return));
    assert!(matches!(arms[0].body.node, Expr::Block(_)));
    assert!(matches!(arms[1].kind, HandleArmKind::Effect(_)));
}

#[test]
fn test_handle_comma_still_valid() {
    // comma-separated arms must still work after the separator fix
    let body = fn_body_expr(
        "fn f() -> Int { handle g() { return _ => 0, gen.yield v => 1 } }",
    );
    let Expr::Handle { arms, .. } = body else { panic!("expected Handle"); };
    assert_eq!(arms.len(), 2, "comma-separated arms regression");
    assert!(matches!(arms[0].kind, HandleArmKind::Return));
    assert!(matches!(arms[1].kind, HandleArmKind::Effect(_)));
}

#[test]
fn test_handle_mixed_separator() {
    // first separator is comma, second is absent
    let body = fn_body_expr(
        "fn f() -> Int { handle g() { return _ => 0, exn e => 1 gen.yield v => 2 } }",
    );
    let Expr::Handle { arms, .. } = body else { panic!("expected Handle"); };
    assert_eq!(arms.len(), 3);
}

#[test]
fn test_handle_single_effect_arm() {
    // handle block with only an effect arm (no return arm)
    let body = fn_body_expr(
        "fn f() -> Int { handle g() { gen.yield v => v } }",
    );
    let Expr::Handle { arms, .. } = body else { panic!("expected Handle"); };
    assert_eq!(arms.len(), 1);
    assert!(matches!(arms[0].kind, HandleArmKind::Effect(_)));
}

#[test]
fn test_handle_effect_multi_segment_path() {
    // three-segment dotted effect path
    let body = fn_body_expr(
        "fn f() -> Int { handle g() { a.b.c v => 0 } }",
    );
    let Expr::Handle { arms, .. } = body else { panic!("expected Handle"); };
    assert_eq!(arms.len(), 1);
    let HandleArmKind::Effect(ref path) = arms[0].kind else {
        panic!("expected Effect kind");
    };
    assert_eq!(path, &["a".to_string(), "b".to_string(), "c".to_string()]);
}

#[test]
fn test_handle_effect_arm_pattern_bound_name() {
    // arm pattern name is available as Some(Pattern::Bind)
    let body = fn_body_expr(
        "fn f() -> Int { handle g() { gen.yield myval => myval } }",
    );
    let Expr::Handle { arms, .. } = body else { panic!("expected Handle"); };
    assert_eq!(arms.len(), 1);
    let pat = arms[0].pattern.as_ref().expect("effect arm must have a pattern");
    assert_eq!(pat.node, Pattern::Bind("myval".into()));
}

#[test]
fn test_handle_return_arm_wildcard_pattern() {
    let body = fn_body_expr(
        "fn f() -> Int { handle g() { return _ => 0 } }",
    );
    let Expr::Handle { arms, .. } = body else { panic!("expected Handle"); };
    assert_eq!(arms.len(), 1);
    assert!(matches!(arms[0].kind, HandleArmKind::Return));
    let pat = arms[0].pattern.as_ref().expect("return _ arm must have wildcard pattern");
    assert_eq!(pat.node, Pattern::Wildcard);
}
