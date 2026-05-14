pub mod hir;
pub mod lower;
pub mod codegen;
pub mod mono;
pub mod closures;
pub mod effects;

use crate::ast;
use crate::bytecode::{Chunk, OpCode, Register};
use crate::vm::Value;
use std::collections::HashMap;

pub struct Compiler {
    pub(super) constants: Vec<Value>,
    pub(super) code: Vec<OpCode>,
    pub(super) next_reg: u8,
    pub(super) var_to_reg: HashMap<String, Register>,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            constants: Vec::new(),
            code: Vec::new(),
            next_reg: 0,
            var_to_reg: HashMap::new(),
        }
    }

    pub fn compile(&mut self, ast: &[ast::Decl]) -> Result<Chunk, String> {
        for decl in ast {
            if let ast::Decl::Fn(fn_decl) = decl {
                if fn_decl.name == "main" {
                    let result_reg = self.compile_block(&fn_decl.body)?;
                    self.emit(OpCode::Ret(result_reg));
                }
            }
        }
        Ok(Chunk {
            code: self.code.clone(),
            constants: self.constants.clone(),
        })
    }
}
