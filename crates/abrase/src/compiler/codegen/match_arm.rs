use crate::ast;
use crate::bytecode::{OpCode, Register};
use crate::compiler::Compiler;
use crate::bytecode::Value;

impl Compiler {
    pub(in crate::compiler) fn compile_match(
        &mut self,
        scrutinee: &ast::Spanned<ast::Expr>,
        arms: &[ast::MatchArm],
    ) -> Result<Register, String> {
        if !arms.is_empty() {
            let last_arm = &arms[arms.len() - 1];
            match &last_arm.pattern.node {
                ast::Pattern::Wildcard | ast::Pattern::Bind(_) => {}
                _ => return Err("Non-exhaustive match pattern".to_string()),
            }
        } else {
            return Err("Empty match expression".to_string());
        }

        let scrutinee_reg = self.compile_expr(scrutinee)?;
        let result_reg = self.alloc_register()?;
        let mut exit_jumps = Vec::new();

        for arm in arms {
            match &arm.pattern.node {
                ast::Pattern::Wildcard => {
                    let body_reg = self.compile_expr(&arm.body)?;
                    self.emit(OpCode::Copy(result_reg, body_reg));
                    break;
                }
                ast::Pattern::Literal(lit) => {
                    self.compile_literal_arm(
                        lit, &arm.body, scrutinee_reg, result_reg, &mut exit_jumps)?;
                }
                ast::Pattern::Bind(name) => {
                    if self.layouts.variants.contains_key(name) {
                        let info = self.layouts.variants.get(name).cloned()
                            .ok_or_else(|| format!(
                                "internal: variant '{}' lost between contains_key and get",
                                name
                            ))?;
                        self.compile_variant_tag_arm(
                            info.tag, &arm.body, scrutinee_reg, result_reg, &mut exit_jumps)?;
                    } else {
                        // Plain bind: catch-all. Subsequent arms unreachable.
                        self.var_to_reg.insert(name.clone(), scrutinee_reg);
                        let body_reg = self.compile_expr(&arm.body)?;
                        self.emit(OpCode::Copy(result_reg, body_reg));
                        break;
                    }
                }
                ast::Pattern::Variant { ty: vty, args } => {
                    self.compile_variant_pattern_arm(
                        vty, args, &arm.body, scrutinee_reg, result_reg, &mut exit_jumps)?;
                }
                ast::Pattern::Record { ty: rty, fields, .. } => {
                    self.compile_record_pattern_arm(
                        rty, fields, &arm.body, scrutinee_reg, result_reg)?;
                    break;
                }
                _ => return Err("Unsupported pattern in match".to_string()),
            }
        }

        let end_addr = self.code.len();
        for &jmp_idx in &exit_jumps {
            self.patch_jmp_at(jmp_idx, end_addr)?;
        }

        Ok(result_reg)
    }

    fn compile_literal_arm(
        &mut self,
        lit: &ast::Literal,
        body: &ast::Spanned<ast::Expr>,
        scrutinee_reg: Register,
        result_reg: Register,
        exit_jumps: &mut Vec<usize>,
    ) -> Result<(), String> {
        let pat_idx = match lit {
            ast::Literal::Int(n)    => self.add_constant(Value::from_int(*n))?,
            ast::Literal::Float(f)  => self.add_constant(Value::from_float(*f))?,
            ast::Literal::Bool(b)   => self.add_constant(Value::from_bool(*b))?,
            ast::Literal::String(s) => self.add_string_constant(s)?,
            ast::Literal::Unit      => self.add_constant(Value::UNIT)?,
            _ => return Err("Unsupported literal in pattern".to_string()),
        };
        let pat_reg = self.alloc_register()?;
        self.emit(OpCode::PushConst(pat_reg, pat_idx));

        let eq_reg = self.alloc_register()?;
        self.emit(OpCode::Eq(eq_reg, scrutinee_reg, pat_reg));

        let jz_idx = self.code.len();
        self.emit(OpCode::Jz(eq_reg, 0));

        let body_reg = self.compile_expr(body)?;
        self.emit(OpCode::Copy(result_reg, body_reg));
        exit_jumps.push(self.code.len());
        self.emit(OpCode::Jmp(0));

        let next_addr = self.code.len();
        self.patch_jz_at(jz_idx, next_addr)?;
        Ok(())
    }

