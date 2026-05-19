use polka::{HANDLE_NONE, HANDLE_SLOT_MAX};

pub const HEAP_BYTES_PER_SLOT: usize = 8 + 1; // data u64 + amortized mask bit

pub struct Cell {
    pub data: Box<[u64]>,
    // Bit i (LSB of word i/64) = 1 → data[i] is a handle.
    pub mask: Box<[u64]>,
}

pub struct Heap {
    cells: Vec<Option<Cell>>,
    rc: Vec<u32>,
    generation: Vec<u32>,
    free_list: Vec<u32>,
    bytes_used: usize,
}

#[inline]
fn mask_words_for(size: usize) -> usize { (size + 63) / 64 }

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

impl Heap {
    pub fn new() -> Self {
        Self {
            cells: Vec::new(),
            rc: Vec::new(),
            generation: Vec::new(),
            free_list: Vec::new(),
            bytes_used: 0,
        }
    }

    pub fn bytes_used(&self) -> usize { self.bytes_used }

    fn make_cell(size: usize, init_mask: &[u64]) -> Cell {
        let data: Box<[u64]> = vec![HANDLE_NONE; size].into_boxed_slice();
        let nwords = mask_words_for(size);
        let mut mask = vec![0u64; nwords];
        let copy_n = init_mask.len().min(nwords);
        mask[..copy_n].copy_from_slice(&init_mask[..copy_n]);
        Cell { data, mask: mask.into_boxed_slice() }
    }

    fn alloc_inner(&mut self, size: usize, init_mask: &[u64]) -> Result<(u32, u32), String> {
        let cell = Self::make_cell(size, init_mask);
        self.bytes_used = self.bytes_used.saturating_add(
            size * 8 + mask_words_for(size) * 8
        );
        let (h, g) = if let Some(h) = self.free_list.pop() {
            let idx = h as usize;
            self.cells[idx] = Some(cell);
            self.rc[idx] = 1;
            self.generation[idx] = self.generation[idx].wrapping_add(1);
            (h, self.generation[idx])
        } else {
            if self.cells.len() as u32 > HANDLE_SLOT_MAX {
                return Err(format!(
                    "heap.alloc: slot index would exceed handle limit {}",
                    HANDLE_SLOT_MAX
                ));
            }
            self.cells.push(Some(cell));
            self.rc.push(1);
            self.generation.push(0);
            let h = (self.cells.len() - 1) as u32;
            (h, 0)
        };
        if std::env::var("TRACE_SLOT").map(|v| v.parse::<u32>().ok() == Some(h)).unwrap_or(false) {
            eprintln!("[ALLOC] slot {} gen {} size {}", h, g, size);
        }
        Ok((h, g))
    }

    pub fn alloc(&mut self, size: usize) -> (u32, u32) {
        self.alloc_inner(size, &[]).expect("alloc slot overflow")
    }

    pub fn alloc_with_mask(&mut self, size: usize, init_mask: &[u64]) -> (u32, u32) {
        self.alloc_inner(size, init_mask).expect("alloc slot overflow")
    }

    pub fn try_alloc(&mut self, size: usize) -> Result<(u32, u32), String> {
        self.alloc_inner(size, &[])
    }

    pub fn try_alloc_with_mask(&mut self, size: usize, init_mask: &[u64]) -> Result<(u32, u32), String> {
        self.alloc_inner(size, init_mask)
    }

    fn check(&self, slot: u32, generation: u32, op: &str) -> Result<usize, String> {
        let idx = slot as usize;
        if idx >= self.cells.len() {
            return Err(format!("{}: invalid slot {}", op, slot));
        }
        if self.cells[idx].is_none() {
            return Err(format!("{}: use-after-free of slot {}", op, slot));
        }
        if self.generation[idx] != generation {
            return Err(format!(
                "{}: stale handle for slot {} (have generation {}, live generation {})",
                op, slot, generation, self.generation[idx]
            ));
        }
        Ok(idx)
    }

    // Returns (raw u64, is_handle_bit).
    pub fn ld(&self, slot: u32, generation: u32, offset: usize) -> Result<(u64, bool), String> {
        let idx = self.check(slot, generation, "ld")?;
        let cell = self.cells[idx].as_ref().unwrap();
        if offset >= cell.data.len() {
            return Err(format!("ld: offset {} out of bounds (size {})", offset, cell.data.len()));
        }
        let val = cell.data[offset];
        let is_handle = mask_bit(&cell.mask, offset);
        Ok((val, is_handle))
    }

    // Store raw u64; caller supplies whether it is a handle. Returns (old_data, old_is_handle).
    pub fn st(
        &mut self, slot: u32, generation: u32,
        offset: usize, val: u64, is_handle: bool,
    ) -> Result<(u64, bool), String> {
        let idx = self.check(slot, generation, "st")?;
        let cell = self.cells[idx].as_mut().unwrap();
        if offset >= cell.data.len() {
            return Err(format!("st: offset {} out of bounds (size {})", offset, cell.data.len()));
        }
        let old = cell.data[offset];
        let old_is_handle = mask_bit(&cell.mask, offset);
        cell.data[offset] = val;
        mask_set(&mut cell.mask, offset, is_handle);
        Ok((old, old_is_handle))
    }

