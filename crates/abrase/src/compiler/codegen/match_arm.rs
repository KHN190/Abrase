use crate::ast;
use crate::bytecode::{OpCode, Register};
use crate::compiler::Compiler;
use crate::bytecode::Value;

impl Compiler {
    fn drop_arm_bindings(&mut self, regs: &[Register]) {
        for &r in regs {
            if self.reg_holds_handle.get(r.0 as usize).copied().unwrap_or(false) {
                self.emit(OpCode::Drop(r));
                self.set_reg_handle(r, false);
            }
        }
    }

    pub(in crate::compiler) fn compile_match(
        &mut self,
        scrutinee: &ast::Spanned<ast::Expr>,
        arms: &[ast::MatchArm],
    ) -> Result<Register, String> {
        if arms.is_empty() {
            return Err("Empty match expression".to_string());
        }
        if !self.match_covers_all_variants(arms) {
            let last_arm = &arms[arms.len() - 1];
            match &last_arm.pattern.node {
                ast::Pattern::Wildcard | ast::Pattern::Bind(_) => {}
                _ => return Err("Non-exhaustive match pattern".to_string()),
            }
        }

        let scrutinee_reg = self.compile_expr(scrutinee)?;
        let result_reg = self.alloc_register()?;
        let arm_mark = self.snapshot_register_high_water();
        let pre_match_table_reg = self.module_table_reg;
        let mut exit_jumps = Vec::new();

        for arm in arms {
            self.module_table_reg = pre_match_table_reg;
            let pre_vars: std::collections::HashSet<String> =
                self.var_to_reg.keys().cloned().collect();
            let has_guard = arm.guard.is_some();
            let terminal = match &arm.pattern.node {
                ast::Pattern::Wildcard => {
                    if let Some(guard) = &arm.guard {
                        // Guard on wildcard: evaluate guard; if false skip to next arm.
                        let g = self.compile_expr(guard)?;
                        let jz_idx = self.code.len();
                        self.emit(OpCode::Jz(g, 0));
                        let body_reg = self.compile_expr(&arm.body)?;
                        self.emit(OpCode::Copy(result_reg, body_reg));
                        exit_jumps.push(self.code.len());
                        self.emit(OpCode::Jmp(0));
                        let next_addr = self.code.len();
                        self.patch_jz_at(jz_idx, next_addr)?;
                        false
                    } else {
                        let body_reg = self.compile_expr(&arm.body)?;
                        self.emit(OpCode::Copy(result_reg, body_reg));
                        true
                    }
                }
                ast::Pattern::Literal(lit) => {
                    self.compile_literal_arm(
                        lit, &arm.body, scrutinee_reg, result_reg, &mut exit_jumps,
                        arm.guard.as_ref())?;
                    false
                }
                ast::Pattern::Bind(name) => {
                    if self.layouts.variants.contains_key(name) {
                        let info = self.layouts.variants.get(name).cloned()
                            .ok_or_else(|| format!(
                                "internal: variant '{}' lost between contains_key and get",
                                name
                            ))?;
                        self.compile_variant_tag_arm(
                            info.tag, &arm.body, scrutinee_reg, result_reg, &mut exit_jumps,
                            arm.guard.as_ref())?;
                        false
                    } else {
                        if let Some(guard) = &arm.guard {
                            self.var_to_reg.insert(name.clone(), scrutinee_reg);
                            let g = self.compile_expr(guard)?;
                            let jz_idx = self.code.len();
                            self.emit(OpCode::Jz(g, 0));
                            let body_reg = self.compile_expr(&arm.body)?;
                            self.emit(OpCode::Copy(result_reg, body_reg));
                            exit_jumps.push(self.code.len());
                            self.emit(OpCode::Jmp(0));
                            let next_addr = self.code.len();
                            self.patch_jz_at(jz_idx, next_addr)?;
                            false
                        } else {
                            self.var_to_reg.insert(name.clone(), scrutinee_reg);
                            let body_reg = self.compile_expr(&arm.body)?;
                            self.emit(OpCode::Copy(result_reg, body_reg));
                            true
                        }
                    }
                }
                ast::Pattern::Variant { ty: vty, args } => {
                    self.compile_variant_pattern_arm(
                        vty, args, &arm.body, scrutinee_reg, result_reg, &mut exit_jumps,
                        arm.guard.as_ref())?;
                    false
                }
                ast::Pattern::Record { ty: rty, fields, .. } => {
                    if has_guard {
                        self.compile_record_pattern_arm_guarded(
                            rty, fields, &arm.body, scrutinee_reg, result_reg,
                            arm.guard.as_ref(), &mut exit_jumps)?;
                        false
                    } else {
                        self.compile_record_pattern_arm(
                            rty, fields, &arm.body, scrutinee_reg, result_reg)?;
                        true
                    }
                }
                ast::Pattern::Range { start, end, inclusive } => {
                    self.compile_range_arm(
                        start.as_ref(), end.as_ref(), *inclusive,
                        &arm.body, scrutinee_reg, result_reg, &mut exit_jumps)?;
                    false
                }
                _ => return Err("Unsupported pattern in match".to_string()),
            };
            let new_vars: Vec<String> = self.var_to_reg.keys()
                .filter(|k| !pre_vars.contains(*k))
                .cloned()
                .collect();
            for n in new_vars {
                self.var_to_reg.remove(&n);
                self.var_types.remove(&n);
            }
            self.reclaim_temp_regs_above(arm_mark);
            if terminal { break; }
        }
        self.module_table_reg = pre_match_table_reg;

        let end_addr = self.code.len();
        for &jmp_idx in &exit_jumps {
            self.patch_jmp_at(jmp_idx, end_addr)?;
        }

        Ok(result_reg)
    }

