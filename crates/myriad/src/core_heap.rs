use alloc::vec;
use alloc::vec::Vec;

#[path = "core_heap_gen.rs"]
mod cgen;

// Thin wrapper over the vendored trust-archive heap (the abrase-written
// alloc/RC/region, AOT-compiled to no_std Rust). Operates on a flat byte arena;
// handles are the core's `(offset<<24)|gen` encoding.
pub struct CoreHeap {
    arena: Vec<u8>,
}

impl CoreHeap {
    pub fn new(bytes: usize) -> Result<Self, &'static str> {
        let mut arena = vec![0u8; bytes];
        cgen::core_init(&mut arena, bytes as u64)?;
        Ok(Self { arena })
    }

    pub fn alloc(&mut self, size: u64) -> Result<u64, &'static str> {
        cgen::alloc(&mut self.arena, size)
    }
    pub fn rc_inc(&mut self, h: u64) -> Result<u64, &'static str> {
        cgen::rc_inc(&mut self.arena, h)
    }
    pub fn rc_dec(&mut self, h: u64) -> Result<u64, &'static str> {
        cgen::rc_dec(&mut self.arena, h)
    }
    pub fn cell_get(&mut self, h: u64, i: u64) -> Result<u64, &'static str> {
        cgen::cell_get(&mut self.arena, h, i)
    }
    pub fn cell_set(&mut self, h: u64, i: u64, v: u64) -> Result<u64, &'static str> {
        cgen::cell_set(&mut self.arena, h, i, v)
    }
    pub fn cell_set_child(&mut self, h: u64, i: u64, c: u64) -> Result<u64, &'static str> {
        cgen::cell_set_child(&mut self.arena, h, i, c)
    }

    pub fn rc_of(&self, handle: u64) -> u32 {
        let off = (handle >> 24) as usize;
        u32::from_le_bytes([self.arena[off], self.arena[off + 1], self.arena[off + 2], self.arena[off + 3]])
    }
}

#[cfg(test)]
mod tests {
    use super::CoreHeap;
    use crate::Heap;
    use alloc::vec::Vec;

    fn xorshift(s: &mut u64) -> u64 {
        *s ^= *s << 13;
        *s ^= *s >> 7;
        *s ^= *s << 17;
        *s
    }

    // The vendored trust-archive heap must track RC and freeing identically to
    // the hand-written Rust Heap over a random alloc/inc/dec sequence.
    #[test]
    fn vendored_core_matches_rust_heap() {
        let mut core = CoreHeap::new(1 << 20).unwrap();
        let mut heap = Heap::new();
        struct H { core: u64, slot: u32, g: u32, rc: i64, alive: bool }
        let mut hs: Vec<H> = Vec::new();
        let mut rng: u64 = 0x9e37_79b9_7f4a_7c15;

        for _ in 0..3000 {
            let live: Vec<usize> = (0..hs.len()).filter(|&i| hs[i].alive).collect();
            let pick = if live.is_empty() { 0 } else { xorshift(&mut rng) % 3 };
            if pick == 0 {
                let size = (xorshift(&mut rng) % 4 + 1) as u64;
                let c = core.alloc(size).unwrap();
                assert_ne!(c, 0, "arena OOM");
                let (slot, g) = heap.alloc(size as usize);
                assert_eq!(core.rc_of(c), 1);
                assert_eq!(heap.rc(slot, g), Some(1));
                hs.push(H { core: c, slot, g, rc: 1, alive: true });
            } else {
                let idx = live[(xorshift(&mut rng) as usize) % live.len()];
                if pick == 1 {
                    assert_eq!(core.rc_inc(hs[idx].core).unwrap(), 0);
                    heap.rc_inc(hs[idx].slot, hs[idx].g).unwrap();
                    hs[idx].rc += 1;
                } else {
                    let cf = core.rc_dec(hs[idx].core).unwrap();
                    let rf = heap.rc_dec(hs[idx].slot, hs[idx].g).unwrap();
                    hs[idx].rc -= 1;
                    let freed = hs[idx].rc == 0;
                    assert_eq!(cf == 1, freed, "core freed-flag");
                    assert_eq!(rf, freed, "rust freed-flag");
                    if freed { hs[idx].alive = false; }
                }
                if hs[idx].alive {
                    assert_eq!(core.rc_of(hs[idx].core) as i64, hs[idx].rc, "core rc");
                    assert_eq!(heap.rc(hs[idx].slot, hs[idx].g).map(|x| x as i64), Some(hs[idx].rc), "rust rc");
                }
            }
        }
        for h in hs.iter_mut().filter(|h| h.alive) {
            while h.rc > 0 {
                core.rc_dec(h.core).unwrap();
                heap.rc_dec(h.slot, h.g).unwrap();
                h.rc -= 1;
            }
        }
        assert_eq!(heap.live_count(), 0, "rust heap reclaimed");
    }
}
