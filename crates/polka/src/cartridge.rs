use crate::{BytecodeChunk, Chunk, Module, NativeChunk, OpCode, Register};

pub const MAGIC: u32 = 0xECFF_00EC;
pub const VERSION: u16 = 0x0100;

const KIND_BYTECODE: u8 = 0;
const KIND_NATIVE: u8 = 1;

#[derive(Debug)]
pub enum EncodeError {
    /// An `ld`/`st` offset operand exceeds the 255-byte wire-format limit.
    OffsetTooLarge { value: u16, op: &'static str },
    /// A count or index value is larger than its wire-format field can hold.
    CountOverflow { value: usize, what: &'static str },
    /// An import name is longer than the cart's `name_len` field (u16) allows.
    NameTooLong { length: usize },
}

impl std::fmt::Display for EncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncodeError::OffsetTooLarge { value, op } =>
                write!(f, "{} offset {} exceeds the 255-byte cartridge limit", op, value),
            EncodeError::CountOverflow { value, what } =>
                write!(f, "{} count {} is too large to fit in the cartridge format", what, value),
            EncodeError::NameTooLong { length } =>
                write!(f, "import name is too long ({} bytes; cartridge limit is 65 535)", length),
        }
    }
}

impl std::error::Error for EncodeError {}

#[derive(Debug)]
pub enum LoadError {
    /// The file doesn't start with the Polka magic — almost certainly not a cart.
    NotACartridge,
    /// Magic matched, but the version field is one this runtime can't load.
    UnsupportedVersion(u16),
    /// The cart parsed past the header but is structurally malformed.
    /// `offset` is the byte position in the input where parsing failed.
    Corrupt { offset: usize, kind: Corruption },
}

#[derive(Debug)]
pub enum Corruption {
    /// Hit end-of-input while a wire-format field still needed bytes.
    Truncated,
    /// A function-table entry has an unrecognized kind byte.
    UnknownKind(u8),
    /// An instruction byte does not match any defined opcode.
    UnknownOpcode(u8),
    /// A string-pool entry or import name is not valid UTF-8.
    InvalidUtf8,
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::NotACartridge =>
                write!(f, "not a Polka cartridge (.pk file expected)"),
            LoadError::UnsupportedVersion(v) =>
                write!(f, "cartridge format version 0x{:04X} is not supported by this runtime", v),
            LoadError::Corrupt { offset, kind } => match kind {
                Corruption::Truncated =>
                    write!(f, "cartridge ends unexpectedly at byte {}", offset),
                Corruption::UnknownKind(k) =>
                    write!(f, "malformed cartridge at byte {}: unknown function entry kind {}", offset, k),
                Corruption::UnknownOpcode(b) =>
                    write!(f, "malformed cartridge at byte {}: unknown opcode 0x{:02X}", offset, b),
                Corruption::InvalidUtf8 =>
                    write!(f, "malformed cartridge at byte {}: invalid UTF-8 in string pool", offset),
            }
        }
    }
}

impl std::error::Error for LoadError {}

pub fn write_pk(module: &Module) -> Result<Vec<u8>, EncodeError> {
    let mut out = Vec::new();
    out.extend_from_slice(&MAGIC.to_le_bytes());
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    let entry = u32::try_from(module.entry)
        .map_err(|_| EncodeError::CountOverflow { value: module.entry, what: "entry fn_id" })?;
    out.extend_from_slice(&entry.to_le_bytes());

    let fn_count = u32::try_from(module.functions.len())
        .map_err(|_| EncodeError::CountOverflow { value: module.functions.len(), what: "function" })?;
    out.extend_from_slice(&fn_count.to_le_bytes());
    for chunk in &module.functions {
        match chunk {
            Chunk::Bytecode(bc) => write_bc_header(&mut out, bc)?,
            Chunk::Native(n) => write_native_header(&mut out, n)?,
        }
    }
    for chunk in &module.functions {
        match chunk {
            Chunk::Bytecode(bc) => write_bc_payload(&mut out, bc)?,
            Chunk::Native(n) => write_native_payload(&mut out, n),
        }
    }
    Ok(out)
}

