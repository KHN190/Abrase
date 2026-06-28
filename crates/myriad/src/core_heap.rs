use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use crate::memory::{handle_parts, make_handle};

#[path = "core_heap_gen.rs"]
mod cgen;

// Drop-in replacement for `Heap`, backed by the vendored trust-archive heap (the
// abrase-written alloc/RC/region, AOT-compiled to no_std Rust). Handles use the
// core's `(offset<<24)|gen` encoding, identical to `make_handle`, so `slot` ==
// byte offset. The arena is a `Vec<u64>` so its base is u64-aligned by the type
// system; cell data/mask views are then plain safe slices. The core fns work on
// bytes, reached through a `&[u8]`/`&mut [u8]` reinterpretation of the words
// (always sound: u8 has alignment 1 and the length is exact). The arena grows on
// OOM — offsets are arena-relative, so moving the backing Vec is safe.
const GLOBAL_HDR: u64 = 32;
const BLK_HDR: u64 = 24;
const HDR_SIZE: u64 = 8;
const HDR_FRONTIER: u64 = 0;

pub struct CoreHeap {
    arena: Vec<u64>,
    pub trace_pc: usize,
}

fn mask_words(size: u64) -> u64 { (size + 63) / 64 }

impl CoreHeap {
    pub fn new() -> Self {
        Self::with_capacity(1 << 16)
    }

    pub fn with_capacity(bytes: usize) -> Self {
        let words = (bytes + 7) / 8;
        let mut h = Self { arena: vec![0u64; words], trace_pc: 0 };
        debug_assert!(h.arena.as_ptr() as usize & 7 == 0, "Vec<u64> base must be 8-aligned");
        let blen = h.blen();
        cgen::core_init(h.bytes_mut(), blen).expect("core_init");
        h
    }

    fn blen(&self) -> u64 { (self.arena.len() * 8) as u64 }

