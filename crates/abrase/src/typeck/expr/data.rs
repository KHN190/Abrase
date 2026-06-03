use crate::ast;
use crate::ast::Spanned;
use crate::ty::Type;
use super::types_assignable;
use super::super::*;

impl Checker {
    pub(super) fn infer_tuple(&mut self, elems: &[Spanned<ast::Expr>]) -> Type {
        self.context_stack.push("In tuple construction".into());
        let elem_types: Vec<_> = elems.iter().map(|e| self.infer_expr(e)).collect();
        self.context_stack.pop();
        Type::Tuple(elem_types)
    }

    pub(super) fn infer_array(&mut self, elems: &[Spanned<ast::Expr>]) -> Type {
        self.context_stack.push("In array construction".into());
        let result = if elems.is_empty() {
            Type::Generic { name: "Array".into(), args: vec![Type::Unknown] }
        } else {
            let first_ty = self.infer_expr(&elems[0]);
            for elem in &elems[1..] {
                let elem_ty = self.infer_expr(elem);
                if elem_ty != first_ty && elem_ty != Type::Unknown && first_ty != Type::Unknown {
                    self.report_error("Array elements must have same type".into(), elem.span);
                }
            }
            Type::Generic { name: "Array".into(), args: vec![first_ty] }
        };
        self.context_stack.pop();
        result
    }

    pub(super) fn infer_array_repeat(&mut self, elem: &Spanned<ast::Expr>, count: &Spanned<ast::Expr>) -> Type {
        self.context_stack.push("In array repeat".into());
        let elem_ty = self.infer_expr(elem);
        let count_ty = self.infer_expr(count);
        if count_ty != Type::Int && count_ty != Type::Unknown {
            self.report_error("Array repeat count must be Int".into(), count.span);
        }
        self.context_stack.pop();
        Type::Generic { name: "Array".into(), args: vec![elem_ty] }
    }

    pub(super) fn infer_index(&mut self, base: &Spanned<ast::Expr>, index: &Spanned<ast::Expr>) -> Type {
        self.context_stack.push("In array indexing".into());
        let base_ty = if let ast::Expr::Identifier(name) = &base.node {
            self.get_var(name, true, base.span)
        } else {
            self.infer_expr(base)
        };
        let index_ty = self.infer_expr(index);
        if index_ty != Type::Int && index_ty != Type::Unknown {
            self.report_error("Index must be Int".into(), index.span);
        }
        let result = match base_ty {
            Type::Generic { ref name, ref args } if name == "Array" => args.get(0).cloned().unwrap_or(Type::Unknown),
            Type::Tuple(ref elems) => if elems.is_empty() { Type::Unknown } else { elems[0].clone() },
            Type::Unknown => Type::Unknown,
            _ => self.report_error("Can only index arrays or tuples".into(), base.span),
        };
        self.context_stack.pop();
        result
    }

    pub(super) fn infer_field_access(&mut self, base: &Spanned<ast::Expr>, field: &str) -> Type {
        if let ast::Expr::Identifier(base_name) = &base.node {
            if let Some(cases) = self.variant_registry.get(base_name) {
                if cases.iter().any(|c| c == field) {
                    return Type::Named(base_name.clone());
                }
            }
        }
        self.context_stack.push(format!("In field access '{}'", field));
        let base_ty = if let ast::Expr::Identifier(name) = &base.node {
            self.get_var(name, true, base.span)
        } else {
            self.infer_expr(base)
        };
        let field_type = self.resolve_field_access(&base_ty, field, base.span);
        self.context_stack.pop();
        field_type
    }