pub fn read_pk(data: &[u8]) -> Result<Module, LoadError> {
    let mut r = Reader::new(data);

    let magic = r.read_u32().map_err(|_| LoadError::NotACartridge)?;
    if magic != MAGIC { return Err(LoadError::NotACartridge); }

    let version = r.read_u16()?;
    if version != VERSION { return Err(LoadError::UnsupportedVersion(version)); }
    let _flags = r.read_u16()?;
    let entry = r.read_u32()? as usize;

    let fn_count = r.read_u32()? as usize;
    let mut headers: Vec<FnHeader> = Vec::new(); // fn_count is attacker-controlled; avoid with_capacity
    for _ in 0..fn_count {
        headers.push(read_fn_header(&mut r)?);
    }
    let mut functions: Vec<Chunk> = Vec::new();
    for h in headers {
        functions.push(read_fn_payload(&mut r, h)?);
    }
    Ok(Module { functions, entry })
}

fn write_bc_header(out: &mut Vec<u8>, bc: &BytecodeChunk) -> Result<(), EncodeError> {
    let const_count = u16::try_from(bc.constants.len())
        .map_err(|_| EncodeError::CountOverflow { value: bc.constants.len(), what: "constant" })?;
    let string_count = u16::try_from(bc.string_constants.len())
        .map_err(|_| EncodeError::CountOverflow { value: bc.string_constants.len(), what: "string" })?;
    let code_count = u32::try_from(bc.code.len())
        .map_err(|_| EncodeError::CountOverflow { value: bc.code.len(), what: "instruction" })?;
    let param_count = u8::try_from(bc.param_count)
        .map_err(|_| EncodeError::CountOverflow { value: bc.param_count, what: "param" })?;
    let reg_count = u8::try_from(bc.reg_count)
        .map_err(|_| EncodeError::CountOverflow { value: bc.reg_count, what: "register" })?;

    out.push(KIND_BYTECODE);
    out.push(param_count);
    out.push(reg_count);
    out.push(0); // pad
    out.extend_from_slice(&const_count.to_le_bytes());
    out.extend_from_slice(&string_count.to_le_bytes());
    out.extend_from_slice(&code_count.to_le_bytes());
    Ok(())
}

fn write_native_header(out: &mut Vec<u8>, n: &NativeChunk) -> Result<(), EncodeError> {
    let param_count = u8::try_from(n.param_count)
        .map_err(|_| EncodeError::CountOverflow { value: n.param_count, what: "param" })?;
    let name_len = u16::try_from(n.name.len())
        .map_err(|_| EncodeError::NameTooLong { length: n.name.len() })?;
    out.push(KIND_NATIVE);
    out.push(param_count);
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&name_len.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    Ok(())
}

fn write_bc_payload(out: &mut Vec<u8>, bc: &BytecodeChunk) -> Result<(), EncodeError> {
    for &v in &bc.constants {
        out.extend_from_slice(&v.to_le_bytes());
    }
    let mask_bytes = (bc.constants.len() + 7) / 8;
    let mut mask = vec![0u8; mask_bytes];
    for i in 0..bc.constants.len() {
        if bc.const_is_handle(i as u16) {
            mask[i / 8] |= 1 << (i % 8);
        }
    }
    out.extend_from_slice(&mask);
    for s in &bc.string_constants {
        let len = u32::try_from(s.len())
            .map_err(|_| EncodeError::CountOverflow { value: s.len(), what: "string byte" })?;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(s.as_bytes());
    }
    for op in &bc.code {
        out.extend_from_slice(&encode_op(op)?);
    }
    Ok(())
}

fn write_native_payload(out: &mut Vec<u8>, n: &NativeChunk) {
    out.extend_from_slice(n.name.as_bytes());
}

enum FnHeader {
    Bytecode { param_count: u8, reg_count: u8, const_count: u16, string_count: u16, code_count: u32 },
    Native   { param_count: u8, name_len: u16 },
}

