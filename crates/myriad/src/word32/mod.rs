pub use polka::word32::{Word, Int, Float, WORD_BITS, WORD_BYTES, Value, HANDLE_NONE, HANDLE_SLOT_MAX};

#[path = "../memory.rs"]
pub mod memory;
#[path = "../core_heap.rs"]
pub mod core_heap;
#[path = "../core.rs"]
pub mod core_mem;
#[path = "../value.rs"]
pub mod value;
#[path = "../region.rs"]
pub mod region;
#[path = "../heap_backend32.rs"]
pub(crate) mod heap_backend;
#[path = "../core_heap32_gen.rs"]
pub(crate) mod heap_core;

pub use memory::Heap;
pub use core_mem::CoreArena;
pub use value::{alloc_string, read_string, alloc_bytes, read_bytes};
pub use region::RegionTable;

pub(crate) use crate::word32 as runtime;
pub const IS_INT32: bool = true;
pub fn format_float(value: Float) -> alloc::string::String { crate::formatter32::format_float(value) }
pub(crate) fn format_diagnostic_float(value: Float, _legacy_precision: usize) -> alloc::string::String { format_float(value) }
pub const FRAME_MASK_WORDS: usize = 4;
pub const DISPATCH_TAIL_FLAG: Word = 1 << 16;
pub type TraceMask = [Word; FRAME_MASK_WORDS];
pub fn empty_trace_mask() -> TraceMask { [0; FRAME_MASK_WORDS] }
pub fn set_trace_mask(mask: &mut TraceMask, index: usize) { mask[index / 32] |= 1 << (index % 32); }
pub(crate) use alloc::collections::{BTreeMap as ProfileMap, BTreeSet as RuntimeSet};
pub use polka::word32::{decode_float, encode_float};
pub(crate) fn decode_constant(raw: polka::Constant) -> Result<Word, &'static str> { polka::constant_word32(raw) }

pub mod fmath {
    pub fn sqrt(x: super::Float) -> super::Float { crate::math32::sqrt(x) }
    pub fn sin(x: super::Float) -> super::Float { crate::math32::sin(x) }
    pub fn cos(x: super::Float) -> super::Float { crate::math32::cos(x) }
    pub fn floor(x: super::Float) -> super::Float { libm::floorf(x) }
    pub fn ceil(x: super::Float) -> super::Float { libm::ceilf(x) }
    pub fn abs(x: super::Float) -> super::Float { libm::fabsf(x) }
    pub fn fmax(a: super::Float, b: super::Float) -> super::Float { libm::fmaxf(a, b) }
    pub fn fmin(a: super::Float, b: super::Float) -> super::Float { libm::fminf(a, b) }
}

include!("../vm_shared.rs");
