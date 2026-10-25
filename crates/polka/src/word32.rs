pub type Word = u32;
pub type Int = i32;
pub type Float = f32;
pub const WORD_BITS: u32 = 32;
pub const WORD_BYTES: usize = 4;
const HANDLE_SLOT_BITS: u32 = 16;
const HANDLE_SLOT_MASK: Word = u16::MAX as Word;
const HANDLE_GEN_MASK: Word = u16::MAX as Word;
pub const HANDLE_SLOT_MAX: u32 = u16::MAX as u32 - 1;

#[path = "value_impl.rs"]
mod value;
pub use value::{Value, HANDLE_NONE};

fn validate_handle_fields(slot: u32, generation: u32) {
    assert!(slot <= HANDLE_SLOT_MAX && generation <= u16::MAX as u32, "handle exceeds 32-bit layout");
}

#[inline(always)]
pub fn decode_float(raw: Word, _int32: bool) -> Float { Float::from_bits(raw) }
#[inline(always)]
pub fn encode_float(value: Float, _int32: bool) -> Word { value.to_bits() }
