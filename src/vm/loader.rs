use crate::bytecode::Module;

pub struct LoadedModule {
    pub module: Module,
}

pub fn load(module: Module) -> LoadedModule {
    LoadedModule { module }
}
