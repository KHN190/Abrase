use crate::ast;
use crate::ast::{Span, Spanned};
use crate::bytecode::{OpCode, Register};
use crate::compiler::Compiler;
use crate::myriad::Value;

impl Compiler {
    fn emit_builtin_call(
        &mut self,
        fn_id: usize,
        arg_srcs: &[Register],
    ) -> Result<Register, String> {
        if fn_id > u16::MAX as usize {
            return Err(format!("Function id {} exceeds u16 range", fn_id));
        }
        for (i, src) in arg_srcs.iter().enumerate() {
            if i > u8::MAX as usize {
                return Err("Too many arguments (>255)".to_string());
            }
            let pos = self.code.len();
            self.emit(OpCode::Copy(Register(0), *src));
            self.pending_arg_patches.push((pos, i as u8));
        }
        let dest = self.alloc_register()?;
        self.emit(OpCode::Call(dest, fn_id as u16));
        Ok(dest)
    }
}

impl Compiler {
    pub(in crate::compiler) fn compile_literal(
        &mut self,
        lit: &ast::Literal,
    ) -> Result<Register, String> {
        let reg = self.alloc_register()?;
        let idx = match lit {
            ast::Literal::Int(n)    => self.add_constant(Value::from_int(*n))?,
            ast::Literal::Float(f)  => self.add_constant(Value::from_float(*f))?,
            ast::Literal::Bool(b)   => self.add_constant(Value::from_bool(*b))?,
            ast::Literal::String(s) => self.add_string_constant(s)?,
            ast::Literal::Unit      => self.add_constant(Value::UNIT)?,
            _ => return Err("Unsupported literal".to_string()),
        };
        self.emit(OpCode::PushConst(reg, idx));
        Ok(reg)
    }

    pub(in crate::compiler) fn compile_string_interp(
        &mut self,
        parts: &[ast::StringPart],
        span: Span,
    ) -> Result<Register, String> {
        if parts.is_empty() {
            let reg = self.alloc_register()?;
            let idx = self.add_string_constant("")?;
            self.emit(OpCode::PushConst(reg, idx));
            return Ok(reg);
        }

        let concat_id = self.concat_fn_id
            .ok_or_else(|| "internal: __concat builtin not registered".to_string())?;

        let mut acc: Option<Register> = None;
        for part in parts {
            let part_reg = self.compile_string_part(part, span)?;
            acc = Some(match acc {
                None => part_reg,
                Some(prev) => self.emit_builtin_call(concat_id, &[prev, part_reg])?,
            });
        }
        acc.ok_or_else(|| "internal: string interp produced no parts".to_string())
    }

    fn compile_string_part(
        &mut self,
        part: &ast::StringPart,
        span: Span,
    ) -> Result<Register, String> {
        match part {
            ast::StringPart::Literal(s) => {
                let reg = self.alloc_register()?;
                let idx = self.add_string_constant(s)?;
                self.emit(OpCode::PushConst(reg, idx));
                Ok(reg)
            }
            ast::StringPart::Interp(path) => {
                if path.is_empty() {
                    let reg = self.alloc_register()?;
                    let idx = self.add_string_constant("")?;
                    self.emit(OpCode::PushConst(reg, idx));
                    return Ok(reg);
                }
                let mut expr = Spanned {
                    node: ast::Expr::Identifier(path[0].clone()),
                    span,
                };
                for field in &path[1..] {
                    expr = Spanned {
                        node: ast::Expr::FieldAccess {
                            base: Box::new(expr),
                            field: field.clone(),
                        },
                        span,
                    };
                }
                let val_reg = self.compile_expr(&expr)?;
                let to_str_id = self.to_str_fn_id
                    .ok_or_else(|| "internal: __to_str builtin not registered".to_string())?;
                self.emit_builtin_call(to_str_id, &[val_reg])
            }
        }
    }

    pub(in crate::compiler) fn compile_identifier(
        &mut self,
        name: &str,
    ) -> Result<Register, String> {
        if let Some(reg) = self.var_to_reg.get(name).copied() {
            return Ok(reg);
        }
        if let Some(info) = self.layouts.variants.get(name).cloned() {
            let dest = self.alloc_register()?;
            self.emit(OpCode::Alloc(dest, 1));
            let tag_reg = self.alloc_register()?;
            let idx = self.add_constant(Value::from_int(info.tag as i64))?;
            self.emit(OpCode::PushConst(tag_reg, idx));
            self.emit(OpCode::St(tag_reg, dest, 0));
            return Ok(dest);
        }
        Err(format!("Undefined variable: {}", name))
    }

