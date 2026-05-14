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
    // Comparison
    Eq(Register, Register, Register),
    Neq(Register, Register, Register),
    Lt(Register, Register, Register),
    Gt(Register, Register, Register),
    Lte(Register, Register, Register),
    Gte(Register, Register, Register),
    // Control flow
    Jz(Register, usize),
    Jnz(Register, usize),
    Jmp(usize),
    Ret(Register),
    // Functions
    Call(Register, usize, Register, u8), // dest, func_id, first_arg_reg, arg_count
}

#[derive(Clone)]
pub struct Chunk {
    pub code: Vec<OpCode>,
    pub constants: Vec<Value>,
    pub reg_count: usize,
}

pub struct Module {
    pub functions: Vec<Chunk>,
    pub entry: usize,
}
