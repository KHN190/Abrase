use crate::ast;
use crate::bytecode::{OpCode, Register};
use crate::compiler::Compiler;

impl Compiler {
    pub(in crate::compiler) fn compile_closure(
        &mut self,
        span: ast::Span,
    ) -> Result<Register, String> {
        let info = self.closure_by_span.get(&span).cloned()
            .ok_or_else(|| format!(
                "internal: closure at {:?} not registered by pre-pass", span
            ))?;

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
        Ok(env_reg)
    }
}
