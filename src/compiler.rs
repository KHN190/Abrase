use std::collections::HashMap;
use crate::ast;
use crate::bytecode;
use crate::host;

pub struct Compiler<'a> {
    globals: &'a HashMap<String, host::NativeFn>,
    modules: &'a HashMap<String, host::Module>,
}

impl<'a> Compiler<'a> {
    pub fn new(
        globals: &'a HashMap<String, host::NativeFn>,
        modules: &'a HashMap<String, host::Module>,
    ) -> Self {
        Self { globals, modules }
    }

    pub fn compile(&mut self, _ast: &[ast::Decl]) -> Result<bytecode::Chunk, String> {
        unimplemented!()
    }
}