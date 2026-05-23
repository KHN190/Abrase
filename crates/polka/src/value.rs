use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Value(pub u64);

pub const HANDLE_NONE: u64 = u64::MAX;

const HANDLE_SLOT_BITS: u32 = 24;
const HANDLE_SLOT_MASK: u64 = (1u64 << HANDLE_SLOT_BITS) - 1;
const HANDLE_GEN_MASK:  u64 = (1u64 << HANDLE_SLOT_BITS) - 1;
pub const HANDLE_SLOT_MAX: u32 = (1u32 << HANDLE_SLOT_BITS) - 2;

impl Value {
    pub const ZERO: Value = Value(0);
    pub const NONE: Value = Value(HANDLE_NONE);
    pub const UNIT:  Value = Value(0);
    pub const FALSE: Value = Value(0);
    pub const TRUE:  Value = Value(1);

    #[inline(always)]
    pub fn raw(self) -> u64 { self.0 }

    #[inline(always)]
    pub fn from_raw(n: u64) -> Value { Value(n) }

    #[inline(always)]
    pub fn from_int(n: i64) -> Value { Value(n as u64) }

    #[inline(always)]
    pub fn from_float(f: f64) -> Value { Value(f.to_bits()) }

    #[inline(always)]
    pub fn from_float_f32(f: f64) -> Value { Value((f as f32).to_bits() as u64) }

    #[inline(always)]
    pub fn from_bool(b: bool) -> Value { Value(if b { 1 } else { 0 }) }

    #[inline(always)]
    pub fn from_char(c: char) -> Value { Value(c as u64) }

    #[inline(always)]
    pub fn from_handle(slot: u32, generation: u32) -> Value {
        let s = (slot as u64) & HANDLE_SLOT_MASK;
        let g = (generation as u64) & HANDLE_GEN_MASK;
        Value((s << HANDLE_SLOT_BITS) | g)
    }

    #[inline(always)]
    pub fn is_handle_none(self) -> bool { self.0 == HANDLE_NONE }

    #[inline(always)]
    pub fn as_int(self) -> i64 { self.0 as i64 }

    #[inline(always)]
    pub fn as_float(self) -> f64 { f64::from_bits(self.0) }

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
        write!(f, "Value({:#018x})", self.0)
    }
}

