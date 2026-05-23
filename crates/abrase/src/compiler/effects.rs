use crate::ast;
use super::hir::{LayoutCtx, VariantLayout, VariantShape};

pub const OK_TAG: u32 = 0;
pub const ERR_TAG: u32 = 1;
pub const RESULT_TYPE: &str = "Result";

pub fn install_result_variant(ctx: &mut LayoutCtx) {
    ctx.register_variant("Ok".to_string(), VariantLayout {
        type_name: RESULT_TYPE.to_string(),
        tag: OK_TAG,
        shape: VariantShape::Tuple(1),
        field_types: vec![ast::Type::Named("T".into())],
    });
    ctx.register_variant("Err".to_string(), VariantLayout {
        type_name: RESULT_TYPE.to_string(),
        tag: ERR_TAG,
        shape: VariantShape::Tuple(1),
        field_types: vec![ast::Type::Named("E".into())],
    });
}

pub fn fn_is_fallible(fn_decl: &ast::FnDecl) -> bool {
    fn_decl.effects.iter().any(|e| matches!(e.name.as_slice(), [n] if n == "exn"))
}