    fn compile_variant_tag_arm(
        &mut self,
        expected_tag: u32,
        body: &ast::Spanned<ast::Expr>,
        scrutinee_reg: Register,
        result_reg: Register,
        exit_jumps: &mut Vec<usize>,
    ) -> Result<(), String> {
        let tag_reg = self.alloc_register()?;
        self.emit(OpCode::Ld(tag_reg, scrutinee_reg, 0));
        let expected = self.alloc_register()?;
        let ti = self.add_constant(Value::from_int(expected_tag as i64))?;
        self.emit(OpCode::PushConst(expected, ti));
        let eq_reg = self.alloc_register()?;
        self.emit(OpCode::Eq(eq_reg, tag_reg, expected));
        let jz_idx = self.code.len();
        self.emit(OpCode::Jz(eq_reg, 0));
        let body_reg = self.compile_expr(body)?;
        self.emit(OpCode::Copy(result_reg, body_reg));
        exit_jumps.push(self.code.len());
        self.emit(OpCode::Jmp(0));
        let next_addr = self.code.len();
        self.patch_jz_at(jz_idx, next_addr)?;
        Ok(())
    }

    fn compile_variant_pattern_arm(
        &mut self,
        vty: &[String],
        args: &[ast::Spanned<ast::Pattern>],
        body: &ast::Spanned<ast::Expr>,
        scrutinee_reg: Register,
        result_reg: Register,
        exit_jumps: &mut Vec<usize>,
    ) -> Result<(), String> {
        let vname = vty.last().cloned()
            .ok_or_else(|| "Variant pattern missing name".to_string())?;
        let info = self.layouts.variants.get(&vname).cloned()
            .ok_or_else(|| format!("Unknown variant: {}", vname))?;

        let tag_reg = self.alloc_register()?;
        self.emit(OpCode::Ld(tag_reg, scrutinee_reg, 0));
        let expected_tag = self.alloc_register()?;
        let ti = self.add_constant(Value::from_int(info.tag as i64))?;
        self.emit(OpCode::PushConst(expected_tag, ti));
        let eq_reg = self.alloc_register()?;
        self.emit(OpCode::Eq(eq_reg, tag_reg, expected_tag));
        let jz_idx = self.code.len();
        self.emit(OpCode::Jz(eq_reg, 0));

        for (i, arg_pat) in args.iter().enumerate() {
            if let ast::Pattern::Bind(n) = &arg_pat.node {
                let r = self.alloc_register()?;
                let offset = super::scaffold::to_u16(i + 1, "Variant pattern arg offset")?;
                self.emit(OpCode::Ld(r, scrutinee_reg, offset));
                self.var_to_reg.insert(n.clone(), r);
                if let Some(ft) = info.field_types.get(i) {
                    self.var_types.insert(n.clone(), ft.clone());
                }
            }
        }

        let body_reg = self.compile_expr(body)?;
        self.emit(OpCode::Copy(result_reg, body_reg));
        exit_jumps.push(self.code.len());
        self.emit(OpCode::Jmp(0));

        let next_addr = self.code.len();
        self.patch_jz_at(jz_idx, next_addr)?;
        Ok(())
    }

    fn compile_record_pattern_arm(
        &mut self,
        rty: &[String],
        fields: &[ast::FieldPattern],
        body: &ast::Spanned<ast::Expr>,
        scrutinee_reg: Register,
        result_reg: Register,
    ) -> Result<(), String> {
        let type_name = rty.last().cloned()
            .ok_or_else(|| "Record pattern missing type name".to_string())?;
        let field_order = self.layouts.records.get(&type_name).cloned()
            .ok_or_else(|| format!("Unknown record type: {}", type_name))?;

        for fp in fields {
            if let Some(idx) = field_order.fields.iter().position(|n| n == &fp.name) {
                let bind_name = match &fp.pattern {
                    Some(p) => if let ast::Pattern::Bind(n) = &p.node {
                        Some(n.clone())
                    } else { None },
                    None => Some(fp.name.clone()),
                };
                if let Some(n) = bind_name {
                    let r = self.alloc_register()?;
                    let offset = super::scaffold::to_u16(idx, "Record pattern field offset")?;
                    self.emit(OpCode::Ld(r, scrutinee_reg, offset));
                    self.var_to_reg.insert(n, r);
                }
            }
        }
        let body_reg = self.compile_expr(body)?;
        self.emit(OpCode::Copy(result_reg, body_reg));
        Ok(())
    }
}
