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