    pub(in crate::compiler) fn compile_record(
        &mut self,
        ty: &[String],
        fields: &[ast::FieldInit],
    ) -> Result<Register, String> {
        let type_name = ty.last().cloned()
            .ok_or_else(|| "Record literal missing type name".to_string())?;
        let field_order = self.layouts.records.get(&type_name).cloned()
            .ok_or_else(|| format!("Unknown record type: {}", type_name))?;
        let dest = self.alloc_register()?;
        self.emit(OpCode::Alloc(dest, field_order.fields.len() as u16));
        for (i, fname) in field_order.fields.iter().enumerate() {
            let init = fields.iter().find(|f| &f.name == fname)
                .ok_or_else(|| format!("Missing field '{}' in {}", fname, type_name))?;
            let src = if let Some(v) = &init.value {
                self.compile_expr(v)?
            } else {
                self.var_to_reg.get(&init.name).copied()
                    .ok_or_else(|| format!("Undefined variable: {}", init.name))?
            };
            self.emit(OpCode::St(src, dest, i as u16));
        }
        Ok(dest)
    }

    pub(in crate::compiler) fn compile_variant_expr(
        &mut self,
        ty: &[String],
        args: &[ast::Spanned<ast::Expr>],
    ) -> Result<Register, String> {
        let vname = ty.last().cloned()
            .ok_or_else(|| "Variant constructor missing name".to_string())?;
        let info = self.layouts.variants.get(&vname).cloned()
            .ok_or_else(|| format!("Unknown variant: {}", vname))?;
        let dest = self.alloc_register()?;
        let payload = args.len();
        self.emit(OpCode::Alloc(dest, (payload + 1) as u16));
        let tag_reg = self.alloc_register()?;
        let ti = self.add_constant(Value::from_int(info.tag as i64))?;
        self.emit(OpCode::PushConst(tag_reg, ti));
        self.emit(OpCode::St(tag_reg, dest, 0));
        for (i, arg) in args.iter().enumerate() {
            let v = self.compile_expr(arg)?;
            self.emit(OpCode::St(v, dest, (i + 1) as u16));
        }
        Ok(dest)
    }

    pub(in crate::compiler) fn compile_field_access(
        &mut self,
        base: &ast::Spanned<ast::Expr>,
        field: &str,
    ) -> Result<Register, String> {
        if let ast::Expr::Identifier(base_name) = &base.node {
            if let Some(info) = self.layouts.variants.get(field).cloned() {
                if info.type_name == *base_name {
                    let dest = self.alloc_register()?;
                    self.emit(OpCode::Alloc(dest, 1));
                    let tag_reg = self.alloc_register()?;
                    let idx = self.add_constant(Value::from_int(info.tag as i64))?;
                    self.emit(OpCode::PushConst(tag_reg, idx));
                    self.emit(OpCode::St(tag_reg, dest, 0));
                    return Ok(dest);
                }
            }
        }
        let base_reg = self.compile_expr(base)?;
        let type_name = self.infer_expr_type(base).and_then(|t| match t {
            ast::Type::Named(n) => Some(n),
            _ => None,
        }).ok_or_else(|| format!("Cannot determine record type for field access '.{}'", field))?;
        let layout = self.layouts.records.get(&type_name)
            .ok_or_else(|| format!("Unknown record type: {}", type_name))?;
        let idx = layout.offset_of(field)
            .ok_or_else(|| format!("No field '{}' in {}", field, type_name))?;
        let dest = self.alloc_register()?;
        self.emit(OpCode::Ld(dest, base_reg, idx));
        Ok(dest)
    }

    pub(in crate::compiler) fn compile_array(
        &mut self,
        items: &[ast::Spanned<ast::Expr>],
    ) -> Result<Register, String> {
        let dest = self.alloc_register()?;
        self.emit(OpCode::Alloc(dest, items.len() as u16));
        for (i, item) in items.iter().enumerate() {
            let v = self.compile_expr(item)?;
            self.emit(OpCode::St(v, dest, i as u16));
        }
        Ok(dest)
    }

    pub(in crate::compiler) fn compile_index(
        &mut self,
        base: &ast::Spanned<ast::Expr>,
        index: &ast::Spanned<ast::Expr>,
    ) -> Result<Register, String> {
        let base_reg = self.compile_expr(base)?;
        let idx_reg = self.compile_expr(index)?;
        let dest = self.alloc_register()?;
        self.emit(OpCode::LdIdx(dest, base_reg, idx_reg));
        Ok(dest)
    }
}
