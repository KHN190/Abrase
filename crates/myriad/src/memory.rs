// `Heap` is now backed by the abrase-written trust-archive core (vendored as
// `core_heap_gen.rs`, AOT-compiled to no_std Rust) — see `core_heap.rs`. This
// module keeps only the shared handle/mask helpers consumed across the VM.
pub use crate::core_heap::CoreHeap as Heap;

// slot = byte offset; arena grows past 16MB so slot needs >24 bits (gen stays 24).
#[inline(always)]
pub fn handle_parts(raw: u64) -> (u32, u32) {
    (((raw >> 24) & 0xFFFF_FFFF) as u32, (raw & 0x00FF_FFFF) as u32)
}

#[inline(always)]
pub fn make_handle(slot: u32, generation: u32) -> u64 {
    ((slot as u64) << 24) | (generation as u64 & 0x00FF_FFFF)
}

#[inline]
pub fn mask_bit(mask: &[u64], i: usize) -> bool {
    let w = i / 64;
    let b = i % 64;
    mask.get(w).map_or(false, |x| (x >> b) & 1 == 1)
}

#[inline]
pub fn mask_set(mask: &mut [u64], i: usize, on: bool) {
    let w = i / 64;
    let b = i % 64;
    if let Some(x) = mask.get_mut(w) {
        if on { *x |= 1u64 << b; } else { *x &= !(1u64 << b); }
    }
}

