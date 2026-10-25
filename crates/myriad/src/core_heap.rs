use super::{Int, WORD_BITS, WORD_BYTES, Word};
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use super::memory::{handle_parts, make_handle};
use super::{heap_backend as backend, heap_core as cgen};

pub(super) const GLOBAL_HDR: usize = 32;
pub(super) const BLK_HDR: usize = 24;
const HDR_SIZE: usize = 8;
const HDR_FRONTIER: usize = 0;
const MAX_ARENA_BYTES: usize = 1 << 28;
pub struct CoreHeap {
    arena: Vec<Word>,
    pub trace_pc: usize,
    trace_slot: Option<u32>,
    trace_all: bool,
    trace_out: Option<fn(&str)>,
    alloc_count: Word,
}

fn mask_words(size: usize) -> usize {
    size.div_ceil(WORD_BITS as usize)
}

impl CoreHeap {
    pub fn new() -> Self {
        Self::with_capacity(1 << 16)
    }

    pub fn with_capacity(bytes: usize) -> Self {
        Self::try_with_capacity(bytes).expect("invalid heap capacity")
    }

    pub fn try_with_capacity(bytes: usize) -> Result<Self, String> {
        let (bytes, slots) = backend::default_layout(bytes);
        Self::try_with_capacity_and_slots(bytes, slots)
    }

    /// The 32-bit table consumes `(slots + 1) * 8` arena bytes and never grows.
    /// The 64-bit direct-handle backend accepts zero slots.
    pub fn try_with_capacity_and_slots(bytes: usize, slots: usize) -> Result<Self, String> {
        backend::validate_layout(bytes, slots)?;
        let mut heap = Self::reserve_arena(bytes)?;
        backend::init(heap.bytes_mut(), slots)?;
        Ok(heap)
    }

    fn reserve_arena(bytes: usize) -> Result<Self, String> {
        let words = bytes.div_ceil(WORD_BYTES);
        let rounded = words
            .checked_mul(WORD_BYTES)
            .ok_or_else(|| String::from("heap: arena size overflow"))?;
        if rounded > MAX_ARENA_BYTES {
            return Err("heap: arena capacity exceeds limit".into());
        }
        let mut arena = Vec::new();
        arena
            .try_reserve_exact(words)
            .map_err(|_| String::from("heap: arena allocation failed"))?;
        arena.resize(words, 0);
        Ok(Self {
            arena,
            trace_pc: 0,
            trace_slot: None,
            trace_all: false,
            trace_out: None,
            alloc_count: 0,
        })
    }

    fn blen(&self) -> usize {
        self.arena.len() * WORD_BYTES
    }

