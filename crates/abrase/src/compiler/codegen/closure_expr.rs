use crate::ast;
use crate::bytecode::{OpCode, Register, Value};
use crate::compiler::Compiler;

impl Compiler {
    // A closure compiles to a self-contained 2-slot value cell [fn_id, env]:
    //   slot 0 = lifted fn id (plain int)   slot 1 = env cell (captures)
    pub(in crate::compiler) fn compile_closure(
        &mut self,
        span: ast::Span,
    ) -> Result<Register, String> {
        let info = self.closure_by_span.get(&span).cloned()
            .ok_or_else(|| format!(
                "internal: closure at {:?} not registered by pre-pass", span
            ))?;
        let fn_id = *self.func_map.get(&info.lifted_fn)
            .ok_or_else(|| format!("internal: lifted closure fn '{}' not in fn table", info.lifted_fn))?;

        let env_reg = self.alloc_register()?;
        let n = info.captures.len();
        let alloc_size = super::scaffold::to_u16(n.max(1), "Closure env size")?;
        self.emit(OpCode::Alloc(env_reg, alloc_size));
        for (i, cap) in info.captures.iter().enumerate() {
            let offset = super::scaffold::to_u16(i, "Closure env offset")?;
            let src = match self.var_to_reg.get(&cap.name).copied() {
                Some(r) => r,
                None => {
                    let outer_env_reg = self.var_to_reg.get("__env").copied()
                        .ok_or_else(|| format!(
                            "internal: closure capture '{}' not in scope at closure site",
                            cap.name
                        ))?;
                    let outer_offset = self.current_closure_layout.get(&cap.name).copied()
                        .ok_or_else(|| format!(
                            "internal: closure capture '{}' not in scope at closure site",
                            cap.name
                        ))?;
                    let tmp = self.alloc_register()?;
                    let off16 = super::scaffold::to_u16(outer_offset, "Outer env offset")?;
                    self.emit(OpCode::Ld(tmp, outer_env_reg, off16));
                    tmp
                }
            };
            if info.is_move {
                self.emit(OpCode::St(src, env_reg, offset));
                self.var_to_reg.remove(&cap.name);
                self.var_types.remove(&cap.name);
            } else {
                let tmp = self.alloc_register()?;
                self.emit(OpCode::Copy(tmp, src));
                self.emit(OpCode::St(tmp, env_reg, offset));
            }
        }

        // Wrap into the [fn_id, env] value cell.
        let cell = self.alloc_register()?;
        self.emit(OpCode::Alloc(cell, 2));
        let fid_reg = self.alloc_register()?;
        let idx = self.add_constant(Value::from_int(fn_id as i64))?;
        self.emit(OpCode::PushConst(fid_reg, idx));
        self.set_reg_handle(fid_reg, false);
        self.emit(OpCode::St(fid_reg, cell, 0));
        self.emit(OpCode::St(env_reg, cell, 1));
        Ok(cell)
    }
}
