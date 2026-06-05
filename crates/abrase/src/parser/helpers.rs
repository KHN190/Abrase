use crate::ast::Expr;

pub fn is_block_terminated(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Block(_)
            | Expr::If { .. }
            | Expr::Match { .. }
            | Expr::While { .. }
            | Expr::For { .. }
            | Expr::Loop { .. }
            | Expr::Region { .. }
            | Expr::Handle { .. }
            | Expr::Record { .. }
    )
}

// True when the expression's PARSE consumed past its last token (cursor sits
// beyond it): the expression itself is block-terminated, or its rightmost
// subexpression is (binary/unary/throw/range chains end in their right arm).
pub fn rightmost_block_terminated(expr: &Expr) -> bool {
    if is_block_terminated(expr) { return true; }
    match expr {
        Expr::Binary { right, .. }
        | Expr::Unary { right, .. }
        | Expr::Throw(right) => rightmost_block_terminated(&right.node),
        Expr::Range { end: Some(end), .. } => rightmost_block_terminated(&end.node),
        Expr::Break(Some(e)) | Expr::Return(Some(e)) | Expr::Resume(Some(e)) =>
            rightmost_block_terminated(&e.node),
        _ => false,
    }
}