fn read_fn_header(r: &mut Reader) -> Result<FnHeader, LoadError> {
    let kind_offset = r.pos;
    let kind = r.read_u8()?;
    match kind {
        KIND_BYTECODE => {
            let param_count = r.read_u8()?;
            let reg_count = r.read_u8()?;
            let _pad = r.read_u8()?;
            let const_count = r.read_u16()?;
            let string_count = r.read_u16()?;
            let code_count = r.read_u32()?;
            Ok(FnHeader::Bytecode { param_count, reg_count, const_count, string_count, code_count })
        }
        KIND_NATIVE => {
            let param_count = r.read_u8()?;
            let _pad = r.read_u16()?;
            let name_len = r.read_u16()?;
            let _pad = r.read_u32()?;
            Ok(FnHeader::Native { param_count, name_len })
        }
        k => Err(LoadError::Corrupt {
            offset: kind_offset,
            kind: Corruption::UnknownKind(k),
        }),
    }
}

fn read_fn_payload(r: &mut Reader, header: FnHeader) -> Result<Chunk, LoadError> {
    match header {
        FnHeader::Bytecode { param_count, reg_count, const_count, string_count, code_count } => {
            let mut constants = Vec::new();
            for _ in 0..const_count {
                constants.push(r.read_u64()?);
            }
            let mask_bytes = (const_count as usize + 7) / 8;
            let mask_raw = r.take(mask_bytes)?.to_vec();
            let mask_word_count = (const_count as usize + 63) / 64;
            let mut const_mask = vec![0u64; mask_word_count];
            for i in 0..const_count as usize {
                if (mask_raw[i / 8] >> (i % 8)) & 1 == 1 {
                    const_mask[i / 64] |= 1 << (i % 64);
                }
            }
            let mut string_constants = Vec::new();
            for _ in 0..string_count {
                let len = r.read_u32()? as usize;
                let utf8_offset = r.pos;
                let bytes = r.take(len)?.to_vec();
                let s = String::from_utf8(bytes).map_err(|_| LoadError::Corrupt {
                    offset: utf8_offset,
                    kind: Corruption::InvalidUtf8,
                })?;
                string_constants.push(s);
            }
            let mut code = Vec::new();
            for _ in 0..code_count {
                let op_offset = r.pos;
                let raw = r.take(4)?;
                let mut buf = [0u8; 4];
                buf.copy_from_slice(raw);
                code.push(decode_op(buf, op_offset)?);
            }
            Ok(Chunk::Bytecode(BytecodeChunk {
                code,
                constants,
                const_mask,
                string_constants,
                reg_count: reg_count as usize,
                param_count: param_count as usize,
            }))
        }
        FnHeader::Native { param_count, name_len } => {
            let utf8_offset = r.pos;
            let bytes = r.take(name_len as usize)?.to_vec();
            let name = String::from_utf8(bytes).map_err(|_| LoadError::Corrupt {
                offset: utf8_offset,
                kind: Corruption::InvalidUtf8,
            })?;
            Ok(Chunk::Native(NativeChunk { name, param_count: param_count as usize }))
        }
    }
}
//
// Each instruction is 4 bytes: opcode byte + 3 operand bytes. Layout per
// opcode follows §3 of the spec; see Appendix-Wire-Format.md for the full
// per-opcode byte table.

fn r(reg: Register) -> u8 { reg.0 }

