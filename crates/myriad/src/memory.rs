pub use super::core_heap::CoreHeap as Heap;
use super::{WORD_BITS, Word};

#[inline(always)]
pub fn handle_parts(raw: Word) -> (u32, u32) {
    super::Value::from_raw(raw).as_handle()
}

#[inline(always)]
pub fn make_handle(slot: u32, generation: u32) -> Word {
    super::Value::from_handle(slot, generation).raw()
}

#[inline]
pub fn mask_bit(mask: &[Word], i: usize) -> bool {
    let w = i / WORD_BITS as usize;
    let b = i % WORD_BITS as usize;
    mask.get(w).is_some_and(|x| (x >> b) & 1 == 1)
}

#[inline]
pub fn mask_set(mask: &mut [Word], i: usize, on: bool) {
    let w = i / WORD_BITS as usize;
    let b = i % WORD_BITS as usize;
    if let Some(x) = mask.get_mut(w) {
        if on {
            *x |= 1 << b;
        } else {
            *x &= !(1 << b);
        }
    }
}
