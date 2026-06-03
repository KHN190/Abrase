use crate::ast;
use crate::ast::{Span, Spanned};
use crate::ty::Type;
use super::*;

impl Checker {

    // Type Environment Management

    pub fn register_type(&mut self, name: String, body: ast::TypeBody) {
        if let ast::TypeBody::Variant(cases) = &body {
            let case_names: Vec<String> = cases.iter().map(|c| match c {
                ast::VariantCase::Unit(n) => n.clone(),
                ast::VariantCase::Tuple(n, _) => n.clone(),
                ast::VariantCase::Record(n, _) => n.clone(),
            }).collect();
            self.variant_registry.insert(name.clone(), case_names);
        }
        self.type_registry.insert(name, body);
    }

    pub fn get_type(&self, name: &str) -> Option<ast::TypeBody> {
        self.type_registry.get(name).cloned()
    }

    pub fn register_variant_cases(&mut self, type_name: String, cases: Vec<String>) {
        self.variant_registry.insert(type_name, cases);
    }

    pub fn get_variant_cases(&self, type_name: &str) -> Option<&Vec<String>> {
        self.variant_registry.get(type_name)
    }

    pub fn get_const(&self, name: &str) -> Option<Type> {
        self.const_registry.get(name).cloned()
    }

    // Generic Variance

    pub fn register_type_variance(&mut self, type_name: String, variances: Vec<crate::ty::Variance>) {
        self.variance_registry.insert(type_name, variances);
    }

    pub fn get_type_variance(&self, type_name: &str) -> Option<&Vec<crate::ty::Variance>> {
        self.variance_registry.get(type_name)
    }

    pub fn register_named_subtype(&mut self, sub: String, sup: String) {
        self.named_subtype_registry
            .entry(sub)
            .or_insert_with(Vec::new)
            .push(sup);
    }

    pub fn is_subtype(&self, sub: &Type, sup: &Type) -> bool {
        match (sub, sup) {
            (Type::Unknown, _) | (_, Type::Unknown) => true,
            (a, b) if a == b => true,
            (Type::Generic { name: s1, args: a1 }, Type::Generic { name: s2, args: a2 }) => {
                s1 == s2 && a1.len() == a2.len() &&
                a1.iter().zip(a2.iter()).all(|(sa, pa)| self.is_subtype(sa, pa))
            }
            (Type::Named(s1), Type::Named(s2)) => {
                self.is_generic_subtype(s1, s2) || self.is_named_subtype(s1, s2)
            }
            (Type::Function { params: p1, ret: r1, .. },
             Type::Function { params: p2, ret: r2, .. }) => {
                p1.len() == p2.len()
                    && p1.iter().zip(p2.iter()).all(|(sp, pp)| self.is_subtype(pp, sp))
                    && self.is_subtype(r1, r2)
            }
            (Type::Tuple(e1), Type::Tuple(e2)) => {
                e1.len() == e2.len()
                    && e1.iter().zip(e2.iter()).all(|(s, p)| self.is_subtype(s, p))
            }
            (Type::Shared { inner: i1, region: r1 },
             Type::Shared { inner: i2, region: r2 }) => {
                r1 == r2 && self.is_subtype(i1, i2)
            }
            _ => false,
        }
    }

    fn is_named_subtype(&self, sub: &str, sup: &str) -> bool {
        if sub == sup {
            return true;
        }
        if let Some(supertypes) = self.named_subtype_registry.get(sub) {
            for supertype in supertypes {
                if supertype == sup {
                    return true;
                }
                if self.is_named_subtype_with_depth(sub, sup, 10) {
                    return true;
                }
            }
        }
        false
    }

    fn is_named_subtype_with_depth(&self, sub: &str, sup: &str, depth: usize) -> bool {
        if depth == 0 || sub == sup {
            return sub == sup;
        }
        if let Some(supertypes) = self.named_subtype_registry.get(sub) {
            for supertype in supertypes {
                if supertype == sup || self.is_named_subtype_with_depth(supertype, sup, depth - 1) {
                    return true;
                }
            }
        }
        false
    }