fn encode_op(op: &OpCode) -> Result<[u8; 4], EncodeError> {
    use OpCode::*;
    Ok(match op {
        Add(a,b,c) => [0x00, r(*a), r(*b), r(*c)],
        Sub(a,b,c) => [0x01, r(*a), r(*b), r(*c)],
        Mul(a,b,c) => [0x02, r(*a), r(*b), r(*c)],
        Div(a,b,c) => [0x03, r(*a), r(*b), r(*c)],
        Mod(a,b,c) => [0x04, r(*a), r(*b), r(*c)],
        Neg(a,b)   => [0x05, r(*a), r(*b), 0],

        Eq(a,b,c)  => [0x06, r(*a), r(*b), r(*c)],
        Neq(a,b,c) => [0x07, r(*a), r(*b), r(*c)],
        Lt(a,b,c)  => [0x08, r(*a), r(*b), r(*c)],
        Gt(a,b,c)  => [0x09, r(*a), r(*b), r(*c)],
        Lte(a,b,c) => [0x0a, r(*a), r(*b), r(*c)],
        Gte(a,b,c) => [0x0b, r(*a), r(*b), r(*c)],

        And(a,b,c) => [0x0c, r(*a), r(*b), r(*c)],
        Or (a,b,c) => [0x0d, r(*a), r(*b), r(*c)],
        Xor(a,b,c) => [0x0e, r(*a), r(*b), r(*c)],
        Shl(a,b,c) => [0x0f, r(*a), r(*b), r(*c)],
        Shr(a,b,c) => [0x10, r(*a), r(*b), r(*c)],

        Jmp(off)   => { let o = off.to_le_bytes(); [0x11, 0, o[0], o[1]] }
        Jz (a,off) => { let o = off.to_le_bytes(); [0x12, r(*a), o[0], o[1]] }
        Jnz(a,off) => { let o = off.to_le_bytes(); [0x13, r(*a), o[0], o[1]] }
        Call(a,id) => { let o = id.to_le_bytes();  [0x14, r(*a), o[0], o[1]] }
        Ret(a)     => [0x15, r(*a), 0, 0],
        CallReg(a,b) => [0x16, r(*a), r(*b), 0],

        PushConst(a, idx) => { let o = idx.to_le_bytes(); [0x17, r(*a), o[0], o[1]] }
        Copy(a,b)         => [0x18, r(*a), r(*b), 0],
        Move(a,b)         => [0x19, r(*a), r(*b), 0],

        Ld(a,b,off) => {
            if *off > u8::MAX as u16 {
                return Err(EncodeError::OffsetTooLarge { value: *off, op: "ld" });
            }
            [0x1a, r(*a), r(*b), *off as u8]
        }
        St(a,b,off) => {
            if *off > u8::MAX as u16 {
                return Err(EncodeError::OffsetTooLarge { value: *off, op: "st" });
            }
            [0x1b, r(*a), r(*b), *off as u8]
        }
        LdIdx(a,b,c) => [0x1c, r(*a), r(*b), r(*c)],
        StIdx(a,b,c) => [0x1d, r(*a), r(*b), r(*c)],
        Ref(a,b)     => [0x1e, r(*a), r(*b), 0],

        Alloc(a, sz) => { let o = sz.to_le_bytes(); [0x1f, r(*a), o[0], o[1]] }
        Drop(a)      => [0x20, r(*a), 0, 0],

        Dei(a,b) => [0x21, r(*a), r(*b), 0],
        Deo(a,b) => [0x22, r(*a), r(*b), 0],

        Handle(a, eid) => { let o = eid.to_le_bytes(); [0x23, r(*a), o[0], o[1]] }
        Resume(a,b)    => [0x24, r(*a), r(*b), 0],

        AddImm(a,b,imm) => [0x25, r(*a), r(*b), *imm as u8],
        SubImm(a,b,imm) => [0x26, r(*a), r(*b), *imm as u8],

        FAdd(a,b,c) => [0x27, r(*a), r(*b), r(*c)],
        FSub(a,b,c) => [0x28, r(*a), r(*b), r(*c)],
        FMul(a,b,c) => [0x29, r(*a), r(*b), r(*c)],
        FDiv(a,b,c) => [0x2a, r(*a), r(*b), r(*c)],
        FNeg(a,b)   => [0x2b, r(*a), r(*b), 0],
        FLt (a,b,c) => [0x2c, r(*a), r(*b), r(*c)],
        FEq (a,b,c) => [0x2d, r(*a), r(*b), r(*c)],
    })
}

