use crate::vm::Value;
use std::rc::Rc;

pub type NativeFn = Rc<dyn Fn(&[Value]) -> Result<Value, String>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Register(pub u8);

impl Register {
    pub fn to_usize(self) -> usize {
        self.0 as usize
    }
}

#[derive(Debug, Clone)]
pub enum OpCode {
    Add(Register, Register, Register),
    Sub(Register, Register, Register),
    Mul(Register, Register, Register),
    Div(Register, Register, Register),
    Mod(Register, Register, Register),
    Neg(Register, Register),
    FAdd(Register, Register, Register),
    FSub(Register, Register, Register),
    FMul(Register, Register, Register),
    FDiv(Register, Register, Register),

    Eq(Register, Register, Register),
    Neq(Register, Register, Register),
    Lt(Register, Register, Register),
    Gt(Register, Register, Register),
    Lte(Register, Register, Register),
    Gte(Register, Register, Register),
    FLt(Register, Register, Register),

    And(Register, Register, Register),
    Or(Register, Register, Register),
    Xor(Register, Register, Register),
    Shl(Register, Register, Register),
    Shr(Register, Register, Register),

    Jmp(i16),
    Jz(Register, i16),
    Jnz(Register, i16),
    Call(Register, u16),
    Ret(Register),

    PushConst(Register, u16),
    Copy(Register, Register),
    Move(Register, Register),

    Ld(Register, Register, u16),
    St(Register, Register, u16),
    LdIdx(Register, Register, Register),
    StIdx(Register, Register, Register),
    Lea(Register, Register, u16),
    Ref(Register, Register),

    Alloc(Register, u16),
    Free(Register),
    Drop(Register),

    Dei(Register, Register),
    Deo(Register, Register),

    Handle(Register, u16),
    Resume(Register),
}

#[derive(Clone)]
pub struct BytecodeChunk {
    pub code: Vec<OpCode>,
    pub constants: Vec<Value>,
    pub reg_count: usize,
    pub param_count: usize,
}

#[derive(Clone)]
pub struct NativeChunk {
    pub func: NativeFn,
    pub param_count: usize,
}

#[derive(Clone)]
pub enum Chunk {
    Bytecode(BytecodeChunk),
    Native(NativeChunk),
}

impl Chunk {
    pub fn param_count(&self) -> usize {
        match self {
            Chunk::Bytecode(b) => b.param_count,
            Chunk::Native(n) => n.param_count,
        }
    }

    pub fn as_bytecode(&self) -> Option<&BytecodeChunk> {
        if let Chunk::Bytecode(b) = self { Some(b) } else { None }
    }
}

pub struct Module {
    pub functions: Vec<Chunk>,
    pub entry: usize,
}
