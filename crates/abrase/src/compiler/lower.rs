use crate::ast;
use super::hir::{LayoutCtx, VariantLayout, VariantShape};

pub fn collect_layouts(ast: &[ast::Decl]) -> LayoutCtx {
    let mut ctx = LayoutCtx::new();
    for decl in ast {
        if let ast::Decl::Type { name, body, .. } = decl {
            register_type_decl(&mut ctx, name, body);
        }
    }
    ctx
}

pub fn register_type_decl(ctx: &mut LayoutCtx, name: &str, body: &ast::TypeBody) {
    match body {
        ast::TypeBody::Record(fields) => {
            let names = fields.iter().map(|f| f.name.clone()).collect();
            let types = fields.iter().map(|f| f.ty.clone()).collect();
            ctx.register_record(name.to_string(), names, types);
        }
        ast::TypeBody::Variant(cases) => {
            for (tag, case) in cases.iter().enumerate() {
                let (ctor, shape, field_types) = match case {
                    ast::VariantCase::Unit(n) => (n.clone(), VariantShape::Unit, vec![]),
                    ast::VariantCase::Tuple(n, tys) => (n.clone(), VariantShape::Tuple(tys.len()), tys.clone()),
                    ast::VariantCase::Record(n, fs) => (
                        n.clone(),
                        VariantShape::Record(fs.iter().map(|f| f.name.clone()).collect()),
                        fs.iter().map(|f| f.ty.clone()).collect(),
                    ),
                };
                ctx.register_variant(ctor, VariantLayout {
                    type_name: name.to_string(),
                    tag: tag as u32,
                    shape,
                    field_types,
                });
            }
        }
    }
}
