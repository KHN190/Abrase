#[cfg(all(target_os = "none", target_pointer_width = "32"))]
pub use crate::word32::{Word, Int, Float};
#[cfg(not(all(target_os = "none", target_pointer_width = "32")))]
pub use crate::word64::{Word, Int, Float};

pub const WORD_BITS: u32 = Word::BITS;
pub const WORD_BYTES: usize = core::mem::size_of::<Word>();

#[inline(always)]
pub fn decode_float(raw: Word) -> Float {
    Float::from_bits(raw)
}

#[inline(always)]
pub fn encode_float(value: Float) -> Word {
    value.to_bits()
}