    fn bytes(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.arena.as_ptr() as *const u8, self.blen()) }
    }
    pub(super) fn bytes_mut(&mut self) -> &mut [u8] {
        let n = self.blen();
        unsafe { core::slice::from_raw_parts_mut(self.arena.as_mut_ptr() as *mut u8, n) }
    }
    fn r32(&self, addr: usize) -> u32 {
        read_u32(self.bytes(), addr)
    }
    fn rword(&self, addr: usize) -> Word {
        read_word(self.bytes(), addr)
    }
    fn w_bytes(&mut self, addr: usize, src: &[u8]) {
        self.bytes_mut()[addr..addr + src.len()].copy_from_slice(src);
    }
    fn block_size(&self, off: usize) -> usize {
        self.r32(off + 8) as usize
    }
    fn data_start(&self, off: usize, size: usize) -> usize {
        off + BLK_HDR + mask_words(size) * WORD_BYTES
    }

    pub(super) fn valid(&self, slot: u32, generation: u32, op: &str) -> Result<usize, String> {
        backend::resolve(self.bytes(), slot, generation, op)
    }

    fn grow(&mut self) -> Result<(), String> {
        let bytes = self
            .blen()
            .checked_mul(2)
            .filter(|n| *n <= MAX_ARENA_BYTES)
            .ok_or_else(|| String::from("heap: arena exhausted"))?;
        let words = bytes / WORD_BYTES;
        self.arena
            .try_reserve_exact(words - self.arena.len())
            .map_err(|_| String::from("heap: arena allocation failed"))?;
        self.arena.resize(words, 0);
        self.w_bytes(HDR_SIZE, &(bytes as Word).to_le_bytes());
        Ok(())
    }

    pub fn try_alloc(&mut self, size: usize) -> Result<(u32, u32), String> {
        if size > 268435456 {
            return Err("heap: invalid allocation size".into());
        }
        loop {
            if !backend::binding_available(self.bytes()) {
                return Err("heap: handle slots exhausted".into());
            }
            let h = cgen::alloc(self.bytes_mut(), size as Word).map_err(String::from)?;
            if h != 0 {
                let (slot, generation) = handle_parts(h);
                if make_handle(slot, generation) != h {
                    return Err("alloc: handle exceeds slot capacity".into());
                }
                self.emit_trace("alloc", slot, generation);
                self.alloc_count = self.alloc_count.wrapping_add(1);
                return Ok((slot, generation));
            }
            self.grow()?;
        }
    }
    pub fn alloc(&mut self, size: usize) -> (u32, u32) {
        self.try_alloc(size).expect("alloc")
    }

    pub fn try_alloc_with_mask(
        &mut self,
        size: usize,
        init_mask: &[Word],
    ) -> Result<(u32, u32), String> {
        let (slot, g) = self.try_alloc(size)?;
        let off = self.valid(slot, g, "alloc_with_mask")?;
        for w in 0..mask_words(size).min(init_mask.len()) {
            self.w_bytes(off + BLK_HDR + w * WORD_BYTES, &init_mask[w].to_le_bytes());
        }
        Ok((slot, g))
    }
    pub fn alloc_with_mask(&mut self, size: usize, init_mask: &[Word]) -> (u32, u32) {
        self.try_alloc_with_mask(size, init_mask)
            .expect("alloc_with_mask")
    }

    pub fn ld(&self, slot: u32, generation: u32, offset: usize) -> Result<(Word, bool), String> {
        let off = self.valid(slot, generation, "ld")?;
        let size = self.block_size(off);
        if offset >= size {
            return Err(format!(
                "ld: offset {} out of bounds (size {})",
                offset, size
            ));
        }
        let val = self.rword(self.data_start(off, size) + offset * WORD_BYTES);
        let mw = self.rword(off + BLK_HDR + (offset / WORD_BITS as usize) * WORD_BYTES);
        Ok((val, (mw >> (offset % WORD_BITS as usize)) & 1 == 1))
    }

    pub fn st(
        &mut self,
        slot: u32,
        generation: u32,
        offset: usize,
        val: Word,
        is_handle: bool,
    ) -> Result<(Word, bool), String> {
        let off = self.valid(slot, generation, "st")?;
        let size = self.block_size(off);
        if offset >= size {
            return Err(format!(
                "st: offset {} out of bounds (size {})",
                offset, size
            ));
        }
        let waddr = self.data_start(off, size) + offset * WORD_BYTES;
        let old = self.rword(waddr);
        let maddr = off + BLK_HDR + (offset / WORD_BITS as usize) * WORD_BYTES;
        let mword = self.rword(maddr);
        let bit = (1 as Word) << (offset % WORD_BITS as usize);
        let old_is_handle = mword & bit != 0;
        self.w_bytes(waddr, &val.to_le_bytes());
        let newm = if is_handle { mword | bit } else { mword & !bit };
        self.w_bytes(maddr, &newm.to_le_bytes());
        Ok((old, old_is_handle))
    }

    fn data_words(&self, off: usize, size: usize) -> &[Word] {
        let w = self.data_start(off, size) / WORD_BYTES;
        &self.arena[w..w + size]
    }
    pub fn cell_data(&self, slot: u32, generation: u32) -> Result<&[Word], String> {
        let off = self.valid(slot, generation, "cell")?;
        Ok(self.data_words(off, self.block_size(off)))
    }
    pub fn cell_data_mut(&mut self, slot: u32, generation: u32) -> Result<&mut [Word], String> {
        let off = self.valid(slot, generation, "cell_mut")?;
        let size = self.block_size(off);
        let w = self.data_start(off, size) / WORD_BYTES;
        Ok(&mut self.arena[w..w + size])
    }
    pub fn cell_mask(&self, slot: u32, generation: u32) -> Result<&[Word], String> {
        let off = self.valid(slot, generation, "cell_mask")?;
        let size = self.block_size(off);
        let w = (off + BLK_HDR) / WORD_BYTES;
        Ok(&self.arena[w..w + mask_words(size)])
    }
    pub fn size(&self, slot: u32, generation: u32) -> Result<usize, String> {
        let off = self.valid(slot, generation, "size")?;
        Ok(self.block_size(off))
    }
    pub fn is_live(&self, slot: u32, generation: u32) -> bool {
        self.valid(slot, generation, "is_live").is_ok()
    }

    pub fn rc_inc_handle(&mut self, raw: Word) -> Result<(), String> {
        if raw == super::HANDLE_NONE {
            return Ok(());
        }
        let (s, g) = handle_parts(raw);
        self.rc_inc(s, g)
    }
    pub fn rc_dec_handle(&mut self, raw: Word) -> Result<(), String> {
        if raw == super::HANDLE_NONE {
            return Ok(());
        }
        let (s, g) = handle_parts(raw);
        self.rc_dec(s, g)?;
        Ok(())
    }
    pub fn rc_inc(&mut self, slot: u32, generation: u32) -> Result<(), String> {
        self.valid(slot, generation, "rc_inc")?;
        let r =
            cgen::rc_inc(self.bytes_mut(), make_handle(slot, generation)).map_err(String::from)?;
        if r != 0 {
            return Err(format!("rc_inc: stale slot {}", slot));
        }
        self.emit_trace("inc", slot, generation);
        Ok(())
    }
    pub fn rc_dec(&mut self, slot: u32, generation: u32) -> Result<bool, String> {
        self.valid(slot, generation, "rc_dec")?;
        let r =
            cgen::rc_dec(self.bytes_mut(), make_handle(slot, generation)).map_err(String::from)?;
        self.emit_trace("dec", slot, generation);
        Ok(r == 1)
    }
    pub fn force_free(&mut self, slot: u32, generation: u32) -> Result<(), String> {
        if !self.is_live(slot, generation) {
            return Ok(());
        }
        self.emit_trace("force_free", slot, generation);
        cgen::force_free(self.bytes_mut(), make_handle(slot, generation)).map_err(String::from)?;
        Ok(())
    }

    fn scan_live<F: FnMut(u32, u32, usize)>(&self, mut f: F) {
        let mut cursor = 0;
        while let Some((slot, generation, off)) = backend::next_live(self.bytes(), &mut cursor) {
            f(slot, generation, off);
        }
    }
    pub fn live_count(&self) -> usize {
        let mut n = 0;
        self.scan_live(|_, _, _| n += 1);
        n
    }
    pub fn alloc_count(&self) -> Word {
        self.alloc_count
    }
    pub fn rc(&self, slot: u32, generation: u32) -> Option<u32> {
        self.valid(slot, generation, "rc")
            .ok()
            .map(|off| self.r32(off))
    }
    pub fn live_cells(&self) -> Vec<(u32, u32, u32, Vec<Word>, Vec<bool>)> {
        let mut out = Vec::new();
        self.scan_live(|slot, generation, off| {
            let size = self.block_size(off);
            let data = self.data_words(off, size).to_vec();
            let mask = (0..size)
                .map(|i| {
                    let mw = self.rword(off + BLK_HDR + (i / WORD_BITS as usize) * WORD_BYTES);
                    (mw >> (i % WORD_BITS as usize)) & 1 == 1
                })
                .collect();
            out.push((slot, generation, self.r32(off), data, mask));
        });
        out
    }

    pub fn clear(&mut self) {
        if backend::PRESERVE_GENERATIONS {
            let mut cursor = 0;
            while let Some((slot, generation, _)) = backend::next_live(self.bytes(), &mut cursor) {
                self.force_free(slot, generation)
                    .expect("clear live binding");
            }
        } else {
            self.arena.fill(0);
            backend::init(self.bytes_mut(), 0).expect("core_init");
        }
    }

    /// Conservative arena high-water usage, including the word32 table and retired slots.
    /// Excludes the global header, allocator bookkeeping, and other VM state.
    pub fn bytes_used(&self) -> usize {
        self.rword(HDR_FRONTIER) as usize - GLOBAL_HDR
    }

    pub fn set_trace(&mut self, slot: Option<u32>, all: bool, out: fn(&str)) {
        self.trace_slot = slot;
        self.trace_all = all;
        self.trace_out = Some(out);
    }
    fn emit_trace(&self, op: &str, slot: u32, generation: u32) {
        let Some(out) = self.trace_out else {
            return;
        };
        if !self.trace_all && self.trace_slot != Some(slot) {
            return;
        }
        let rc = self.rc(slot, generation).map(|r| r as Int).unwrap_or(-1);
        out(&format!(
            "[rc] {} slot {} gen {} -> rc {} @pc {}",
            op, slot, generation, rc, self.trace_pc
        ));
    }
}

pub(super) fn read_u32(arena: &[u8], addr: usize) -> u32 {
    u32::from_le_bytes(arena[addr..addr + 4].try_into().unwrap())
}

pub(super) fn read_word(arena: &[u8], addr: usize) -> Word {
    Word::from_le_bytes(arena[addr..addr + WORD_BYTES].try_into().unwrap())
}