    fn compile_range_arm(
        &mut self,
        start: Option<&ast::Literal>,
        end: Option<&ast::Literal>,
        inclusive: bool,
        body: &ast::Spanned<ast::Expr>,
        scrutinee_reg: Register,
        result_reg: Register,
        exit_jumps: &mut Vec<usize>,
    ) -> Result<(), String> {
        let mut cond_reg = None;

        if let Some(lo) = start {
            let lo_reg = self.compile_literal(lo)?;
            let ge = self.alloc_register()?;
            self.emit(OpCode::Gte(ge, scrutinee_reg, lo_reg));
            cond_reg = Some(ge);
        }

        if let Some(hi) = end {
            let hi_reg = self.compile_literal(hi)?;
            let cmp = self.alloc_register()?;
            if inclusive {
                self.emit(OpCode::Lte(cmp, scrutinee_reg, hi_reg));
            } else {
                self.emit(OpCode::Lt(cmp, scrutinee_reg, hi_reg));
            }
            cond_reg = Some(match cond_reg {
                None => cmp,
                Some(prev) => {
                    let combined = self.alloc_register()?;
                    self.emit(OpCode::And(combined, prev, cmp));
                    combined
                }
            });
        }

        let jz_idx = match cond_reg {
            Some(r) => {
                let idx = self.code.len();
                self.emit(OpCode::Jz(r, 0));
                Some(idx)
            }
            None => None,
        };

        let body_reg = self.compile_expr(body)?;
        self.emit(OpCode::Copy(result_reg, body_reg));
        exit_jumps.push(self.code.len());
        self.emit(OpCode::Jmp(0));

        let next_addr = self.code.len();
        if let Some(idx) = jz_idx {
            self.patch_jz_at(idx, next_addr)?;
        }
        Ok(())
    }

