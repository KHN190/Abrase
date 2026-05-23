use crate::ast;
use crate::ast::{Span, Spanned};
use crate::bytecode::{OpCode, Register};
use crate::compiler::Compiler;
use crate::compiler::codegen::scaffold::{to_u8, to_u16};
use crate::bytecode::Value;

impl Compiler {
    pub(in crate::compiler) fn emit_builtin_call(
        &mut self,
        fn_id: usize,
        arg_srcs: &[Register],
    ) -> Result<Register, String> {
        let fn_id_u16 = to_u16(fn_id, "Function id")?;
        for (i, src) in arg_srcs.iter().enumerate() {
            let slot = to_u8(i, "Argument index")?;
            let pos = self.code.len();
            self.emit(OpCode::Copy(Register(0), *src));
            self.pending_arg_patches.push((pos, slot));
        }
        let dest = self.alloc_register()?;
        self.emit(OpCode::Call(dest, fn_id_u16));
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
            ast::Literal::Int(n)    => {
                self.check_int32_literal(*n)?;
                self.add_constant(Value::from_int(*n))?
            }
            ast::Literal::Float(f)  => {
                self.check_float32_literal(*f)?;
                let v = if self.int32_mode { Value::from_float_f32(*f) } else { Value::from_float(*f) };
                self.add_constant(v)?
            }
            ast::Literal::Bool(b)   => self.add_constant(Value::from_bool(*b))?,
            ast::Literal::Char(c)   => self.add_constant(Value::from_char(*c))?,
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
                let inferred = self.infer_expr_type(&expr);
                let val_reg = self.compile_expr(&expr)?;
                let mangled = match &inferred {
                    Some(ast::Type::Named(n)) => match n.as_str() {
                        "Int"    => "__int_to_s",
                        "Float"  => "__float_to_s",
                        "Bool"   => "__bool_to_s",
                        "Char"   => "__char_to_s",
                        "String" => "__string_to_s",
                        _ => "__to_str",
                    },
                    Some(ast::Type::Tuple(items)) if items.is_empty() => "__unit_to_s",
                    _ => "__to_str",
                };
                let to_str_id = *self.func_map.get(mangled).unwrap_or(
                    &self.to_str_fn_id
                        .ok_or_else(|| "internal: __to_str builtin not registered".to_string())?
                );
                self.emit_builtin_call(to_str_id, &[val_reg])
            }
        }
    }

    pub(in crate::compiler) fn compile_identifier(
        &mut self,
        name: &str,
    ) -> Result<Register, String> {
        if let Some(reg) = self.var_to_reg.get(name).copied() {
            if self.cell_bindings.contains(name) {
                let dest = self.alloc_register()?;
                self.emit(OpCode::Ld(dest, reg, 0));
                return Ok(dest);
            }
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
        let field_count = to_u16(field_order.fields.len(), &format!("Record '{}' field count", type_name))?;
        self.emit(OpCode::Alloc(dest, field_count));
        for (i, fname) in field_order.fields.iter().enumerate() {
            let offset = to_u16(i, "Record field offset")?;
            let init = fields.iter().find(|f| &f.name == fname)
                .ok_or_else(|| format!("Missing field '{}' in {}", fname, type_name))?;
            let (src, want_move) = if let Some(v) = &init.value {
                let m = self.arg_should_move(v);
                let s = self.compile_expr(v)?;
                (s, m)
            } else {
                let name = init.name.clone();
                let m = self.id_should_move(&name);
                let s = self.var_to_reg.get(&name).copied()
                    .ok_or_else(|| format!("Undefined variable: {}", name))?;
                (s, m)
            };
            self.emit_store_field(src, want_move, dest, offset)?;
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
        let alloc_size = to_u16(payload + 1, &format!("Variant '{}' payload size", vname))?;
        self.emit(OpCode::Alloc(dest, alloc_size));
        let tag_reg = self.alloc_register()?;
        let ti = self.add_constant(Value::from_int(info.tag as i64))?;
        self.emit(OpCode::PushConst(tag_reg, ti));
        self.emit(OpCode::St(tag_reg, dest, 0));
        for (i, arg) in args.iter().enumerate() {
            let offset = to_u16(i + 1, "Variant payload offset")?;
            let v = self.compile_expr(arg)?;
            self.emit(OpCode::St(v, dest, offset));
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
        let count = to_u16(items.len(), "Array literal length")?;
        self.emit(OpCode::Alloc(dest, count));
        for (i, item) in items.iter().enumerate() {
            let offset = to_u16(i, "Array element offset")?;
            let v = self.compile_expr(item)?;
            self.emit(OpCode::St(v, dest, offset));
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

    pub(in crate::compiler) fn compile_tuple(
        &mut self,
        items: &[ast::Spanned<ast::Expr>],
    ) -> Result<Register, String> {
        let dest = self.alloc_register()?;
        let count = to_u16(items.len(), "Tuple length")?;
        self.emit(OpCode::Alloc(dest, count));
        for (i, item) in items.iter().enumerate() {
            let offset = to_u16(i, "Tuple element offset")?;
            let v = self.compile_expr(item)?;
            self.emit(OpCode::St(v, dest, offset));
        }
        Ok(dest)
    }

    pub(in crate::compiler) fn compile_array_repeat(
        &mut self,
        elem: &ast::Spanned<ast::Expr>,
        count: &ast::Spanned<ast::Expr>,
    ) -> Result<Register, String> {
        let n = match &count.node {
            ast::Expr::Literal(ast::Literal::Int(n)) if *n >= 0 => *n as usize,
            _ => return Err("array-repeat count must be a non-negative integer literal".into()),
        };
        let dest = self.alloc_register()?;
        let n_u16 = to_u16(n, "Array-repeat length")?;
        self.emit(OpCode::Alloc(dest, n_u16));
        let src = self.compile_expr(elem)?;
        for i in 0..n {
            let offset = to_u16(i, "Array-repeat offset")?;
            let copy = self.alloc_register()?;
            self.emit(OpCode::Copy(copy, src));
            self.emit(OpCode::St(copy, dest, offset));
        }
        Ok(dest)
    }

    pub(in crate::compiler) fn compile_range(
        &mut self,
        start: Option<&ast::Spanned<ast::Expr>>,
        end: Option<&ast::Spanned<ast::Expr>>,
        inclusive: bool,
    ) -> Result<Register, String> {
        let dest = self.alloc_register()?;
        self.emit(OpCode::Alloc(dest, 3));

        let s_reg = match start {
            Some(e) => self.compile_expr(e)?,
            None => {
                let r = self.alloc_register()?;
                let i = self.add_constant(Value::from_int(0))?;
                self.emit(OpCode::PushConst(r, i));
                r
            }
        };
        self.emit(OpCode::St(s_reg, dest, 0));

        let e_reg = match end {
            Some(e) => self.compile_expr(e)?,
            None => {
                let r = self.alloc_register()?;
                let i = self.add_constant(Value::from_int(i64::MAX >> 16))?;
                self.emit(OpCode::PushConst(r, i));
                r
            }
        };
        self.emit(OpCode::St(e_reg, dest, 1));

        let inc_reg = self.alloc_register()?;
        let inc_idx = self.add_constant(Value::from_int(if inclusive { 1 } else { 0 }))?;
        self.emit(OpCode::PushConst(inc_reg, inc_idx));
        self.emit(OpCode::St(inc_reg, dest, 2));

        Ok(dest)
    }
}