    fn is_generic_subtype(&self, sub_str: &str, sup_str: &str) -> bool {
        if sub_str == sup_str {
            return true;
        }
        match (Self::parse_generic_named(sub_str), Self::parse_generic_named(sup_str)) {
            (Some((sub_name, sub_args)), Some((sup_name, sup_args)))
                if sub_name == sup_name && sub_args.len() == sup_args.len() =>
            {
                let variances = match self.variance_registry.get(sub_name) {
                    Some(v) => v.clone(),
                    None => return sub_str == sup_str,
                };
                sub_args
                    .iter()
                    .zip(sup_args.iter())
                    .enumerate()
                    .all(|(i, (sa, pa))| {
                        let variance = variances
                            .get(i)
                            .copied()
                            .unwrap_or(crate::ty::Variance::Invariant);
                        match variance {
                            crate::ty::Variance::Covariant => {
                                self.is_generic_subtype(sa.trim(), pa.trim())
                                    || self.is_named_subtype(sa.trim(), pa.trim())
                            }
                            crate::ty::Variance::Contravariant => {
                                self.is_generic_subtype(pa.trim(), sa.trim())
                                    || self.is_named_subtype(pa.trim(), sa.trim())
                            }
                            crate::ty::Variance::Invariant => sa.trim() == pa.trim(),
                        }
                    })
            }
            _ => false,
        }
    }

    fn parse_generic_named(s: &str) -> Option<(&str, Vec<&str>)> {
        let lt = s.find('<')?;
        if !s.ends_with('>') {
            return None;
        }
        let name = &s[..lt];
        let args_str = &s[lt + 1..s.len() - 1];
        let args = Self::split_top_level(args_str, ',');
        Some((name, args))
    }

    fn split_top_level(s: &str, sep: char) -> Vec<&str> {
        let mut result = Vec::new();
        let mut current_start = 0;
        let mut depth = 0;
        for (i, c) in s.char_indices() {
            if c == '<' {
                depth += 1;
            } else if c == '>' {
                depth -= 1;
            } else if c == sep && depth == 0 {
                result.push(s[current_start..i].trim());
                current_start = i + 1;
            }
        }
        if current_start < s.len() {
            result.push(s[current_start..].trim());
        }
        result.into_iter().filter(|x| !x.is_empty()).collect()
    }

    // Type Compatibility & Unification

    pub fn types_compatible(&self, expected: &Type, actual: &Type) -> bool {
        match (expected, actual) {
            (a, b) if a == b => true,
            (Type::Unknown, _) | (_, Type::Unknown) => true,
            (_, Type::Never) => true,
            (Type::Generic { name: e_name, args: e_args }, Type::Generic { name: a_name, args: a_args }) => {
                e_name == a_name && e_args.len() == a_args.len() &&
                e_args.iter().zip(a_args.iter())
                    .all(|(e, a)| self.types_compatible(e, a))
            },
            (Type::Named(exp_name), Type::Named(act_name)) => {
                exp_name == act_name
                    || self.are_types_equivalent(exp_name, act_name)
                    || self.is_generic_subtype(act_name, exp_name)
            },
            (Type::Tuple(exp_elems), Type::Tuple(act_elems)) => {
                exp_elems.len() == act_elems.len() &&
                exp_elems.iter().zip(act_elems.iter())
                    .all(|(e, a)| self.types_compatible(e, a))
            },
            (Type::Reference { is_mut: e_mut, inner: e_inner },
             Type::Reference { is_mut: a_mut, inner: a_inner }) => {
                e_mut == a_mut && self.types_compatible(e_inner, a_inner)
            },
            (Type::Shared { inner: e_inner, region: e_r },
             Type::Shared { inner: a_inner, region: a_r }) => {
                e_r == a_r && self.types_compatible(e_inner, a_inner)
            },
            (Type::Function { params: e_params, ret: e_ret, .. },
             Type::Function { params: a_params, ret: a_ret, .. }) => {
                e_params.len() == a_params.len() &&
                e_params.iter().zip(a_params.iter())
                    .all(|(e, a)| self.types_compatible(e, a)) &&
                self.types_compatible(e_ret, a_ret)
            },
            _ => false,
        }
    }