    fn compile_literal_arm(
        &mut self,
        lit: &ast::Literal,
        body: &ast::Spanned<ast::Expr>,
        scrutinee_reg: Register,
        result_reg: Register,
        exit_jumps: &mut Vec<usize>,
        guard: Option<&ast::Spanned<ast::Expr>>,
    ) -> Result<(), String> {
        let pat_idx = match lit {
            ast::Literal::Int(n)    => {
                self.add_constant(Value::from_int(*n))?
            }
            ast::Literal::Float(f)  => {
                let v = if self.int32_mode { Value::from_float_f32(*f) } else { Value::from_float(*f) };
                self.add_constant(v)?
            }
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

        let guard_jz = self.compile_guard_check(guard)?;
        let body_reg = self.compile_expr(body)?;
        self.emit(OpCode::Copy(result_reg, body_reg));
        exit_jumps.push(self.code.len());
        self.emit(OpCode::Jmp(0));

        let next_addr = self.code.len();
        self.patch_jz_at(jz_idx, next_addr)?;
        if let Some(idx) = guard_jz { self.patch_jz_at(idx, next_addr)?; }
        Ok(())
    }

    fn compile_variant_tag_arm(
        &mut self,
        expected_tag: u32,
        body: &ast::Spanned<ast::Expr>,
        scrutinee_reg: Register,
        result_reg: Register,
        exit_jumps: &mut Vec<usize>,
        guard: Option<&ast::Spanned<ast::Expr>>,
    ) -> Result<(), String> {
        let tag_reg = self.alloc_register()?;
        self.emit(OpCode::Ld(tag_reg, scrutinee_reg, 0));
        if self.typed_ld { self.set_reg_handle(tag_reg, false); }
        let expected = self.alloc_register()?;
        let ti = self.add_constant(Value::from_int(expected_tag as i64))?;
        self.emit(OpCode::PushConst(expected, ti));
        let eq_reg = self.alloc_register()?;
        self.emit(OpCode::Eq(eq_reg, tag_reg, expected));
        let jz_idx = self.code.len();
        self.emit(OpCode::Jz(eq_reg, 0));
        let guard_jz = self.compile_guard_check(guard)?;
        let body_reg = self.compile_expr(body)?;
        self.emit(OpCode::Copy(result_reg, body_reg));
        exit_jumps.push(self.code.len());
        self.emit(OpCode::Jmp(0));
        let next_addr = self.code.len();
        self.patch_jz_at(jz_idx, next_addr)?;
        if let Some(idx) = guard_jz { self.patch_jz_at(idx, next_addr)?; }
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
        guard: Option<&ast::Spanned<ast::Expr>>,
    ) -> Result<(), String> {
        let vname = vty.last().cloned()
            .ok_or_else(|| "Variant pattern missing name".to_string())?;
        let info = self.layouts.variants.get(&vname).cloned()
            .ok_or_else(|| format!("Unknown variant: {}", vname))?;

        let tag_reg = self.alloc_register()?;
        self.emit(OpCode::Ld(tag_reg, scrutinee_reg, 0));
        if self.typed_ld { self.set_reg_handle(tag_reg, false); }
        let expected_tag = self.alloc_register()?;
        let ti = self.add_constant(Value::from_int(info.tag as i64))?;
        self.emit(OpCode::PushConst(expected_tag, ti));
        let eq_reg = self.alloc_register()?;
        self.emit(OpCode::Eq(eq_reg, tag_reg, expected_tag));
        let jz_idx = self.code.len();
        self.emit(OpCode::Jz(eq_reg, 0));

        let mut bound_regs = Vec::new();
        for (i, arg_pat) in args.iter().enumerate() {
            if let ast::Pattern::Bind(n) = &arg_pat.node {
                let r = self.alloc_register()?;
                let offset = super::scaffold::to_u16(i + 1, "Variant pattern arg offset")?;
                self.emit(OpCode::Ld(r, scrutinee_reg, offset));
                if let Some(ft) = info.field_types.get(i) {
                    if self.typed_ld && super::data::type_is_unboxed(ft) { self.set_reg_handle(r, false); }
                    self.var_types.insert(n.clone(), ft.clone());
                }
                self.var_to_reg.insert(n.clone(), r);
                bound_regs.push(r);
            }
        }

        let guard_jz = self.compile_guard_check(guard)?;
        let body_reg = self.compile_expr(body)?;
        self.emit(OpCode::Copy(result_reg, body_reg));
        self.drop_arm_bindings(&bound_regs);
        exit_jumps.push(self.code.len());
        self.emit(OpCode::Jmp(0));

        let next_addr = self.code.len();
        self.patch_jz_at(jz_idx, next_addr)?;
        if let Some(idx) = guard_jz { self.patch_jz_at(idx, next_addr)?; }
        Ok(())
    }

    fn compile_record_pattern_arm_guarded(
        &mut self,
        rty: &[String],
        fields: &[ast::FieldPattern],
        body: &ast::Spanned<ast::Expr>,
        scrutinee_reg: Register,
        result_reg: Register,
        guard: Option<&ast::Spanned<ast::Expr>>,
        exit_jumps: &mut Vec<usize>,
    ) -> Result<(), String> {
        let type_name = rty.last().cloned()
            .ok_or_else(|| "Record pattern missing type name".to_string())?;
        let field_order = self.layouts.records.get(&type_name).cloned()
            .ok_or_else(|| format!("Unknown record type: {}", type_name))?;
        let mut bound_regs = Vec::new();
        for fp in fields {
            if let Some(idx) = field_order.fields.iter().position(|n| n == &fp.name) {
                let bind_name = match &fp.pattern {
                    Some(p) => if let ast::Pattern::Bind(n) = &p.node { Some(n.clone()) } else { None },
                    None => Some(fp.name.clone()),
                };
                if let Some(n) = bind_name {
                    let r = self.alloc_register()?;
                    let offset = super::scaffold::to_u16(idx, "Record pattern field offset")?;
                    self.emit(OpCode::Ld(r, scrutinee_reg, offset));
                    if self.typed_ld {
                        if let Some(ft) = field_order.field_types.get(idx) {
                            if super::data::type_is_unboxed(ft) { self.set_reg_handle(r, false); }
                        }
                    }
                    self.var_to_reg.insert(n, r);
                    bound_regs.push(r);
                }
            }
        }
        let guard_jz = self.compile_guard_check(guard)?;
        let body_reg = self.compile_expr(body)?;
        self.emit(OpCode::Copy(result_reg, body_reg));
        self.drop_arm_bindings(&bound_regs);
        exit_jumps.push(self.code.len());
        self.emit(OpCode::Jmp(0));
        let next_addr = self.code.len();
        if let Some(idx) = guard_jz { self.patch_jz_at(idx, next_addr)?; }
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

    fn compile_guard_check(
        &mut self,
        guard: Option<&ast::Spanned<ast::Expr>>,
    ) -> Result<Option<usize>, String> {
        let Some(g) = guard else { return Ok(None) };
        let g_reg = self.compile_expr(g)?;
        let idx = self.code.len();
        self.emit(OpCode::Jz(g_reg, 0));
        Ok(Some(idx))
    }

    fn match_covers_all_variants(&self, arms: &[ast::MatchArm]) -> bool {
        let mut covered: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut type_name: Option<String> = None;
        for arm in arms {
            if arm.guard.is_some() { continue; }
            let ctor = match &arm.pattern.node {
                ast::Pattern::Variant { ty, .. } => ty.last(),
                ast::Pattern::Bind(name) if self.layouts.variants.contains_key(name) => Some(name),
                _ => continue,
            };
            let Some(ctor) = ctor else { continue };
            let Some(info) = self.layouts.variants.get(ctor) else { continue };
            type_name.get_or_insert_with(|| info.type_name.clone());
            covered.insert(ctor.clone());
        }
        let Some(tn) = type_name else { return false };
        self.layouts.variants.iter()
            .filter(|(_, v)| v.type_name == tn)
            .all(|(c, _)| covered.contains(c))
    }
}