fn decode_op(b: [u8; 4], offset: usize) -> Result<OpCode, LoadError> {
    use OpCode::*;
    let reg = Register;
    let imm16 = |b: [u8;4]| i16::from_le_bytes([b[2], b[3]]);
    let u16le = |b: [u8;4]| u16::from_le_bytes([b[2], b[3]]);
    Ok(match b[0] {
        0x00 => Add(reg(b[1]), reg(b[2]), reg(b[3])),
        0x01 => Sub(reg(b[1]), reg(b[2]), reg(b[3])),
        0x02 => Mul(reg(b[1]), reg(b[2]), reg(b[3])),
        0x03 => Div(reg(b[1]), reg(b[2]), reg(b[3])),
        0x04 => Mod(reg(b[1]), reg(b[2]), reg(b[3])),
        0x05 => Neg(reg(b[1]), reg(b[2])),
        0x06 => Eq (reg(b[1]), reg(b[2]), reg(b[3])),
        0x07 => Neq(reg(b[1]), reg(b[2]), reg(b[3])),
        0x08 => Lt (reg(b[1]), reg(b[2]), reg(b[3])),
        0x09 => Gt (reg(b[1]), reg(b[2]), reg(b[3])),
        0x0a => Lte(reg(b[1]), reg(b[2]), reg(b[3])),
        0x0b => Gte(reg(b[1]), reg(b[2]), reg(b[3])),
        0x0c => And(reg(b[1]), reg(b[2]), reg(b[3])),
        0x0d => Or (reg(b[1]), reg(b[2]), reg(b[3])),
        0x0e => Xor(reg(b[1]), reg(b[2]), reg(b[3])),
        0x0f => Shl(reg(b[1]), reg(b[2]), reg(b[3])),
        0x10 => Shr(reg(b[1]), reg(b[2]), reg(b[3])),
        0x11 => Jmp(imm16(b)),
        0x12 => Jz (reg(b[1]), imm16(b)),
        0x13 => Jnz(reg(b[1]), imm16(b)),
        0x14 => Call(reg(b[1]), u16le(b)),
        0x15 => Ret(reg(b[1])),
        0x16 => CallReg(reg(b[1]), reg(b[2])),
        0x17 => PushConst(reg(b[1]), u16le(b)),
        0x18 => Copy(reg(b[1]), reg(b[2])),
        0x19 => Move(reg(b[1]), reg(b[2])),
        0x1a => Ld (reg(b[1]), reg(b[2]), b[3] as u16),
        0x1b => St (reg(b[1]), reg(b[2]), b[3] as u16),
        0x1c => LdIdx(reg(b[1]), reg(b[2]), reg(b[3])),
        0x1d => StIdx(reg(b[1]), reg(b[2]), reg(b[3])),
        0x1e => Ref(reg(b[1]), reg(b[2])),
        0x1f => Alloc(reg(b[1]), u16le(b)),
        0x20 => Drop(reg(b[1])),
        0x21 => Dei(reg(b[1]), reg(b[2])),
        0x22 => Deo(reg(b[1]), reg(b[2])),
        0x23 => Handle(reg(b[1]), u16le(b)),
        0x24 => Resume(reg(b[1]), reg(b[2])),
        0x25 => AddImm(reg(b[1]), reg(b[2]), b[3] as i8),
        0x26 => SubImm(reg(b[1]), reg(b[2]), b[3] as i8),
        0x27 => FAdd(reg(b[1]), reg(b[2]), reg(b[3])),
        0x28 => FSub(reg(b[1]), reg(b[2]), reg(b[3])),
        0x29 => FMul(reg(b[1]), reg(b[2]), reg(b[3])),
        0x2a => FDiv(reg(b[1]), reg(b[2]), reg(b[3])),
        0x2b => FNeg(reg(b[1]), reg(b[2])),
        0x2c => FLt (reg(b[1]), reg(b[2]), reg(b[3])),
        0x2d => FEq (reg(b[1]), reg(b[2]), reg(b[3])),
        other => return Err(LoadError::Corrupt {
            offset,
            kind: Corruption::UnknownOpcode(other),
        }),
    })
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self { Self { data, pos: 0 } }

    fn take(&mut self, n: usize) -> Result<&'a [u8], LoadError> {
        if self.pos + n > self.data.len() {
            return Err(LoadError::Corrupt {
                offset: self.pos,
                kind: Corruption::Truncated,
            });
        }
        let s = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }

    fn read_u8 (&mut self) -> Result<u8,  LoadError> {
        let b = self.take(1)?; Ok(b[0])
    }
    fn read_u16(&mut self) -> Result<u16, LoadError> {
        let b = self.take(2)?; Ok(u16::from_le_bytes([b[0],b[1]]))
    }
    fn read_u32(&mut self) -> Result<u32, LoadError> {
        let b = self.take(4)?; Ok(u32::from_le_bytes([b[0],b[1],b[2],b[3]]))
    }
    fn read_u64(&mut self) -> Result<u64, LoadError> {
        let b = self.take(8)?;
        Ok(u64::from_le_bytes([b[0],b[1],b[2],b[3],b[4],b[5],b[6],b[7]]))
    }
}
