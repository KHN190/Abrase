use core::fmt;
use super::{Float, Int, Word, WORD_BYTES, HANDLE_SLOT_BITS, HANDLE_SLOT_MASK, HANDLE_GEN_MASK};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Value(pub Word);

pub const HANDLE_NONE: Word = Word::MAX;

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
    pub fn from_float(f: Float) -> Value { Value(super::encode_float(f, false)) }

    #[inline(always)]
    pub fn from_float_f32(f: Float) -> Value { Value(super::encode_float(f, true)) }

    #[inline(always)]
    pub fn from_bool(b: bool) -> Value { Value(if b { 1 } else { 0 }) }

    #[inline(always)]
    pub fn from_char(c: char) -> Value { Value(c as Word) }

    #[inline(always)]
    pub fn from_handle(slot: u32, generation: u32) -> Value {
        super::validate_handle_fields(slot, generation);
        let s = (slot as Word) & HANDLE_SLOT_MASK;
        let g = (generation as Word) & HANDLE_GEN_MASK;
        Value((s << HANDLE_SLOT_BITS) | g)
    }

    #[inline(always)]
    pub fn is_handle_none(self) -> bool { self.0 == HANDLE_NONE }

    #[inline(always)]
    pub fn as_int(self) -> Int { self.0 as Int }

    #[inline(always)]
    pub fn as_float(self) -> Float { super::decode_float(self.0, false) }

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
        write!(f, "Value({:#0width$x})", self.0, width = WORD_BYTES * 2 + 2)
    }
}
