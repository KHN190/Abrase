// Codegen for the closure expression: allocate an env heap object holding
// one slot per capture, populate it from the surrounding scope, and return
// the env handle as the closure's runtime value.
//
// The closures pre-pass (compiler/closures.rs) does the lambda lifting; this
// file only handles the per-site env construction.

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
        self.emit(OpCode::Alloc(env_reg, n.max(1) as u16));
        for (i, cap) in info.captures.iter().enumerate() {
            let src = *self.var_to_reg.get(&cap.name)
                .ok_or_else(|| format!(
                    "internal: closure capture '{}' not in scope at closure site",
                    cap.name
                ))?;
            // `move |...|` takes src; default is Copy-then-St so the outer binding survives.
            if info.is_move {
                self.emit(OpCode::St(src, env_reg, i as u16));
                self.var_to_reg.remove(&cap.name);
                self.var_types.remove(&cap.name);
            } else {
                let tmp = self.alloc_register()?;
                self.emit(OpCode::Copy(tmp, src));
                self.emit(OpCode::St(tmp, env_reg, i as u16));
            }
        }
        Ok(env_reg)
    }
}
