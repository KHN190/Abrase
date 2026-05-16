use polka::{Chunk, Module, FRAME_REGS};

pub struct LoadedModule {
    pub module: Module,
}

// Validate per-frame register budget and entry index before running.
pub fn load(module: Module) -> Result<LoadedModule, String> {
    for (i, chunk) in module.functions.iter().enumerate() {
        if let Chunk::Bytecode(b) = chunk {
            if b.reg_count > FRAME_REGS {
                return Err(format!(
                    "module load: fn {} has reg_count {} > frame budget {}",
                    i, b.reg_count, FRAME_REGS
                ));
            }
            if b.param_count > b.reg_count {
                return Err(format!(
                    "module load: fn {} has param_count {} > reg_count {}",
                    i, b.param_count, b.reg_count
                ));
            }
        }
    }
    if module.entry >= module.functions.len() {
        return Err(format!(
            "module load: entry {} out of range (functions: {})",
            module.entry,
            module.functions.len()
        ));
    }
    Ok(LoadedModule { module })
}
