pub type Word = u64;
pub type Int = i64;
pub type Float = f64;
pub const WORD_BITS: u32 = 64;
pub const WORD_BYTES: usize = 8;
const HANDLE_SLOT_BITS: u32 = 24;
const HANDLE_SLOT_MASK: Word = u32::MAX as Word;
const HANDLE_GEN_MASK: Word = (1 << HANDLE_SLOT_BITS) - 1;
pub const HANDLE_SLOT_MAX: u32 = u32::MAX - 1;

#[path = "value_impl.rs"]
mod value;
pub use value::{Value, HANDLE_NONE};

fn validate_handle_fields(_slot: u32, _generation: u32) {}

#[inline(always)]
pub fn decode_float(raw: Word, int32: bool) -> Float {
    if int32 { f32::from_bits(raw as u32) as Float } else { Float::from_bits(raw) }
}
#[inline(always)]
pub fn encode_float(value: Float, int32: bool) -> Word {
    if int32 { (value as f32).to_bits() as Word } else { value.to_bits() }
}
