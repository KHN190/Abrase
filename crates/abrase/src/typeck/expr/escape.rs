use crate::ast;
use crate::ty::Type;
use super::super::*;

impl Checker {
    pub(super) fn check_escape_past(
        &self,
        val: &ast::Spanned<ast::Expr>,
        barrier_depth: usize,
        require_heap: bool,
    ) -> Option<(String, ast::Span)> {
        let root = match &val.node {
            ast::Expr::Unary { op: ast::UnaryOp::Ref, right }
            | ast::Expr::Unary { op: ast::UnaryOp::RefMut, right } => {
                Self::root_ident(right)?
            }
            ast::Expr::Identifier(n) => {
                let ty = self.resolve_var_in_scopes(n)?;
                if !matches!(ty, Type::Reference { .. }) { return None; }
                n.clone()
            }
            _ => return None,
        };
        for scope in self.scopes.iter().rev() {
            if let Some(meta) = scope.vars.get(&root) {
                if meta.bound_at_region_depth >= barrier_depth
                    && (!require_heap || is_heap_typed(&meta.ty))
                {
                    return Some((root, val.span));
                }
                return None;
            }
        }
        None
    }

    pub(super) fn check_break_escape(
        &self,
        val: &ast::Spanned<ast::Expr>,
    ) -> Option<(String, ast::Span)> {
        let loop_depth = *self.loop_body_region_depth.last()?;
        self.check_escape_past(val, loop_depth, true)
    }

    pub(super) fn check_return_escape(
        &self,
        val: &ast::Spanned<ast::Expr>,
    ) -> Option<(String, ast::Span)> {
        let outer_loop_depth = *self.loop_body_region_depth.first()?;
        self.check_escape_past(val, outer_loop_depth, true)
    }

    pub(super) fn check_region_result_escape(
        &self,
        body: &ast::Block,
    ) -> Option<(String, ast::Span)> {
        let tail = body.ret.as_deref()?;
        let barrier = self.region_stack.len();
        self.check_escape_past(tail, barrier, false)
    }

    pub(super) fn root_ident(expr: &ast::Spanned<ast::Expr>) -> Option<String> {
        match &expr.node {
            ast::Expr::Identifier(n) => Some(n.clone()),
            ast::Expr::FieldAccess { base, .. } => Self::root_ident(base),
            ast::Expr::Index { base, .. } => Self::root_ident(base),
            _ => None,
        }
    }
}

fn is_heap_typed(ty: &Type) -> bool {
    match ty {
        Type::Int | Type::Float | Type::Bool | Type::Char
        | Type::Unit | Type::Never => false,
        _ => true,
    }
}

impl Checker {
    pub(super) fn arm_resumes_or_diverges(expr: &ast::Spanned<ast::Expr>) -> bool {
        match &expr.node {
            ast::Expr::Resume(_)
            | ast::Expr::Return(_)
            | ast::Expr::Throw(_) => true,
            ast::Expr::Block(b) => {
                let stmt_diverges = b.stmts.iter().any(|s| match &s.node {
                    ast::Stmt::Expr(e) => Self::arm_resumes_or_diverges(e),
                    ast::Stmt::Let { value, .. } => Self::arm_resumes_or_diverges(value),
                    ast::Stmt::Empty => false,
                });
                if stmt_diverges { return true; }
                match &b.ret {
                    Some(tail) => Self::arm_resumes_or_diverges(tail),
                    None => false,
                }
            }
            ast::Expr::If { condition, consequence, alternative: Some(alt) } => {
                Self::arm_resumes_or_diverges(condition)
                    || (Self::arm_resumes_or_diverges(consequence)
                        && Self::arm_resumes_or_diverges(alt))
            }
            ast::Expr::If { condition, alternative: None, .. } => {
                Self::arm_resumes_or_diverges(condition)
            }
            ast::Expr::Match { scrutinee, arms } => {
                if Self::arm_resumes_or_diverges(scrutinee) { return true; }
                !arms.is_empty()
                    && arms.iter().all(|a| Self::arm_resumes_or_diverges(&a.body))
            }
            ast::Expr::Binary { left, right, .. } => {
                Self::arm_resumes_or_diverges(left)
                    || Self::arm_resumes_or_diverges(right)
            }
            ast::Expr::Unary { right, .. } => Self::arm_resumes_or_diverges(right),
            ast::Expr::Call { callee, args } => {
                Self::arm_resumes_or_diverges(callee)
                    || args.iter().any(|a| Self::arm_resumes_or_diverges(a))
            }
            ast::Expr::Index { base, index } => {
                Self::arm_resumes_or_diverges(base)
                    || Self::arm_resumes_or_diverges(index)
            }
            ast::Expr::FieldAccess { base, .. } => Self::arm_resumes_or_diverges(base),
            ast::Expr::Tuple(items) | ast::Expr::Array(items) => {
                items.iter().any(|e| Self::arm_resumes_or_diverges(e))
            }
            _ => false,
        }
    }

