// Compile-time type inference for: best-effort moves vs copies and method dispatch.
// The real type system lives in `typeck`.

use crate::ast;
use crate::compiler::Compiler;

impl Compiler {
    pub(in crate::compiler) fn infer_expr_type(&self, expr: &ast::Spanned<ast::Expr>) -> Option<ast::Type> {
        match &expr.node {
            ast::Expr::Literal(lit) => Some(match lit {
                ast::Literal::Int(_) => ast::Type::Named("Int".into()),
                ast::Literal::Float(_) => ast::Type::Named("Float".into()),
                ast::Literal::Bool(_) => ast::Type::Named("Bool".into()),
                ast::Literal::Char(_) => ast::Type::Named("Char".into()),
                ast::Literal::String(_) => ast::Type::Named("String".into()),
                ast::Literal::Unit => ast::Type::Tuple(vec![]),
                _ => return None,
            }),
            ast::Expr::Identifier(name) => self.var_types.get(name).cloned(),
            ast::Expr::Record { ty, .. } => ty.last().map(|n| ast::Type::Named(n.clone())),
            ast::Expr::Variant { ty, .. } => ty.last().and_then(|vname| {
                self.layouts.variants.get(vname).map(|info| ast::Type::Named(info.type_name.clone()))
            }),
            ast::Expr::Unary { op, right } => match op {
                ast::UnaryOp::Ref | ast::UnaryOp::RefMut => {
                    let inner = self.infer_expr_type(right)?;
                    Some(ast::Type::Reference {
                        is_mut: matches!(op, ast::UnaryOp::RefMut),
                        inner: Box::new(inner),
                        region: None,
                    })
                }
                ast::UnaryOp::Deref => {
                    let inner = self.infer_expr_type(right)?;
                    if let ast::Type::Reference { inner, .. } = inner { Some(*inner) } else { None }
                }
                _ => self.infer_expr_type(right),
            },
            ast::Expr::Call { callee, args: _ } => {
                // Calls to named fns: look up the fn's return type via the
                // compiler's recorded signatures. Allow overloading.
                if let ast::Expr::Identifier(name) = &callee.node {
                    let fid = self.func_map.get(name).copied()?;
                    let (_, ret) = self.fn_signatures.get(&fid)?;
                    ty_to_ast(ret)
                } else { None }
            }
            ast::Expr::Binary { op, left, right } => {
                use ast::BinaryOp as B;
                match op {
                    B::Add | B::Sub | B::Mul | B::Div | B::Mod
                    | B::AddAssign | B::SubAssign | B::MulAssign
                    | B::DivAssign | B::ModAssign => {
                        let lt = self.infer_expr_type(left)?;
                        let rt = self.infer_expr_type(right)?;
                        if lt == rt { Some(lt) } else { None }
                    }
                    B::Eq | B::Neq | B::Lt | B::Gt | B::Lte | B::Gte
                    | B::And | B::Or => Some(ast::Type::Named("Bool".into())),
                    B::Assign => None,
                }
            }
            _ => None,
        }
    }

    // Handles literals, identifiers, and references (`&x`).
    pub(in crate::compiler) fn receiver_type_name(&self, base: &ast::Spanned<ast::Expr>) -> Option<String> {
        let ty = self.infer_expr_type(base)?;
        receiver_name_of(&ty)
    }
}

// Primitive ty::Type -> ast::Type bridge. Only the cases that show up as fn
// return types in current builtins; complex returns yield None.
fn ty_to_ast(ty: &crate::ty::Type) -> Option<ast::Type> {
    use crate::ty::Type as T;
    Some(match ty {
        T::Int    => ast::Type::Named("Int".into()),
        T::Float  => ast::Type::Named("Float".into()),
        T::Bool   => ast::Type::Named("Bool".into()),
        T::Char   => ast::Type::Named("Char".into()),
        T::String => ast::Type::Named("String".into()),
        T::Unit   => ast::Type::Tuple(vec![]),
        _ => return None,
    })
}

fn receiver_name_of(ty: &ast::Type) -> Option<String> {
    match ty {
        ast::Type::Named(n) => Some(n.clone()),
        ast::Type::Reference { inner, .. } => receiver_name_of(inner),
        _ => None,
    }
}

pub(in crate::compiler) fn is_move_type(ty: &ast::Type) -> bool {
    use crate::ty::Ownership;
    let rty = ast_type_to_rt(ty);
    matches!(rty.ownership(), Ownership::Move)
}

pub(in crate::compiler) fn is_share_type(ty: &ast::Type) -> bool {
    use crate::ty::Ownership;
    let rty = ast_type_to_rt(ty);
    matches!(rty.ownership(), Ownership::Share)
}

fn ast_type_to_rt(ty: &ast::Type) -> crate::ty::Type {
    use crate::ty::Type as RTy;
    match ty {
        ast::Type::Named(n) => match n.as_str() {
            "Int" => RTy::Int,
            "Float" => RTy::Float,
            "Bool" => RTy::Bool,
            "Char" => RTy::Char,
            "String" => RTy::String,
            "Unit" => RTy::Unit,
            "Never" => RTy::Never,
            _ => RTy::Named(n.clone()),
        },
        ast::Type::Tuple(ts) if ts.is_empty() => RTy::Unit,
        ast::Type::Tuple(ts) => RTy::Tuple(ts.iter().map(ast_type_to_rt).collect()),
        ast::Type::Generic { name, args } => RTy::Generic {
            name: name.clone(),
            args: args.iter().map(ast_type_to_rt).collect(),
        },
        ast::Type::Function { params, ret, .. } => RTy::Function {
            params: params.iter().map(ast_type_to_rt).collect(),
            effects: vec![],
            ret: Box::new(ast_type_to_rt(ret)),
        },
        ast::Type::Reference { is_mut, inner, .. } => RTy::Reference {
            is_mut: *is_mut,
            inner: Box::new(ast_type_to_rt(inner)),
        },
        _ => RTy::Unknown,
    }
}