    pub fn are_types_equivalent(&self, ty1: &str, ty2: &str) -> bool {
        if ty1 == ty2 {
            return true;
        }

        match (self.type_registry.get(ty1), self.type_registry.get(ty2)) {
            (Some(def1), Some(def2)) => {

                match (def1, def2) {
                    (ast::TypeBody::Record(fields1), ast::TypeBody::Record(fields2)) => {
                        if fields1.len() != fields2.len() {
                            return false;
                        }
                        fields1.iter().zip(fields2.iter()).all(|(f1, f2)| {
                            f1.name == f2.name && self.convert_type(&f1.ty) == self.convert_type(&f2.ty)
                        })
                    }
                    (ast::TypeBody::Variant(vars1), ast::TypeBody::Variant(vars2)) => {
                        if vars1.len() != vars2.len() {
                            return false;
                        }
                        vars1.iter().zip(vars2.iter()).all(|(v1, v2)| {
                            match (v1, v2) {
                                (ast::VariantCase::Unit(n1), ast::VariantCase::Unit(n2)) => n1 == n2,
                                (ast::VariantCase::Tuple(n1, ts1), ast::VariantCase::Tuple(n2, ts2)) => {
                                    n1 == n2 && ts1.len() == ts2.len() &&
                                    ts1.iter().zip(ts2.iter()).all(|(t1, t2)| self.convert_type(t1) == self.convert_type(t2))
                                }
                                (ast::VariantCase::Record(n1, f1), ast::VariantCase::Record(n2, f2)) => {
                                    n1 == n2 && f1.len() == f2.len() &&
                                    f1.iter().zip(f2.iter()).all(|(rf1, rf2)| {
                                        rf1.name == rf2.name && self.convert_type(&rf1.ty) == self.convert_type(&rf2.ty)
                                    })
                                }
                                _ => false,
                            }
                        })
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    pub fn unify_types(&self, ty1: &Type, ty2: &Type) -> Option<Type> {
        match (ty1, ty2) {
            (a, b) if a == b => Some(a.clone()),
            (Type::Unknown, b) => Some(b.clone()),
            (a, Type::Unknown) => Some(a.clone()),
            // Unify generic type variable with concrete type
            (Type::Generic { .. }, _) => Some(ty2.clone()),
            (_, Type::Generic { .. }) => Some(ty1.clone()),
            (Type::Tuple(elems1), Type::Tuple(elems2)) if elems1.len() == elems2.len() => {
                let unified: Option<Vec<_>> = elems1.iter().zip(elems2.iter())
                    .map(|(e1, e2)| self.unify_types(e1, e2))
                    .collect();
                unified.map(Type::Tuple)
            },
            (Type::Reference { is_mut: m1, inner: i1 },
             Type::Reference { is_mut: m2, inner: i2 }) if m1 == m2 => {
                self.unify_types(i1, i2).map(|inner| Type::Reference {
                    is_mut: *m1,
                    inner: Box::new(inner),
                })
            },
            (Type::Function { params: p1, ret: r1, .. },
             Type::Function { params: p2, ret: r2, .. }) if p1.len() == p2.len() => {
                let unified_params: Option<Vec<_>> = p1.iter().zip(p2.iter())
                    .map(|(pp1, pp2)| self.unify_types(pp1, pp2))
                    .collect();
                let unified_ret = self.unify_types(r1, r2);
                match (unified_params, unified_ret) {
                    (Some(params), Some(ret)) => Some(Type::Function {
                        params,
                        effects: vec![],
                        ret: Box::new(ret),
                    }),
                    _ => None,
                }
            },
            _ => None,
        }
    }

    pub fn is_assignable(&self, expected: &Type, actual: &Type) -> bool {
        self.types_compatible(expected, actual) || self.is_subtype(actual, expected)
    }

    pub fn build_substitution_map(
        &self,
        param_types: &[Type],
        arg_types: &[Type],
    ) -> std::collections::HashMap<String, Type> {
        let mut subst = std::collections::HashMap::new();
        for (param, arg) in param_types.iter().zip(arg_types.iter()) {
            if let Some(unified) = self.unify_types(param, arg) {
                self.extract_substitutions(param, &unified, &mut subst);
            }
        }
        subst
    }

    fn extract_substitutions(
        &self,
        param: &Type,
        unified: &Type,
        subst: &mut std::collections::HashMap<String, Type>,
    ) {
        match (param, unified) {
            (Type::Generic { name, .. }, _) => {
                subst.insert(name.clone(), unified.clone());
            }
            (Type::Tuple(p_elems), Type::Tuple(u_elems)) => {
                for (p, u) in p_elems.iter().zip(u_elems.iter()) {
                    self.extract_substitutions(p, u, subst);
                }
            }
            (Type::Reference { inner: p_inner, .. }, Type::Reference { inner: u_inner, .. }) => {
                self.extract_substitutions(p_inner, u_inner, subst);
            }
            (Type::Function { params: p_params, ret: p_ret, .. },
             Type::Function { params: u_params, ret: u_ret, .. }) => {
                for (p, u) in p_params.iter().zip(u_params.iter()) {
                    self.extract_substitutions(p, u, subst);
                }
                self.extract_substitutions(p_ret, u_ret, subst);
            }
            _ => {}
        }
    }

    pub fn apply_substitution(&self, ty: &Type, subst: &std::collections::HashMap<String, Type>) -> Type {
        match ty {
            Type::Generic { name, args } => {
                if let Some(substituted) = subst.get(name) {
                    return substituted.clone();
                }
                Type::Generic {
                    name: name.clone(),
                    args: args.iter().map(|arg| self.apply_substitution(arg, subst)).collect(),
                }
            }
            Type::Tuple(elems) => {
                Type::Tuple(elems.iter().map(|e| self.apply_substitution(e, subst)).collect())
            }
            Type::Reference { is_mut, inner } => {
                Type::Reference {
                    is_mut: *is_mut,
                    inner: Box::new(self.apply_substitution(inner, subst)),
                }
            }
            Type::Function { params, effects, ret } => {
                Type::Function {
                    params: params.iter().map(|p| self.apply_substitution(p, subst)).collect(),
                    effects: effects.clone(),
                    ret: Box::new(self.apply_substitution(ret, subst)),
                }
            }
            _ => ty.clone(),
        }
    }

    pub fn check_stmt(&mut self, stmt: &Spanned<ast::Stmt>) {
        match &stmt.node {
            ast::Stmt::Let { pattern, is_mut, ty, value } => {
                let name = match &pattern.node {
                    ast::Pattern::Bind(n) => n.clone(),
                    _ => "<pattern>".into(),
                };
                self.context_stack.push(format!("In let binding for '{}'", name));
                let mut val_ty = self.infer_expr(value);

                if let Some(expected_ast_ty) = ty {
                    let expected_ty = self.convert_type(expected_ast_ty);
                    if !shared_region_relaxed_eq(&expected_ty, &val_ty) && val_ty != Type::Unknown {
                        self.report_error(
                            format!("Type mismatch: expected {:?}, found {:?}", expected_ty, val_ty),
                            value.span
                        );
                    }
                    val_ty = expected_ty;
                }
                // Register binding directly to preserve let-stmt's is_mut flag in VarMeta.
                if let ast::Pattern::Bind(n) = &pattern.node {
                    self.insert_var(n.clone(), val_ty.clone(), *is_mut, pattern.span);
                    if let ast::Expr::Unary {
                        op: ast::UnaryOp::Ref | ast::UnaryOp::RefMut, right,
                    } = &value.node {
                        if let ast::Expr::Identifier(base) = &right.node {
                            self.set_var_borrows(n, base.clone());
                        }
                    }
                } else {
                    self.check_pattern_mut(pattern, &val_ty, *is_mut);
                }
                self.context_stack.pop();
            }
            ast::Stmt::Expr(expr) => {
                // Statement-position block: value is discarded, so it is an
                // implicit region.
                let is_block = matches!(&expr.node, ast::Expr::Block(_));
                if is_block {
                    self.push_region(format!("stmt_block_{}", self.region_stack.len()));
                }
                self.infer_expr(expr);
                if is_block { self.pop_region(); }
            }
            ast::Stmt::Empty => {}
        }
    }

    pub fn check_recursive_type(&mut self, type_name: &str, body: &ast::TypeBody, span: Span) -> bool {
        self.check_type_recursion(type_name, body, &mut std::collections::HashSet::new(), span)
    }

    pub fn check_type_recursion(
        &mut self,
        type_name: &str,
        body: &ast::TypeBody,
        visited: &mut std::collections::HashSet<String>,
        span: Span,
    ) -> bool {
        if visited.contains(type_name) {
            self.report_error(
                format!("Recursive type cycle detected in '{}'", type_name),
                span,
            );
            return false;
        }
        visited.insert(type_name.to_string());

        let valid = match body {
            ast::TypeBody::Record(fields) => {
                let mut all_valid = true;
                for field in fields {
                    if !self.is_type_valid(&field.ty, type_name, false, visited, span) {
                        all_valid = false;
                    }
                }
                all_valid
            }
            ast::TypeBody::Variant(cases) => {
                let mut all_valid = true;
                for case in cases {
                    let payload_tys: Vec<&ast::Type> = match case {
                        ast::VariantCase::Unit(_) => Vec::new(),
                        ast::VariantCase::Tuple(_, tys) => tys.iter().collect(),
                        ast::VariantCase::Record(_, fields) => {
                            fields.iter().map(|f| &f.ty).collect()
                        }
                    };
                    for ty in payload_tys {
                        if !self.is_type_valid(ty, type_name, true, visited, span) {
                            all_valid = false;
                        }
                    }
                }
                if all_valid {
                    let has_base = cases.iter().any(|c| !case_references_type(c, type_name));
                    if !has_base {
                        self.report_error(
                            format!(
                                "Type '{}' has no base case: every variant case carries a \
                                 '{}'-typed payload, so a value can never be constructed",
                                type_name, type_name
                            ),
                            span,
                        );
                        all_valid = false;
                    }
                }
                all_valid
            }
        };

        visited.remove(type_name);
        valid
    }

    fn is_type_valid(
        &mut self,
        ty: &ast::Type,
        self_type: &str,
        allow_self_ref: bool,
        visited: &mut std::collections::HashSet<String>,
        span: Span,
    ) -> bool {
        match ty {
            ast::Type::Named(name) => {
                if name == self_type {
                    if allow_self_ref {
                        // OK: the parent variant tag heap-allocates this value,
                        // so the reference is pointer-sized.
                        return true;
                    }
                    self.report_error(
                        format!("Type '{}' has recursive cycle: direct self-reference detected", self_type),
                        span,
                    );
                    return false;
                }
                // Mutual recursion through ADTs is safe: every variant/record
                // payload is a heap handle (pointer-sized), and per-type base
                // case checks run independently. If we've already started
                // checking this name on the current path, just skip.
                if visited.contains(name) {
                    return true;
                }
                if let Some(body) = self.get_type(name) {
                    if !self.check_type_recursion(name, &body, visited, span) {
                        return false;
                    }
                }
                true
            }
            ast::Type::Reference { .. } => {
                // References have fixed size, so self-reference through reference is allowed
                // The reference breaks the cycle, so we don't check the inner type
                true
            }
            ast::Type::Array { elem, .. } => {
                // Arrays have fixed size, so recursion is ok
                self.is_type_valid(elem, self_type, allow_self_ref, visited, span)
            }
            _ => true, // Primitive types and other constructs are valid
        }
    }

    pub fn detect_type_cycles(&mut self) -> bool {
        let type_names: Vec<String> = self.type_registry.keys().cloned().collect();
        let mut has_cycles = false;
        // ERROR: the size for values of type `str` cannot be known at compilation time
        for type_name in type_names {
            let body = self.type_registry.get(&type_name).cloned();
            if let Some(body) = body {
                let mut visited = std::collections::HashSet::new();
                if !self.check_type_recursion(&type_name, &body, &mut visited, ast::Span { line: 0, col: 0 }) {
                    has_cycles = true;
                }
            }
        }
        !has_cycles
    }
}

fn case_references_type(case: &ast::VariantCase, self_type: &str) -> bool {
    match case {
        ast::VariantCase::Unit(_) => false,
        ast::VariantCase::Tuple(_, types) => {
            types.iter().any(|t| type_mentions(t, self_type))
        }
        ast::VariantCase::Record(_, fields) => {
            fields.iter().any(|f| type_mentions(&f.ty, self_type))
        }
    }
}

fn type_mentions(ty: &ast::Type, self_type: &str) -> bool {
    match ty {
        ast::Type::Named(n) => n == self_type,
        ast::Type::Qualified(path) => path.last().map(|s| s.as_str()) == Some(self_type),
        ast::Type::Generic { name, args } => {
            name == self_type || args.iter().any(|a| type_mentions(a, self_type))
        }
        ast::Type::Tuple(elems) => elems.iter().any(|e| type_mentions(e, self_type)),
        ast::Type::Array { elem, .. } => type_mentions(elem, self_type),
        ast::Type::Reference { inner, .. } => type_mentions(inner, self_type),
        ast::Type::Function { params, ret, .. } => {
            params.iter().any(|p| type_mentions(p, self_type)) || type_mentions(ret, self_type)
        }
    }
}

fn shared_region_relaxed_eq(expected: &Type, actual: &Type) -> bool {
    if expected == actual { return true; }
    match (expected, actual) {
        (Type::Shared { inner: ei, region: None }, Type::Shared { inner: ai, region: _ }) => {
            shared_region_relaxed_eq(ei, ai)
        }
        _ => false,
    }
}
