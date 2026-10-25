#[cfg(feature = "word32")]
pub type Word = u32;
#[cfg(not(feature = "word32"))]
pub type Word = u64;

#[cfg(feature = "word32")]
pub type Int = i32;
#[cfg(not(feature = "word32"))]
pub type Int = i64;

#[cfg(feature = "word32")]
pub type Float = f32;
#[cfg(not(feature = "word32"))]
pub type Float = f64;

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
