use std::collections::HashSet;
use crate::ast::{Block, Expr, Spanned, Stmt, Pattern, BinaryOp, UnaryOp};

pub fn hoist_invariants(body: &Block, outer_scope: &HashSet<String>) -> (Vec<Spanned<Stmt>>, Block) {
    let modified = collect_modified_in_block(body);
    let mut hoist_eligible: HashSet<String> = HashSet::new();
    let mut hoisted: Vec<Spanned<Stmt>> = Vec::new();
    let mut remaining: Vec<Spanned<Stmt>> = Vec::new();

    for stmt in &body.stmts {
        let mut keep = true;
        if let Stmt::Let { pattern, value, .. } = &stmt.node {
            if let Pattern::Bind(name) = &pattern.node {
                if !modified.contains(name) {
                    let mut allowed: HashSet<String> = outer_scope.clone();
                    for h in &hoist_eligible { allowed.insert(h.clone()); }
                    if is_pure_invariant(&value.node, &allowed, &modified) {
                        hoist_eligible.insert(name.clone());
                        hoisted.push(stmt.clone());
                        keep = false;
                    }
                }
            }
        }
        if keep {
            remaining.push(stmt.clone());
        }
    }

    (hoisted, Block { stmts: remaining, ret: body.ret.clone() })
}

fn collect_modified_in_block(block: &Block) -> HashSet<String> {
    let mut out = HashSet::new();
    for stmt in &block.stmts {
        collect_modified_in_stmt(stmt, &mut out);
    }
    if let Some(ret) = &block.ret {
        collect_modified_in_expr(ret, &mut out);
    }
    out
}

fn collect_modified_in_stmt(stmt: &Spanned<Stmt>, out: &mut HashSet<String>) {
    match &stmt.node {
        Stmt::Expr(e) => collect_modified_in_expr(e, out),
        Stmt::Let { value, .. } => collect_modified_in_expr(value, out),
        Stmt::Empty => {}
    }
}

fn collect_modified_in_expr(expr: &Spanned<Expr>, out: &mut HashSet<String>) {
    match &expr.node {
        Expr::Binary { op, left, right } if is_assign(op) => {
            if let Expr::Identifier(n) = &left.node { out.insert(n.clone()); }
            collect_modified_in_expr(right, out);
        }
        Expr::Binary { left, right, .. } => {
            collect_modified_in_expr(left, out);
            collect_modified_in_expr(right, out);
        }
        Expr::Unary { right, .. } | Expr::Paren(right) => collect_modified_in_expr(right, out),
        Expr::If { condition, consequence, alternative } => {
            collect_modified_in_expr(condition, out);
            collect_modified_in_expr(consequence, out);
            if let Some(a) = alternative.as_deref() { collect_modified_in_expr(a, out); }
        }
        Expr::While { condition, body } => {
            collect_modified_in_expr(condition, out);
            for stmt in &body.stmts { collect_modified_in_stmt(stmt, out); }
            if let Some(ret) = &body.ret { collect_modified_in_expr(ret, out); }
        }
        Expr::Block(b) => {
            for stmt in &b.stmts { collect_modified_in_stmt(stmt, out); }
            if let Some(ret) = &b.ret { collect_modified_in_expr(ret, out); }
        }
        Expr::Call { callee, args } => {
            collect_modified_in_expr(callee, out);
            for a in args { collect_modified_in_expr(a, out); }
        }
        Expr::Match { scrutinee, arms } => {
            collect_modified_in_expr(scrutinee, out);
            for arm in arms { collect_modified_in_expr(&arm.body, out); }
        }
        _ => {}
    }
}

fn is_assign(op: &BinaryOp) -> bool {
    matches!(op,
        BinaryOp::Assign | BinaryOp::AddAssign | BinaryOp::SubAssign
        | BinaryOp::MulAssign | BinaryOp::DivAssign | BinaryOp::ModAssign)
}

fn is_pure_invariant(expr: &Expr, allowed: &HashSet<String>, modified: &HashSet<String>) -> bool {
    match expr {
        Expr::Literal(_) => true,
        Expr::Identifier(n) => allowed.contains(n) && !modified.contains(n),
        Expr::Binary { op, left, right } => {
            !is_assign(op)
            && is_pure_invariant(&left.node, allowed, modified)
            && is_pure_invariant(&right.node, allowed, modified)
        }
        Expr::Unary { op, right } => {
            !matches!(op, UnaryOp::Ref | UnaryOp::RefMut | UnaryOp::Deref)
            && is_pure_invariant(&right.node, allowed, modified)
        }
        _ => false,
    }
}
