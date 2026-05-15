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
    PushConst(Register, usize),
    Copy(Register, Register),
    Move(Register, Register),
    Add(Register, Register, Register),
    Sub(Register, Register, Register),
    Mul(Register, Register, Register),
    Div(Register, Register, Register),
    Mod(Register, Register, Register),
    Eq(Register, Register, Register),
    Neq(Register, Register, Register),
    Lt(Register, Register, Register),
    Gt(Register, Register, Register),
    Lte(Register, Register, Register),
    Gte(Register, Register, Register),
    Jz(Register, usize),
    Jnz(Register, usize),
    Jmp(usize),
    Ret(Register),
    Call(Register, usize, Register, u8),
    MakeShared(Register, Register),
    Ref(Register, Register),
    Deref(Register, Register),
    Drop(Register),
    MakeRecord(Register, u32, Register, u8),
    GetField(Register, Register, u32),
    GetTag(Register, Register),
    MakeArray(Register, Register, u8),
    GetIndex(Register, Register, Register),
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