    pub fn cell_data(&self, slot: u32, generation: u32) -> Result<&[u64], String> {
        let idx = self.check(slot, generation, "cell")?;
        Ok(&self.cells[idx].as_ref().unwrap().data)
    }

    pub fn cell_mask(&self, slot: u32, generation: u32) -> Result<&[u64], String> {
        let idx = self.check(slot, generation, "cell_mask")?;
        Ok(&self.cells[idx].as_ref().unwrap().mask)
    }

    pub fn size(&self, slot: u32, generation: u32) -> Result<usize, String> {
        let idx = self.check(slot, generation, "size")?;
        Ok(self.cells[idx].as_ref().unwrap().data.len())
    }

    #[inline]
    pub fn is_live(&self, slot: u32, generation: u32) -> bool {
        let idx = slot as usize;
        idx < self.cells.len()
            && self.cells[idx].is_some()
            && self.generation[idx] == generation
    }

    pub fn rc_inc(&mut self, slot: u32, generation: u32) -> Result<(), String> {
        let trace = std::env::var("TRACE_SLOT").map(|v| v.parse::<u32>().ok() == Some(slot)).unwrap_or(false);
        let idx = self.check(slot, generation, "rc_inc")?;
        self.rc[idx] = self.rc[idx]
            .checked_add(1)
            .ok_or_else(|| format!("rc_inc: refcount overflow on slot {}", slot))?;
        if trace {
            eprintln!("[RC_INC] slot {} gen {} -> rc {}", slot, generation, self.rc[idx]);
        }
        Ok(())
    }

    // At rc=0: recursively rc_dec child handles per cell mask, then reclaim.
    pub fn rc_dec(&mut self, slot: u32, generation: u32) -> Result<bool, String> {
        let trace = std::env::var("TRACE_SLOT").map(|v| v.parse::<u32>().ok() == Some(slot)).unwrap_or(false);
        if trace {
            let live_gen = self.generation.get(slot as usize).copied().unwrap_or(0);
            let is_live = self.cells.get(slot as usize).map(|c| c.is_some()).unwrap_or(false);
            eprintln!("[RC_DEC] slot {} gen {} (live_gen {} live={})", slot, generation, live_gen, is_live);
        }
        let idx = self.check(slot, generation, "rc_dec")?;
        if self.rc[idx] == 0 {
            return Err(format!("rc_dec: refcount underflow on slot {}", slot));
        }
        self.rc[idx] -= 1;
        if trace {
            eprintln!("[RC_DEC] slot {} -> rc {}", slot, self.rc[idx]);
        }
        if self.rc[idx] != 0 {
            return Ok(false);
        }
        let cell = self.cells[idx].take().unwrap();
        let size = cell.data.len();
        self.bytes_used = self.bytes_used.saturating_sub(size * 8 + mask_words_for(size) * 8);
        self.free_list.push(slot);
        if trace {
            eprintln!("[RC_DEC] slot {} FREED via rc_dec", slot);
        }
        for i in 0..size {
            if mask_bit(&cell.mask, i) {
                let v = cell.data[i];
                if v != HANDLE_NONE {
                    let s = ((v >> 24) & 0x00FF_FFFF) as u32;
                    let g = (v & 0x00FF_FFFF) as u32;
                    self.rc_dec(s, g)?;
                }
            }
        }
        Ok(true)
    }

    // Idempotent against rc=0 reclaim. Used by region_pop.
    pub fn force_free(&mut self, slot: u32, generation: u32) -> Result<(), String> {
        let trace = std::env::var("TRACE_SLOT").map(|v| v.parse::<u32>().ok() == Some(slot)).unwrap_or(false);
        let idx = slot as usize;
        if idx >= self.cells.len() { return Ok(()); }
        if self.cells[idx].is_none() {
            if trace { eprintln!("[FORCE_FREE] slot {} gen {} -- already none", slot, generation); }
            return Ok(());
        }
        if self.generation[idx] != generation {
            if trace { eprintln!("[FORCE_FREE] slot {} gen {} -- generation mismatch (live {})", slot, generation, self.generation[idx]); }
            return Ok(());
        }
        if trace {
            eprintln!("[FORCE_FREE] slot {} gen {} rc was {}", slot, generation, self.rc[idx]);
        }
        let cell = self.cells[idx].take().unwrap();
        let size = cell.data.len();
        self.bytes_used = self.bytes_used.saturating_sub(size * 8 + mask_words_for(size) * 8);
        self.rc[idx] = 0;
        self.free_list.push(slot);
        for i in 0..size {
            if mask_bit(&cell.mask, i) {
                let v = cell.data[i];
                if v != HANDLE_NONE {
                    let s = ((v >> 24) & 0x00FF_FFFF) as u32;
                    let g = (v & 0x00FF_FFFF) as u32;
                    if let Err(e) = self.rc_dec(s, g) {
                        debug_assert!(false, "force_free cascade rc_dec failed: {}", e);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn live_count(&self) -> usize {
        self.cells.iter().filter(|c| c.is_some()).count()
    }

    pub fn clear(&mut self) {
        self.cells.clear();
        self.rc.clear();
        self.generation.clear();
        self.free_list.clear();
        self.bytes_used = 0;
    }
}
