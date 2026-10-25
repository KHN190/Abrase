use alloc::string::String;
use polka::Module;
use super::interpreter::register::validate_module_register_budget;

pub struct LoadedModule {
    pub module: Module,
}

pub fn load(module: Module) -> Result<LoadedModule, String> {
    validate(&module)?;
    Ok(LoadedModule { module })
}

pub fn validate(module: &Module) -> Result<(), String> {
    validate_module_register_budget(module)?;
    if module.entry >= module.functions.len() {
        return Err(format!(
            "module load: entry {} out of range (functions: {})",
            module.entry,
            module.functions.len()
        ));
    }
    Ok(())
}
