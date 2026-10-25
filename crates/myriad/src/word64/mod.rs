pub use polka::word64::{Word, Int, Float, WORD_BITS, WORD_BYTES, Value, HANDLE_NONE, HANDLE_SLOT_MAX};

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
#[path = "../heap_backend64.rs"]
pub(crate) mod heap_backend;
#[path = "../core_heap_gen.rs"]
pub(crate) mod heap_core;

pub use memory::Heap;
pub use core_mem::CoreArena;
pub use value::{alloc_string, read_string, alloc_bytes, read_bytes};
pub use region::RegionTable;

pub(crate) use crate::word64 as runtime;
pub const IS_INT32: bool = false;
pub const FRAME_MASK_WORDS: usize = 2;
pub const DISPATCH_TAIL_FLAG: Word = 1 << 16;
pub type TraceMask = u128;
pub fn empty_trace_mask() -> TraceMask { 0 }
pub fn set_trace_mask(mask: &mut TraceMask, index: usize) { *mask |= 1u128 << index; }
pub(crate) use hashbrown::{HashMap as ProfileMap, HashSet as RuntimeSet};
pub use polka::word64::{decode_float, encode_float};
pub(crate) fn decode_constant(raw: polka::Constant) -> Result<Word, &'static str> { Ok(Word::from_le_bytes(raw.to_le_bytes())) }

pub mod fmath {
    pub fn sqrt(x: super::Float) -> super::Float { libm::sqrt(x) }
    pub fn sin(x: super::Float) -> super::Float { libm::sin(x) }
    pub fn cos(x: super::Float) -> super::Float { libm::cos(x) }
    pub fn floor(x: super::Float) -> super::Float { libm::floor(x) }
    pub fn ceil(x: super::Float) -> super::Float { libm::ceil(x) }
    pub fn abs(x: super::Float) -> super::Float { libm::fabs(x) }
    pub fn fmax(a: super::Float, b: super::Float) -> super::Float { libm::fmax(a, b) }
    pub fn fmin(a: super::Float, b: super::Float) -> super::Float { libm::fmin(a, b) }
}

include!("../vm_shared.rs");
