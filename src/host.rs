use std::collections::HashMap;
use crate::vm::{VirtualMachine, Value};
use crate::compiler::Compiler;
use crate::parser::Parser;
use crate::lexer::Lexer;

pub type NativeFn = Box<dyn Fn(&[Value]) -> Result<Value, String>>;

pub struct HostModule {
    pub name: String,
    pub functions: HashMap<String, NativeFn>,
}

impl HostModule {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            functions: HashMap::new(),
        }
    }

    pub fn register(&mut self, name: &str, func: impl Fn(&[Value]) -> Result<Value, String> + 'static) {
        self.functions.insert(name.to_string(), Box::new(func));
    }
}

pub struct Runtime {
    vm: VirtualMachine,
    pub globals: HashMap<String, NativeFn>,
    pub modules: HashMap<String, HostModule>,
}

impl Runtime {
    pub fn new() -> Self {
        Self {
            vm: VirtualMachine::new(),
            globals: HashMap::new(),
            modules: HashMap::new(),
        }
    }

    pub fn register_global(&mut self, name: &str, func: impl Fn(&[Value]) -> Result<Value, String> + 'static) {
        self.globals.insert(name.to_string(), Box::new(func));
    }

    pub fn register_module(&mut self, module: HostModule) {
        self.modules.insert(module.name.clone(), module);
    }

    pub fn eval(&mut self, source: &str) -> Result<Value, String> {
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);

        let ast = parser.parse_program();
        if !parser.errors.is_empty() {
            let msgs: Vec<String> = parser.errors
                .iter()
                .map(|e| format!("[{}:{}] {}", e.span.line, e.span.col, e.message))
                .collect();
            return Err(msgs.join("\n"));
        }

        let chunk = Compiler::new().compile(&ast)?;
        self.vm.run(&chunk)
    }
}
