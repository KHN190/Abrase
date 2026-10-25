use core::fmt;
use crate::numeric::{Float, Int, Word};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Value(pub Word);

pub const HANDLE_NONE: Word = Word::MAX;

#[cfg(feature = "word32")]
const HANDLE_SLOT_BITS: u32 = 16;
#[cfg(not(feature = "word32"))]
const HANDLE_SLOT_BITS: u32 = 24;
#[cfg(feature = "word32")]
const HANDLE_SLOT_MASK: Word = u16::MAX as Word;
#[cfg(not(feature = "word32"))]
const HANDLE_SLOT_MASK: Word = u32::MAX as Word;
const HANDLE_GEN_MASK: Word = (1 << HANDLE_SLOT_BITS) - 1;
#[cfg(feature = "word32")]
pub const HANDLE_SLOT_MAX: u32 = u16::MAX as u32 - 1;
#[cfg(not(feature = "word32"))]
pub const HANDLE_SLOT_MAX: u32 = u32::MAX - 1;

impl Value {
    pub const ZERO: Value = Value(0);
    pub const NONE: Value = Value(HANDLE_NONE);
    pub const UNIT:  Value = Value(0);
    pub const FALSE: Value = Value(0);
    pub const TRUE:  Value = Value(1);

    #[inline(always)]
    pub fn raw(self) -> Word { self.0 }

    #[inline(always)]
    pub fn from_raw(n: Word) -> Value { Value(n) }

    #[inline(always)]
    pub fn from_int(n: Int) -> Value { Value(n as Word) }

    #[inline(always)]
    pub fn from_float(f: Float) -> Value { Value(encode_float(f, false)) }

    #[inline(always)]
    pub fn from_float_f32(f: Float) -> Value { Value(encode_float(f, true)) }

    #[inline(always)]
    pub fn from_bool(b: bool) -> Value { Value(if b { 1 } else { 0 }) }

    #[inline(always)]
    pub fn from_char(c: char) -> Value { Value(c as Word) }

    #[inline(always)]
    pub fn from_handle(slot: u32, generation: u32) -> Value {
        #[cfg(feature = "word32")]
        assert!(slot <= HANDLE_SLOT_MAX && generation <= u16::MAX as u32, "handle exceeds 32-bit layout");
        let s = (slot as Word) & HANDLE_SLOT_MASK;
        let g = (generation as Word) & HANDLE_GEN_MASK;
        Value((s << HANDLE_SLOT_BITS) | g)
    }

    #[inline(always)]
    pub fn is_handle_none(self) -> bool { self.0 == HANDLE_NONE }

    #[inline(always)]
    pub fn as_int(self) -> Int { self.0 as Int }

    #[inline(always)]
    pub fn as_float(self) -> Float { decode_float(self.0, false) }

    #[inline(always)]
    pub fn as_bool(self) -> bool { self.0 != 0 }

    #[inline(always)]
    pub fn as_char(self) -> Option<char> { char::from_u32(self.0 as u32) }

    #[inline(always)]
    pub fn as_handle(self) -> (u32, u32) {
        let slot = ((self.0 >> HANDLE_SLOT_BITS) & HANDLE_SLOT_MASK) as u32;
        let gen_ = (self.0 & HANDLE_GEN_MASK) as u32;
        (slot, gen_)
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_handle_none() { return write!(f, "None"); }
        write!(f, "Value({:#0width$x})", self.0, width = crate::numeric::WORD_BYTES * 2 + 2)
    }
}


#[cfg(not(feature = "word32"))]
#[inline(always)]
pub fn decode_float(raw: Word, int32: bool) -> Float {
    if int32 { f32::from_bits(raw as u32) as Float } else { Float::from_bits(raw) }
}

#[cfg(not(feature = "word32"))]
#[inline(always)]
pub fn encode_float(f: Float, int32: bool) -> Word {
    if int32 { (f as f32).to_bits() as Word } else { f.to_bits() }
}

#[cfg(feature = "word32")]
#[inline(always)]
pub fn decode_float(raw: Word, _int32: bool) -> Float {
    crate::numeric::decode_float(raw)
}

#[cfg(feature = "word32")]
#[inline(always)]
pub fn encode_float(f: Float, _int32: bool) -> Word {
    crate::numeric::encode_float(f)
}
