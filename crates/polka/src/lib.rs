pub mod value;
pub mod cartridge;

pub use value::Value;

pub const FRAME_REGS: usize = 256;

pub const DISPATCH_ID: u8 = 0xE0;
pub const DISPATCH_PORT_LOOKUP: u8 = 0x00;
pub const DISPATCH_NO_MATCH: u16 = 0xFFFF;

pub const REGION_ID: u8 = 0xE1;
pub const REGION_PORT_PUSH: u8 = 0x00;
pub const REGION_PORT_POP: u8 = 0x01;

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
    CallReg(Register, Register),
    Ret(Register),

    PushConst(Register, u16),
    Copy(Register, Register),
    Move(Register, Register),

    Ld(Register, Register, u16),
    St(Register, Register, u16),
    LdIdx(Register, Register, Register),
    StIdx(Register, Register, Register),
    Ref(Register, Register),

    AddImm(Register, Register, i8),
    SubImm(Register, Register, i8),

    Alloc(Register, u16),
    Drop(Register),

    Dei(Register, Register),
    Deo(Register, Register),

    Handle(Register, u16),
    Resume(Register),
}

#[derive(Clone, Default)]
pub struct BytecodeChunk {
    pub code: Vec<OpCode>,
    pub constants: Vec<Value>,
    pub string_constants: Vec<String>,
    pub reg_count: usize,
    pub param_count: usize,
}

// Declaration of a native (host-implemented) function slot in the module's
// function table. The function body itself lives in the runtime — polka only
// records the name and arity so the wire format stays pure data.
#[derive(Clone, Debug)]
pub struct NativeChunk {
    pub name: String,
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
    pub device_mask: [u8; 32],
}

impl Module {
    pub fn require_device(&mut self, id: u8) {
        self.device_mask[(id / 8) as usize] |= 1 << (id % 8);
    }

    pub fn requires_device(&self, id: u8) -> bool {
        (self.device_mask[(id / 8) as usize] >> (id % 8)) & 1 == 1
    }
}