    fn bytes(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.arena.as_ptr() as *const u8, self.arena.len() * 8) }
    }
    fn bytes_mut(&mut self) -> &mut [u8] {
        let n = self.arena.len() * 8;
        unsafe { core::slice::from_raw_parts_mut(self.arena.as_mut_ptr() as *mut u8, n) }
    }

    fn r32(&self, addr: u64) -> u32 {
        let b = self.bytes();
        let a = addr as usize;
        u32::from_le_bytes([b[a], b[a + 1], b[a + 2], b[a + 3]])
    }
    fn r64(&self, addr: u64) -> u64 {
        let b = self.bytes();
        let a = addr as usize;
        let mut x = [0u8; 8];
        x.copy_from_slice(&b[a..a + 8]);
        u64::from_le_bytes(x)
    }
    fn w_bytes(&mut self, addr: u64, src: &[u8]) {
        let a = addr as usize;
        self.bytes_mut()[a..a + src.len()].copy_from_slice(src);
    }

    fn block_size(&self, off: u64) -> u64 { self.r32(off + 8) as u64 }
    fn data_start(&self, off: u64, size: u64) -> u64 { off + BLK_HDR + mask_words(size) * 8 }

    fn valid(&self, slot: u32, generation: u32, op: &str) -> Result<u64, String> {
        let off = slot as u64;
        let frontier = self.r64(HDR_FRONTIER);
        if off < GLOBAL_HDR || off + BLK_HDR > self.blen() {
            let why = if off >= frontier { "past frontier — overflowed/truncated handle" } else { "below header" };
            return Err(format!("{}: slot {} off {:#x} outside arena [{:#x},{:#x}) ({})",
                op, slot, off, GLOBAL_HDR, self.blen(), why));
        }
        if self.r32(off) == 0 {
            let why = if off >= frontier { "never allocated — overflowed/truncated handle" } else { "freed or mid-block" };
            return Err(format!("{}: use-after-free slot {} off {:#x} rc=0 ({}); frontier {:#x}",
                op, slot, off, why, frontier));
        }
        if (self.r32(off + 4) & 0x00FF_FFFF) != generation {
            return Err(format!("{}: stale handle for slot {} off {:#x} (have gen {}, live {})",
                op, slot, off, generation, self.r32(off + 4) & 0x00FF_FFFF));
        }
        Ok(off)
    }

    fn grow(&mut self) {
        let n = self.arena.len();
        self.arena.resize(n * 2, 0);
        let blen = self.blen();
        self.w_bytes(HDR_SIZE, &blen.to_le_bytes());
    }

    pub fn try_alloc(&mut self, size: usize) -> Result<(u32, u32), String> {
        loop {
            let h = cgen::alloc(self.bytes_mut(), size as u64).map_err(String::from)?;
            if h != 0 {
                let (slot, generation) = handle_parts(h);
                if make_handle(slot, generation) != h {
                    return Err(format!("alloc: arena offset {:#x} exceeds handle slot capacity", h >> 24));
                }
                return Ok((slot, generation));
            }
            if self.blen() >= (1 << 28) {
                return Err("heap: arena exhausted".into());
            }
            self.grow();
        }
    }

    pub fn alloc(&mut self, size: usize) -> (u32, u32) {
        self.try_alloc(size).expect("alloc")
    }

    pub fn try_alloc_with_mask(&mut self, size: usize, init_mask: &[u64]) -> Result<(u32, u32), String> {
        let (slot, g) = self.try_alloc(size)?;
        let off = slot as u64;
        let mw = mask_words(size as u64) as usize;
        for w in 0..mw.min(init_mask.len()) {
            self.w_bytes(off + BLK_HDR + w as u64 * 8, &init_mask[w].to_le_bytes());
        }
        Ok((slot, g))
    }

    pub fn alloc_with_mask(&mut self, size: usize, init_mask: &[u64]) -> (u32, u32) {
        self.try_alloc_with_mask(size, init_mask).expect("alloc_with_mask")
    }

    pub fn ld(&self, slot: u32, generation: u32, offset: usize) -> Result<(u64, bool), String> {
        let off = self.valid(slot, generation, "ld")?;
        let size = self.block_size(off);
        if offset as u64 >= size {
            return Err(format!("ld: offset {} out of bounds (size {})", offset, size));
        }
        let ds = self.data_start(off, size);
        let val = self.r64(ds + offset as u64 * 8);
        let mw = self.r64(off + BLK_HDR + (offset as u64 / 64) * 8);
        let is_handle = (mw >> (offset as u64 & 63)) & 1 == 1;
        Ok((val, is_handle))
    }

    // Cell read/write is a trivial bounds-checked byte op (not allocation logic),
    // so it stays host-side — the abrase core owns only alloc/free/rc.
    pub fn st(&mut self, slot: u32, generation: u32, offset: usize, val: u64, is_handle: bool)
        -> Result<(u64, bool), String>
    {
        let off = self.valid(slot, generation, "st")?;
        let size = self.block_size(off);
        if offset as u64 >= size {
            return Err(format!("st: offset {} out of bounds (size {})", offset, size));
        }
        let waddr = self.data_start(off, size) + offset as u64 * 8;
        let old = self.r64(waddr);
        let maddr = off + BLK_HDR + (offset as u64 / 64) * 8;
        let mword = self.r64(maddr);
        let bit = 1u64 << (offset as u64 & 63);
        let old_is_handle = mword & bit != 0;
        self.w_bytes(waddr, &val.to_le_bytes());
        let newm = if is_handle { mword | bit } else { mword & !bit };
        self.w_bytes(maddr, &newm.to_le_bytes());
        Ok((old, old_is_handle))
    }

    // ds is a multiple of 8, so the byte offset maps to an exact word index.
    fn data_words(&self, off: u64, size: u64) -> &[u64] {
        let ds = self.data_start(off, size);
        debug_assert!(ds & 7 == 0, "cell data not word-aligned");
        let w = (ds / 8) as usize;
        &self.arena[w..w + size as usize]
    }

    pub fn cell_data(&self, slot: u32, generation: u32) -> Result<&[u64], String> {
        let off = self.valid(slot, generation, "cell")?;
        let size = self.block_size(off);
        Ok(self.data_words(off, size))
    }

    pub fn cell_data_mut(&mut self, slot: u32, generation: u32) -> Result<&mut [u64], String> {
        let off = self.valid(slot, generation, "cell_mut")?;
        let size = self.block_size(off);
        let ds = self.data_start(off, size);
        debug_assert!(ds & 7 == 0, "cell data not word-aligned");
        let w = (ds / 8) as usize;
        Ok(&mut self.arena[w..w + size as usize])
    }

    pub fn cell_mask(&self, slot: u32, generation: u32) -> Result<&[u64], String> {
        let off = self.valid(slot, generation, "cell_mask")?;
        let size = self.block_size(off);
        let w = ((off + BLK_HDR) / 8) as usize;
        Ok(&self.arena[w..w + mask_words(size) as usize])
    }

    pub fn size(&self, slot: u32, generation: u32) -> Result<usize, String> {
        let off = self.valid(slot, generation, "size")?;
        Ok(self.block_size(off) as usize)
    }

    pub fn is_live(&self, slot: u32, generation: u32) -> bool {
        let off = slot as u64;
        off >= GLOBAL_HDR
            && off + BLK_HDR <= self.blen()
            && self.r32(off) != 0
            && (self.r32(off + 4) & 0x00FF_FFFF) == generation
    }

    pub fn rc_inc_handle(&mut self, raw: u64) -> Result<(), String> {
        if raw == polka::HANDLE_NONE { return Ok(()); }
        let (s, g) = handle_parts(raw);
        self.rc_inc(s, g)
    }
    pub fn rc_dec_handle(&mut self, raw: u64) -> Result<(), String> {
        if raw == polka::HANDLE_NONE { return Ok(()); }
        let (s, g) = handle_parts(raw);
        self.rc_dec(s, g)?;
        Ok(())
    }

    pub fn rc_inc(&mut self, slot: u32, generation: u32) -> Result<(), String> {
        self.valid(slot, generation, "rc_inc")?;
        let h = make_handle(slot, generation);
        let r = cgen::rc_inc(self.bytes_mut(), h).map_err(String::from)?;
        if r != 0 { return Err(format!("rc_inc: stale slot {}", slot)); }
        Ok(())
    }

    pub fn rc_dec(&mut self, slot: u32, generation: u32) -> Result<bool, String> {
        self.valid(slot, generation, "rc_dec")?;
        let h = make_handle(slot, generation);
        let r = cgen::rc_dec(self.bytes_mut(), h).map_err(String::from)?;
        Ok(r == 1)
    }

    pub fn force_free(&mut self, slot: u32, generation: u32) -> Result<(), String> {
        if !self.is_live(slot, generation) { return Ok(()); }
        let off = slot as u64;
        self.w_bytes(off, &1u32.to_le_bytes());
        let h = make_handle(slot, generation);
        cgen::rc_dec(self.bytes_mut(), h).map_err(String::from)?;
        Ok(())
    }

    fn scan_live<F: FnMut(u64)>(&self, mut f: F) {
        let frontier = self.r64(HDR_FRONTIER);
        let mut off = GLOBAL_HDR;
        while off < frontier {
            let size = self.block_size(off);
            let span = BLK_HDR + mask_words(size) * 8 + size * 8;
            if self.r32(off) != 0 { f(off); }
            off += span;
        }
    }

    pub fn live_count(&self) -> usize {
        let mut n = 0;
        self.scan_live(|_| n += 1);
        n
    }

    pub fn rc(&self, slot: u32, generation: u32) -> Option<u32> {
        self.valid(slot, generation, "rc").ok().map(|off| self.r32(off))
    }

    pub fn live_cells(&self) -> Vec<(u32, u32, u32, Vec<u64>, Vec<bool>)> {
        let mut out = Vec::new();
        self.scan_live(|off| {
            let size = self.block_size(off);
            let data: Vec<u64> = self.data_words(off, size).to_vec();
            let mask: Vec<bool> = (0..size).map(|i| {
                let mw = self.r64(off + BLK_HDR + (i / 64) * 8);
                (mw >> (i & 63)) & 1 == 1
            }).collect();
            out.push((off as u32, self.r32(off + 4) & 0x00FF_FFFF, self.r32(off), data, mask));
        });
        out
    }

    pub fn clear(&mut self) {
        for w in self.arena.iter_mut() { *w = 0; }
        let blen = self.blen();
        cgen::core_init(self.bytes_mut(), blen).expect("core_init");
    }

    // O(1) conservative upper bound: bytes bumped past the global header (counts
    // freed-but-not-reclaimed space too). Used only for the RAM cap, where
    // over-reporting is safe; called per allocation, so it must not scan.
    pub fn bytes_used(&self) -> usize {
        (self.r64(HDR_FRONTIER) - GLOBAL_HDR) as usize
    }

    pub fn set_trace(&mut self, _slot: Option<u32>, _all: bool, _out: fn(&str)) {}
}
