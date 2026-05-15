// Impl-lift pass.
use std::collections::HashMap;
use crate::ast::*;

pub struct ImplLowering {
    pub synthetic_fns: Vec<FnDecl>,
    pub method_dispatch: HashMap<(String, String), String>,
}

impl ImplLowering {
    pub fn new() -> Self {
        Self {
            synthetic_fns: Vec::new(),
            method_dispatch: HashMap::new(),
        }
    }

    pub fn lower(&mut self, decls: &[Decl]) {
        for decl in decls {
            if let Decl::Impl { methods, for_type, trait_name, .. } = decl {
                let type_name = match for_type {
                    Type::Named(n) => n.clone(),
                    Type::Qualified(parts) => parts.join("::"),
                    _ => continue,
                };
                let trait_str = match trait_name {
                    Some(parts) => parts.join("::"),
                    None => continue, // inherent impl — not handled by Feature 22
                };

                for m in methods {
                    let mangled = format!("{}__{}__{}", trait_str, type_name, m.name);
                    let lifted = lift_method(m, &type_name, &mangled);
                    self.method_dispatch.insert(
                        (type_name.clone(), m.name.clone()),
                        mangled.clone(),
                    );
                    self.synthetic_fns.push(lifted);
                }
            }
        }
    }
}

fn lift_method(method: &FnDecl, receiver_type: &str, mangled: &str) -> FnDecl {
    let params: Vec<Param> = method.params.iter().map(|p| match p {
        Param::Named { pattern, ty } => Param::Named {
            pattern: pattern.clone(),
            ty: subst_self_in_type(ty, receiver_type),
        },
        Param::SelfVal => Param::Named {
            pattern: Spanned { node: Pattern::Bind("self".into()), span: Span::new(0, 0) },
            ty: Type::Named(receiver_type.to_string()),
        },
        Param::SelfRef { is_mut } => Param::Named {
            pattern: Spanned { node: Pattern::Bind("self".into()), span: Span::new(0, 0) },
            ty: Type::Reference {
                is_mut: *is_mut,
                inner: Box::new(Type::Named(receiver_type.to_string())),
                region: None,
            },
        },
    }).collect();

    let return_type = method.return_type.as_ref()
        .map(|t| subst_self_in_type(t, receiver_type));

    FnDecl {
        attrs: method.attrs.clone(),
        is_pub: method.is_pub,
        name: mangled.to_string(),
        generics: Vec::new(),
        params,
        effects: method.effects.clone(),
        return_type,
        where_clause: Vec::new(),
        body: method.body.clone(),
    }
}

fn subst_self_in_type(ty: &Type, receiver_type: &str) -> Type {
    match ty {
        Type::Named(n) if n == "Self" => Type::Named(receiver_type.to_string()),
        Type::Named(_) | Type::Qualified(_) => ty.clone(),
        Type::Generic { name, args } => Type::Generic {
            name: name.clone(),
            args: args.iter().map(|a| subst_self_in_type(a, receiver_type)).collect(),
        },
        Type::Array { elem, size } => Type::Array {
            elem: Box::new(subst_self_in_type(elem, receiver_type)),
            size: *size,
        },
        Type::Tuple(ts) => Type::Tuple(
            ts.iter().map(|t| subst_self_in_type(t, receiver_type)).collect()),
        Type::Reference { is_mut, inner, region } => Type::Reference {
            is_mut: *is_mut,
            inner: Box::new(subst_self_in_type(inner, receiver_type)),
            region: region.clone(),
        },
        Type::Function { params, effects, ret } => Type::Function {
            params: params.iter().map(|p| subst_self_in_type(p, receiver_type)).collect(),
            effects: effects.clone(),
            ret: Box::new(subst_self_in_type(ret, receiver_type)),
        },
    }
}
