use std::collections::HashSet;
use abrase::ast::*;
use abrase::compiler::closures::{collect_free_vars, collect_assigned_idents};

fn s() -> Span { Span::new(0, 0) }
fn sp<T>(node: T) -> Spanned<T> { Spanned { node, span: s() } }
fn free(name: &str) -> Spanned<Expr> { sp(Expr::Identifier(name.into())) }
fn lit_int() -> Spanned<Expr> { sp(Expr::Literal(Literal::Int(0))) }
fn empty_block() -> Block { Block { stmts: vec![], ret: None } }
fn block_ret(e: Spanned<Expr>) -> Block { Block { stmts: vec![], ret: Some(Box::new(e)) } }

fn free_vars(expr: &Spanned<Expr>, bound: &[&str]) -> Vec<String> {
    let bound: HashSet<String> = bound.iter().map(|s| s.to_string()).collect();
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    collect_free_vars(expr, &bound, &mut seen, &mut out);
    out
}

fn assigned(expr: &Spanned<Expr>, candidates: &[&str]) -> HashSet<String> {
    let cands: HashSet<String> = candidates.iter().map(|s| s.to_string()).collect();
    let mut out = HashSet::new();
    collect_assigned_idents(expr, &cands, &mut out);
    out
}

#[test]
fn collect_free_vars_recurses_into_compound_exprs() {
    let cases: Vec<(&str, Spanned<Expr>)> = vec![
        ("for iter",  sp(Expr::For { pattern: sp(Pattern::Bind("i".into())), iter: Box::new(free("x")), body: empty_block() })),
        ("for body",  sp(Expr::For { pattern: sp(Pattern::Bind("i".into())), iter: Box::new(lit_int()), body: block_ret(free("x")) })),
        ("loop",      sp(Expr::Loop { body: block_ret(free("x")) })),
        ("region",    sp(Expr::Region { label: None, body: block_ret(free("x")) })),
        ("handle arm",sp(Expr::Handle {
            expr: Box::new(lit_int()),
            arms: vec![HandleArm { kind: HandleArmKind::Return, pattern: None, body: free("x") }],
        })),
        ("arrayrepeat elem", sp(Expr::ArrayRepeat { elem: Box::new(free("x")), count: Box::new(lit_int()) })),
        ("arrayrepeat count",sp(Expr::ArrayRepeat { elem: Box::new(lit_int()), count: Box::new(free("x")) })),
    ];

    for (label, expr) in &cases {
        let vars = free_vars(expr, &[]);
        assert!(vars.contains(&"x".to_string()), "{label}: expected x in free vars, got {vars:?}");
    }
}

#[test]
fn collect_free_vars_for_pattern_shadows_body_var() {
    let expr = sp(Expr::For {
        pattern: sp(Pattern::Bind("i".into())),
        iter: Box::new(lit_int()),
        body: block_ret(free("i")),
    });
    assert!(!free_vars(&expr, &[]).contains(&"i".to_string()),
        "i is bound by for pattern, must not appear as free");
}

#[test]
fn collect_assigned_idents_recurses_into_compound_exprs() {
    let assign_x = sp(Expr::Binary {
        op: BinaryOp::Assign,
        left: Box::new(free("x")),
        right: Box::new(lit_int()),
    });
    let body_with_assign = Block { stmts: vec![sp(Stmt::Expr(assign_x.clone()))], ret: None };

    let cases: Vec<(&str, Spanned<Expr>)> = vec![
        ("if consequence", sp(Expr::If { condition: Box::new(lit_int()), consequence: Box::new(assign_x.clone()), alternative: None })),
        ("if alternative", sp(Expr::If { condition: Box::new(lit_int()), consequence: Box::new(lit_int()), alternative: Some(Box::new(assign_x.clone())) })),
        ("match arm",      sp(Expr::Match { scrutinee: Box::new(lit_int()), arms: vec![MatchArm { pattern: sp(Pattern::Wildcard), guard: None, body: assign_x.clone() }] })),
        ("while body",     sp(Expr::While { condition: Box::new(lit_int()), body: body_with_assign.clone() })),
        ("for body",       sp(Expr::For { pattern: sp(Pattern::Bind("i".into())), iter: Box::new(lit_int()), body: body_with_assign.clone() })),
        ("loop body",      sp(Expr::Loop { body: body_with_assign.clone() })),
        ("region body",    sp(Expr::Region { label: None, body: body_with_assign.clone() })),
        ("handle arm",     sp(Expr::Handle { expr: Box::new(lit_int()), arms: vec![HandleArm { kind: HandleArmKind::Return, pattern: None, body: assign_x.clone() }] })),
        ("index base",     sp(Expr::Index { base: Box::new(assign_x.clone()), index: Box::new(lit_int()) })),
        ("fieldaccess base",sp(Expr::FieldAccess { base: Box::new(assign_x.clone()), field: "f".into() })),
        ("tuple elem",     sp(Expr::Tuple(vec![assign_x.clone()]))),
        ("arrayrepeat elem",sp(Expr::ArrayRepeat { elem: Box::new(assign_x.clone()), count: Box::new(lit_int()) })),
        ("closure body",   sp(Expr::Closure { is_move: false, params: vec![], effects: vec![], return_type: None, body: Box::new(assign_x.clone()) })),
    ];

    for (label, expr) in &cases {
        assert!(assigned(&expr, &["x"]).contains("x"), "{label}: x must be detected as assigned");
    }
}
