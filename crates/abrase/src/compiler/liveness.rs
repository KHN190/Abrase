use std::collections::HashMap;
use crate::ast::{Block, Expr, Stmt};

/// Counts how many times each identifier appears in `block`.
///
/// Loop bodies (For, While, Loop) and closure bodies use `usize::MAX` as a
/// sentinel so that variables used inside them are never considered "last use"
/// — the body can execute multiple times, so ownership cannot be transferred.
pub fn count_uses(block: &Block) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    count_block(block, &mut counts, false);
    counts
}

fn add(counts: &mut HashMap<String, usize>, name: &str, in_loop: bool) {
    let entry = counts.entry(name.to_string()).or_insert(0);
    if in_loop {
        *entry = usize::MAX;
    } else {
        *entry = entry.saturating_add(1);
    }
}

fn count_block(block: &Block, counts: &mut HashMap<String, usize>, in_loop: bool) {
    for stmt in &block.stmts {
        count_stmt(&stmt.node, counts, in_loop);
    }
    if let Some(ret) = &block.ret {
        count_expr(&ret.node, counts, in_loop);
    }
}

fn count_stmt(stmt: &Stmt, counts: &mut HashMap<String, usize>, in_loop: bool) {
    match stmt {
        Stmt::Let { value, .. } => count_expr(&value.node, counts, in_loop),
        Stmt::Expr(e) => count_expr(&e.node, counts, in_loop),
        Stmt::Empty => {}
    }
}

fn count_expr(expr: &Expr, counts: &mut HashMap<String, usize>, in_loop: bool) {
    match expr {
        Expr::Identifier(name) => add(counts, name, in_loop),

        Expr::Call { callee, args } => {
            count_expr(&callee.node, counts, in_loop);
            for arg in args { count_expr(&arg.node, counts, in_loop); }
        }

        Expr::Binary { left, right, .. } => {
            count_expr(&left.node, counts, in_loop);
            count_expr(&right.node, counts, in_loop);
        }
        Expr::Unary { right, .. } => count_expr(&right.node, counts, in_loop),

        Expr::If { condition, consequence, alternative } => {
            count_expr(&condition.node, counts, in_loop);
            // Both branches are counted conservatively (only one runs at runtime,
            // but we don't know which, so we can't call either the "last" use).
            count_expr(&consequence.node, counts, in_loop);
            if let Some(alt) = alternative {
                count_expr(&alt.node, counts, in_loop);
            }
        }

        Expr::Block(block) => count_block(block, counts, in_loop),
        Expr::Region { body, .. } => count_block(body, counts, in_loop),

        Expr::Match { scrutinee, arms } => {
            count_expr(&scrutinee.node, counts, in_loop);
            for arm in arms {
                if let Some(g) = &arm.guard { count_expr(&g.node, counts, in_loop); }
                count_expr(&arm.body.node, counts, in_loop);
            }
        }

        // Loop bodies: any use inside may execute many times.
        Expr::For { iter, body, .. } => {
            count_expr(&iter.node, counts, in_loop);
            count_block(body, counts, true);
        }
        Expr::While { condition, body } => {
            count_expr(&condition.node, counts, true);
            count_block(body, counts, true);
        }
        Expr::Loop { body } => count_block(body, counts, true),

        // Closures may be called many times; treat body like a loop.
        Expr::Closure { body, .. } => count_expr(&body.node, counts, true),

        Expr::Return(Some(e)) | Expr::Break(Some(e)) => {
            count_expr(&e.node, counts, in_loop);
        }
        Expr::Throw(e) | Expr::Question(e) => count_expr(&e.node, counts, in_loop),

        Expr::Tuple(elems) | Expr::Array(elems) => {
            for e in elems { count_expr(&e.node, counts, in_loop); }
        }
        Expr::Variant { args, .. } => {
            for a in args { count_expr(&a.node, counts, in_loop); }
        }
        Expr::ArrayRepeat { elem, count: c } => {
            count_expr(&elem.node, counts, in_loop);
            count_expr(&c.node, counts, in_loop);
        }
        Expr::Record { fields, .. } => {
            for f in fields {
                if let Some(v) = &f.value { count_expr(&v.node, counts, in_loop); }
            }
        }

        Expr::FieldAccess { base, .. } => count_expr(&base.node, counts, in_loop),
        Expr::Index { base, index } => {
            count_expr(&base.node, counts, in_loop);
            count_expr(&index.node, counts, in_loop);
        }

        Expr::Range { start, end, .. } => {
            if let Some(s) = start { count_expr(&s.node, counts, in_loop); }
            if let Some(e) = end { count_expr(&e.node, counts, in_loop); }
        }

        Expr::Handle { expr, arms } => {
            count_expr(&expr.node, counts, in_loop);
            for arm in arms { count_expr(&arm.body.node, counts, in_loop); }
        }
        Expr::Resume(Some(e)) => count_expr(&e.node, counts, in_loop),

        Expr::Literal(crate::ast::Literal::StringInterp(parts)) => {
            for part in parts {
                if let crate::ast::StringPart::Interp(segments) = part {
                    if let Some(name) = segments.first() { add(counts, name, in_loop); }
                }
            }
        }

        _ => {}
    }
}
