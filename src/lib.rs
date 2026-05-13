pub mod ast;
pub mod lexer;
pub mod parser;
pub mod ty;
pub mod typeck;
pub mod host;
pub mod vm;
pub mod compiler;

pub mod bytecode {
    pub enum OpCode {
        PushConst(usize),
        Add,
        Jump(usize),
        Call(usize),
    }
    pub struct Chunk {
        pub code: Vec<OpCode>,
        pub constants: Vec<super::vm::Value>,
    }
}

