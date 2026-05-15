pub mod hir;
pub mod lower;
pub mod codegen;
pub mod mono;
pub mod closures;
pub mod effects;

use crate::ast;
use crate::bytecode::{Chunk, OpCode, Register, Module};
use crate::error::{Error, ErrorCode};
use crate::vm::Value;
use std::collections::HashMap;

pub struct Compiler {
    pub(super) constants: Vec<Value>,
    pub(super) code: Vec<OpCode>,
    pub(super) next_reg: u8,
    pub(super) var_to_reg: HashMap<String, Register>,
    pub(super) var_types: HashMap<String, ast::Type>,
    pub(super) func_map: HashMap<String, usize>,
    pub(super) functions: Vec<Chunk>,
    pub errors: Vec<Error>,
    pub source: String,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            constants: Vec::new(),
            code: Vec::new(),
            next_reg: 0,
            var_to_reg: HashMap::new(),
            var_types: HashMap::new(),
            func_map: HashMap::new(),
            functions: Vec::new(),
            errors: Vec::new(),
            source: String::new(),
        }
    }

    pub fn with_source(mut self, source: String) -> Self {
        self.source = source;
        self
    }

    pub fn pretty_print_errors(&self) -> String {
        self.errors
            .iter()
            .map(|e| e.pretty_print(&self.source))
            .collect::<Vec<_>>()
            .join("\n")
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

    pub fn compile_module(&mut self, ast: &[ast::Decl]) -> Result<Module, Vec<Error>> {
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

        let entry = match self.func_map.get("main").copied() {
            Some(idx) => idx,
            None => {
                self.errors.push(Error::new(
                    ErrorCode::CodegenError,
                    ast::Span::new(0, 0),
                    "No main function found",
                ));
                return Err(self.errors.clone());
            }
        };

        for (idx, fn_decl) in fn_decls {
            let chunk = self.compile_fn(&fn_decl)?;
            self.functions[idx] = chunk;
        }

        Ok(Module {
            functions: self.functions.clone(),
            entry,
        })
    }

    fn compile_fn(&mut self, fn_decl: &ast::FnDecl) -> Result<Chunk, Vec<Error>> {
        let saved_code = std::mem::take(&mut self.code);
        let saved_constants = std::mem::take(&mut self.constants);
        let saved_next_reg = self.next_reg;
        let saved_var_to_reg = std::mem::take(&mut self.var_to_reg);
        let saved_var_types = std::mem::take(&mut self.var_types);

        self.next_reg = 0;

        for param in &fn_decl.params {
            if let ast::Param::Named { pattern, ty } = param {
                if let ast::Pattern::Bind(name) = &pattern.node {
                    match self.alloc_register() {
                        Ok(reg) => {
                            self.var_to_reg.insert(name.clone(), reg);
                            self.var_types.insert(name.clone(), ty.clone());
                        }
                        Err(_) => {
                            self.errors.push(Error::new(
                                ErrorCode::CodegenError,
                                pattern.span,
                                format!("Failed to allocate register for parameter '{}'", name),
                            ));
                        }
                    }
                }
            }
        }

        if !self.errors.is_empty() {
            return Err(self.errors.clone());
        }

        match self.compile_block(&fn_decl.body) {
            Ok(result_reg) => {
                self.emit(OpCode::Ret(result_reg));
            }
            Err(_) => {
                return Err(self.errors.clone());
            }
        }

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
        self.var_types = saved_var_types;

        Ok(chunk)
    }
}