    pub(super) fn infer_record(&mut self, ty: &[String], fields: &[ast::FieldInit], span: ast::Span) -> Type {
        let type_name = ty.join(".");
        self.context_stack.push(format!("In record construction of '{}'", type_name));
        let declared_opt = self.type_registry.get(&type_name).cloned();
        let mut declared_tys: std::collections::HashMap<String, Type> = std::collections::HashMap::new();
        if let Some(ast::TypeBody::Record(declared)) = &declared_opt {
            for f in declared { declared_tys.insert(f.name.clone(), self.convert_type(&f.ty)); }
        }
        for field in fields {
            if let Some(value) = &field.value {
                let v_ty = self.infer_expr(value);
                if let Some(expected) = declared_tys.get(&field.name) {
                    if !types_assignable(expected, &v_ty) {
                        self.report_error(
                            format!("Record '{}' field '{}': expected {:?}, got {:?}", type_name, field.name, expected, v_ty),
                            value.span,
                        );
                    }
                }
            }
        }
        if let Some(ast::TypeBody::Record(declared)) = &declared_opt {
            let known: Vec<String> = declared.iter().map(|f| f.name.clone()).collect();
            for field in fields {
                if !known.iter().any(|n| n == &field.name) {
                    self.report_error(format!("Record '{}' has no field '{}'; known fields: {:?}", type_name, field.name, known), span);
                }
            }
            for declared_name in &known {
                if !fields.iter().any(|f| &f.name == declared_name) {
                    self.report_error(format!("Record '{}' missing required field '{}'", type_name, declared_name), span);
                }
            }
        }
        self.context_stack.pop();
        Type::Named(type_name)
    }

    pub(super) fn infer_variant(&mut self, ty: &[String], args: &[Spanned<ast::Expr>]) -> Type {
        let case_name = ty.last().cloned().unwrap_or_default();
        self.context_stack.push(format!("In variant construction of '{}'", ty.join(".")));
        let payload_tys: Vec<Type> = match self.lookup_variant_constructor(&case_name) {
            Some(Type::Function { params, .. }) => params,
            _ => Vec::new(),
        };
        for (i, arg) in args.iter().enumerate() {
            let arg_ty = self.infer_expr(arg);
            if let Some(expected) = payload_tys.get(i) {
                if !types_assignable(expected, &arg_ty) {
                    self.report_error(format!("Variant '{}' payload {}: expected {:?}, got {:?}", case_name, i, expected, arg_ty), arg.span);
                }
            }
        }
        self.context_stack.pop();
        Type::Named(ty.join("."))
    }

    pub(super) fn infer_range(&mut self, start: &Option<Box<Spanned<ast::Expr>>>, end: &Option<Box<Spanned<ast::Expr>>>) -> Type {
        if let Some(s) = start {
            let s_ty = self.infer_expr(s);
            if s_ty != Type::Int && s_ty != Type::Unknown {
                self.report_error("Range start must be Int".into(), s.span);
            }
        }
        if let Some(e) = end {
            let e_ty = self.infer_expr(e);
            if e_ty != Type::Int && e_ty != Type::Unknown {
                self.report_error("Range end must be Int".into(), e.span);
            }
        }
        Type::Named("Range<Int>".into())
    }

    pub(super) fn infer_question(&mut self, inner: &Spanned<ast::Expr>) -> Type {
        self.exn_prop = true;
        let inner_ty = self.infer_expr(inner);
        let in_exn_fn = self.fn_declared_effects.iter().any(|e| matches!(e, crate::ty::Effect::Exn(_)));
        match &inner_ty {
            Type::Generic { name, args } if name == "Result" => {
                let ok_ty = args.first().cloned().unwrap_or(Type::Unknown);
                let err_ty = args.get(1).cloned().unwrap_or(Type::Unknown);
                self.add_required_effect(crate::ty::Effect::Exn(Box::new(err_ty)));
                ok_ty
            }
            Type::Generic { name, args } if name == "Option" => {
                let inner_t = args.first().cloned().unwrap_or(Type::Unknown);
                self.add_required_effect(crate::ty::Effect::Exn(Box::new(Type::Named("NoneError".into()))));
                inner_t
            }
            Type::Unknown => Type::Unknown,
            _ if in_exn_fn => inner_ty.clone(),
            _ => {
                self.report_error(format!("'?' operator requires Result<T,E> or Option<T>, got {:?}", inner_ty), inner.span);
                Type::Unknown
            }
        }
    }
}
