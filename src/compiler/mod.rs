pub mod hir;
pub mod lower;
pub mod codegen;
pub mod mono;
pub mod closures;
pub mod effects;

use crate::ast;
use crate::bytecode::{Chunk, OpCode, Register, Module};
use crate::vm::Value;
use std::collections::HashMap;

pub struct Compiler {
    pub(super) constants: Vec<Value>,
    pub(super) code: Vec<OpCode>,
    pub(super) next_reg: u8,
    pub(super) var_to_reg: HashMap<String, Register>,
    pub(super) func_map: HashMap<String, usize>,
    pub(super) functions: Vec<Chunk>,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            constants: Vec::new(),
            code: Vec::new(),
            next_reg: 0,
            var_to_reg: HashMap::new(),
            func_map: HashMap::new(),
            functions: Vec::new(),
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
            reg_count: self.next_reg as usize,
        })
    }

    pub fn compile_module(&mut self, ast: &[ast::Decl]) -> Result<Module, String> {
        let mut fn_decls = Vec::new();

        for decl in ast {
            if let ast::Decl::Fn(fn_decl) = decl {
                let idx = self.functions.len();
                self.func_map.insert(fn_decl.name.clone(), idx);
                self.functions.push(Chunk {
                    code: Vec::new(),
                    constants: Vec::new(),
                    reg_count: 0,
                });
                fn_decls.push((idx, fn_decl.clone()));
            }
        }

        let entry = self.func_map.get("main").copied()
            .ok_or("No main function found")?;

        for (idx, fn_decl) in fn_decls {
            let chunk = self.compile_fn(&fn_decl)?;
            self.functions[idx] = chunk;
        }

        Ok(Module {
            functions: self.functions.clone(),
            entry,
        })
    }

    fn compile_fn(&mut self, fn_decl: &ast::FnDecl) -> Result<Chunk, String> {
        let saved_code = std::mem::take(&mut self.code);
        let saved_constants = std::mem::take(&mut self.constants);
        let saved_next_reg = self.next_reg;
        let saved_var_to_reg = std::mem::take(&mut self.var_to_reg);

        self.next_reg = 0;

        for param in &fn_decl.params {
            if let ast::Param::Named { pattern, .. } = param {
                if let ast::Pattern::Bind(name) = &pattern.node {
                    let reg = self.alloc_register()?;
                    self.var_to_reg.insert(name.clone(), reg);
                }
            }
        }

        let result_reg = self.compile_block(&fn_decl.body)?;
        self.emit(OpCode::Ret(result_reg));

        let reg_count = self.next_reg as usize;
        let chunk = Chunk {
            code: std::mem::take(&mut self.code),
            constants: std::mem::take(&mut self.constants),
            reg_count,
        };

        self.code = saved_code;
        self.constants = saved_constants;
        self.next_reg = saved_next_reg;
        self.var_to_reg = saved_var_to_reg;

        Ok(chunk)
    }
}