    pub(super) fn find_nested_handle(expr: &ast::Spanned<ast::Expr>) -> Option<ast::Span> {
        match &expr.node {
            ast::Expr::Handle { .. } => Some(expr.span),
            ast::Expr::Block(b) => {
                for stmt in &b.stmts {
                    let inner = match &stmt.node {
                        ast::Stmt::Expr(e) => Self::find_nested_handle(e),
                        ast::Stmt::Let { value, .. } => Self::find_nested_handle(value),
                        ast::Stmt::Empty => None,
                    };
                    if inner.is_some() { return inner; }
                }
                b.ret.as_deref().and_then(Self::find_nested_handle)
            }
            ast::Expr::If { condition, consequence, alternative } => {
                Self::find_nested_handle(condition)
                    .or_else(|| Self::find_nested_handle(consequence))
                    .or_else(|| alternative.as_deref().and_then(Self::find_nested_handle))
            }
            ast::Expr::Match { scrutinee, arms } => {
                Self::find_nested_handle(scrutinee)
                    .or_else(|| arms.iter().find_map(|a| Self::find_nested_handle(&a.body)))
            }
            ast::Expr::Binary { left, right, .. } => {
                Self::find_nested_handle(left).or_else(|| Self::find_nested_handle(right))
            }
            ast::Expr::Unary { right, .. } => Self::find_nested_handle(right),
            ast::Expr::Call { callee, args } => {
                Self::find_nested_handle(callee)
                    .or_else(|| args.iter().find_map(Self::find_nested_handle))
            }
            ast::Expr::Index { base, index } => {
                Self::find_nested_handle(base).or_else(|| Self::find_nested_handle(index))
            }
            ast::Expr::FieldAccess { base, .. } => Self::find_nested_handle(base),
            ast::Expr::Tuple(items) | ast::Expr::Array(items) => {
                items.iter().find_map(Self::find_nested_handle)
            }
            ast::Expr::ArrayRepeat { elem, count } => {
                Self::find_nested_handle(elem).or_else(|| Self::find_nested_handle(count))
            }
            ast::Expr::Variant { args, .. } => args.iter().find_map(Self::find_nested_handle),
            ast::Expr::Resume(Some(e)) => Self::find_nested_handle(e),
            ast::Expr::Return(Some(e)) | ast::Expr::Throw(e) | ast::Expr::Question(e) => {
                Self::find_nested_handle(e)
            }
            ast::Expr::Break(Some(e)) => Self::find_nested_handle(e),
            ast::Expr::While { condition, body } => {
                Self::find_nested_handle(condition).or_else(|| Self::find_nested_handle_block(body))
            }
            ast::Expr::For { iter, body, .. } => {
                Self::find_nested_handle(iter).or_else(|| Self::find_nested_handle_block(body))
            }
            ast::Expr::Loop { body } => Self::find_nested_handle_block(body),
            ast::Expr::Region { body, .. } => Self::find_nested_handle_block(body),
            ast::Expr::Closure { body, .. } => Self::find_nested_handle(body),
            ast::Expr::Range { start, end, .. } => {
                start.as_deref().and_then(Self::find_nested_handle)
                    .or_else(|| end.as_deref().and_then(Self::find_nested_handle))
            }
            _ => None,
        }
    }

    pub(super) fn find_nested_handle_block(block: &ast::Block) -> Option<ast::Span> {
        for stmt in &block.stmts {
            let inner = match &stmt.node {
                ast::Stmt::Expr(e) => Self::find_nested_handle(e),
                ast::Stmt::Let { value, .. } => Self::find_nested_handle(value),
                ast::Stmt::Empty => None,
            };
            if inner.is_some() { return inner; }
        }
        block.ret.as_deref().and_then(Self::find_nested_handle)
    }
}
