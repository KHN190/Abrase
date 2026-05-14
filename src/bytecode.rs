use crate::vm::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Register(pub u8);

impl Register {
    pub fn to_usize(self) -> usize {
        self.0 as usize
    }
}

#[derive(Debug, Clone)]
pub enum OpCode {
    // Memory
    PushConst(Register, usize),
    Mov(Register, Register),
    // Arithmetic
    Add(Register, Register, Register),
    Sub(Register, Register, Register),
    Mul(Register, Register, Register),
    Div(Register, Register, Register),
    Mod(Register, Register, Register),
    // Control flow
    Ret(Register),
}

pub struct Chunk {
    pub code: Vec<OpCode>,
    pub constants: Vec<Value>,
}

pub struct Module {
    pub functions: Vec<Chunk>,
    pub entry: usize,
}
