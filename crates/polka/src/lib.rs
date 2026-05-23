// OpCode design frozen; type-agnostic bytecode, type from OpCode + masks.

pub mod value;
pub mod cartridge;

pub use value::{Value, HANDLE_NONE, HANDLE_SLOT_MAX};

// 64 fits frame handle-mask in one u64.
pub const FRAME_REGS: usize = 64;
pub const FRAME_MASK_WORDS: usize = FRAME_REGS / 64;

pub const DISPATCH_ID: u8 = 0xE0;
pub const DISPATCH_PORT_LOOKUP: u8 = 0x00;
pub const DISPATCH_PORT_POP_HANDLER: u8 = 0x01;
pub const DISPATCH_PORT_ENV: u8 = 0x02;
pub const DISPATCH_PORT_RETURN_FN: u8 = 0x03;
pub const DISPATCH_PORT_RETURN_ENV: u8 = 0x04;
pub const DISPATCH_NO_MATCH: u16 = 0xFFFF;

pub const REGION_ID: u8 = 0xE1;
pub const REGION_PORT_PUSH: u8 = 0x00;
pub const REGION_PORT_POP: u8 = 0x01;
pub const REGION_PORT_FORGET: u8 = 0x02;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Register(pub u8);

impl Register {
    pub fn to_usize(self) -> usize {
        self.0 as usize
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
    FNeg(Register, Register),
    FLt(Register, Register, Register),
    FEq(Register, Register, Register),

    Eq(Register, Register, Register),
    Neq(Register, Register, Register),
    Lt(Register, Register, Register),
    Gt(Register, Register, Register),
    Lte(Register, Register, Register),
    Gte(Register, Register, Register),

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

    AddImm(Register, Register, i8),
    SubImm(Register, Register, i8),

    Alloc(Register, u16),
    Drop(Register),

    Dei(Register, Register),
    Deo(Register, Register),

    Handle(Register, u16),
    Resume(Register, Register),

    Raise(Register, Register, Register),
}

#[derive(Clone, Default, Debug)]
pub struct BytecodeChunk {
    pub code: Vec<OpCode>,
    pub constants: Vec<u64>,
    // Handle-bit constants store string_constants index; loader replaces with real heap handle.
    pub const_mask: Vec<u64>,
    pub string_constants: Vec<String>,
    pub reg_count: usize,
    pub param_count: usize,
}

impl BytecodeChunk {
    #[inline]
    pub fn const_is_handle(&self, idx: u16) -> bool {
        let i = idx as usize;
        let word = i / 64;
        let bit = i % 64;
        self.const_mask.get(word).map_or(false, |w| (w >> bit) & 1 == 1)
    }
}

#[derive(Clone, Debug)]
pub struct NativeChunk {
    pub name: String,
    pub param_count: usize,
}

#[derive(Clone, Debug)]
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

#[derive(Debug, Default)]
pub struct Module {
    pub functions: Vec<Chunk>,
    pub entry: usize,
    pub flags: u16,
}

pub const CART_FLAG_INT32_SAFE: u16 = 0x0001;

#[inline(always)]
pub fn mask_bit_set(mask: &[u64], idx: usize) -> bool {
    let word = idx / 64;
    let bit = idx % 64;
    mask.get(word).map_or(false, |w| (w >> bit) & 1 == 1)
}

#[inline(always)]
pub fn mask_set(mask: &mut [u64], idx: usize, on: bool) {
    let word = idx / 64;
    let bit = idx % 64;
    if let Some(w) = mask.get_mut(word) {
        if on { *w |= 1u64 << bit; } else { *w &= !(1u64 << bit); }
    }
}
