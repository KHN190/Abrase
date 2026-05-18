// Impl-lift pass.
use std::collections::HashMap;
use crate::ast::*;

pub struct ImplLowering {
    pub synthetic_fns: Vec<FnDecl>,
    pub method_dispatch: HashMap<(String, String), String>,
    pub errors: Vec<String>,
}

impl ImplLowering {
    pub fn new() -> Self {
        Self {
            synthetic_fns: Vec::new(),
            method_dispatch: HashMap::new(),
            errors: Vec::new(),
        }
    }

    pub fn lower(&mut self, decls: &[Decl]) {
        for decl in decls {
            if let Decl::Impl { methods, for_type, trait_name, generics, where_clause, .. } = decl {
                let type_name = match for_type {
                    Type::Named(n) => n.clone(),
                    Type::Qualified(parts) => parts.join("::"),
                    Type::Generic { name, .. } => name.clone(),
                    other => {
                        self.errors.push(format!(
                            "impl target type is not supported (only named/qualified/generic types): {:?}",
                            other
                        ));
                        continue;
                    }
                };
                let trait_str = match trait_name {
                    Some(parts) => Some(parts.join("::")),
                    None => None,
                };

                for m in methods {
                    let mangled = match &trait_str {
                        Some(t) => format!("{}__{}__{}", t, type_name, m.name),
                        None    => format!("{}__{}",     type_name, m.name),
                    };
                    let lifted = lift_method(m, for_type, &mangled, generics, where_clause);
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

fn lift_method(
    method: &FnDecl,
    for_type: &Type,
    mangled: &str,
    impl_generics: &[GenericParam],
    impl_where: &[WhereBound],
) -> FnDecl {
    let params: Vec<Param> = method.params.iter().map(|p| match p {
        Param::Named { pattern, ty } => Param::Named {
            pattern: pattern.clone(),
            ty: subst_self_in_type(ty, for_type),
        },
        Param::SelfVal => Param::Named {
            pattern: Spanned { node: Pattern::Bind("self".into()), span: Span::new(0, 0) },
            ty: for_type.clone(),
        },
        Param::SelfRef { is_mut } => Param::Named {
            pattern: Spanned { node: Pattern::Bind("self".into()), span: Span::new(0, 0) },
            ty: Type::Reference {
                is_mut: *is_mut,
                inner: Box::new(for_type.clone()),
                region: None,
            },
        },
    }).collect();

    let return_type = method.return_type.as_ref()
        .map(|t| subst_self_in_type(t, for_type));

    let mut generics: Vec<GenericParam> = impl_generics.to_vec();
    for g in &method.generics {
        if !generics.iter().any(|x| x.name == g.name) {
            generics.push(g.clone());
        }
    }

    let mut where_clause: Vec<WhereBound> = impl_where.to_vec();
    where_clause.extend(method.where_clause.iter().cloned());

    FnDecl {
        attrs: method.attrs.clone(),
        is_pub: method.is_pub,
        name: mangled.to_string(),
        generics,
        params,
        effects: method.effects.clone(),
        return_type,
        where_clause,
        body: method.body.clone(),
    }
}

fn subst_self_in_type(ty: &Type, for_type: &Type) -> Type {
    match ty {
        Type::Named(n) if n == "Self" => for_type.clone(),
        Type::Named(_) | Type::Qualified(_) => ty.clone(),
        Type::Generic { name, args } => Type::Generic {
            name: name.clone(),
            args: args.iter().map(|a| subst_self_in_type(a, for_type)).collect(),
        },
        Type::Array { elem, size } => Type::Array {
            elem: Box::new(subst_self_in_type(elem, for_type)),
            size: *size,
        },
        Type::Tuple(ts) => Type::Tuple(
            ts.iter().map(|t| subst_self_in_type(t, for_type)).collect()),
        Type::Reference { is_mut, inner, region } => Type::Reference {
            is_mut: *is_mut,
            inner: Box::new(subst_self_in_type(inner, for_type)),
            region: region.clone(),
        },
        Type::Function { params, effects, ret } => Type::Function {
            params: params.iter().map(|p| subst_self_in_type(p, for_type)).collect(),
            effects: effects.clone(),
            ret: Box::new(subst_self_in_type(ret, for_type)),
        },
    }
}

