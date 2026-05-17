use super::{Value, BoxPool};

pub const HEAP_BYTES_PER_VALUE: usize = std::mem::size_of::<Value>();

pub struct Heap {
    cells: Vec<Option<Vec<Value>>>,
    rc: Vec<u32>,
    generation: Vec<u32>,
    free_list: Vec<u32>,
    bytes_used: usize,
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

    pub fn alloc(&mut self, size: usize) -> (u32, u32) {
        let slot = vec![Value::NONE; size];
        self.bytes_used = self.bytes_used.saturating_add(size * HEAP_BYTES_PER_VALUE);
        if let Some(h) = self.free_list.pop() {
            let idx = h as usize;
            self.cells[idx] = Some(slot);
            self.rc[idx] = 1;
            self.generation[idx] = self.generation[idx].wrapping_add(1);
            (h, self.generation[idx])
        } else {
            self.cells.push(Some(slot));
            self.rc.push(1);
            self.generation.push(0);
            let h = (self.cells.len() - 1) as u32;
            (h, 0)
        }
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

    pub fn ld(&self, slot: u32, generation: u32, offset: usize) -> Result<Value, String> {
        let idx = self.check(slot, generation, "ld")?;
        let cell = self.cells[idx].as_ref().unwrap();
        cell.get(offset).cloned()
            .ok_or_else(|| format!("ld: offset {} out of bounds (size {})", offset, cell.len()))
    }

    pub fn st(&mut self, slot: u32, generation: u32, offset: usize, val: Value) -> Result<Value, String> {
        let idx = self.check(slot, generation, "st")?;
        let cell = self.cells[idx].as_mut().unwrap();
        if offset >= cell.len() {
            return Err(format!("st: offset {} out of bounds (size {})", offset, cell.len()));
        }
        let old = std::mem::replace(&mut cell[offset], val);
        Ok(old)
    }

    pub fn size(&self, slot: u32, generation: u32) -> Result<usize, String> {
        let idx = self.check(slot, generation, "size")?;
        Ok(self.cells[idx].as_ref().unwrap().len())
    }

    pub fn rc_inc(&mut self, slot: u32, generation: u32) -> Result<(), String> {
        let idx = self.check(slot, generation, "rc_inc")?;
        self.rc[idx] = self.rc[idx]
            .checked_add(1)
            .ok_or_else(|| format!("rc_inc: refcount overflow on slot {}", slot))?;
        Ok(())
    }

    // At rc=0: recursively rc_dec child handles AND box_pool.dec child boxes,
    // then reclaim. Returns whether reclaimed.
    pub fn rc_dec(&mut self, slot: u32, generation: u32, pool: &mut BoxPool) -> Result<bool, String> {
        let idx = self.check(slot, generation, "rc_dec")?;
        if self.rc[idx] == 0 {
            return Err(format!("rc_dec: refcount underflow on slot {}", slot));
        }
        self.rc[idx] -= 1;
        if self.rc[idx] != 0 {
            return Ok(false);
        }
        let cell = self.cells[idx].take().unwrap();
        self.bytes_used = self.bytes_used.saturating_sub(cell.len() * HEAP_BYTES_PER_VALUE);
        self.free_list.push(slot);
        for v in cell {
            if let Some((s, g)) = v.as_handle() {
                self.rc_dec(s, g, pool)?;
            } else if let Some(box_idx) = v.as_box() {
                pool.dec_cascade(box_idx, self);
            }
        }
        Ok(true)
    }

    // Idempotent against ordinary rc=0 reclaim. Used by region_pop.
    pub fn force_free(&mut self, slot: u32, generation: u32, pool: &mut BoxPool) -> Result<(), String> {
        let idx = slot as usize;
        if idx >= self.cells.len() { return Ok(()); }
        if self.cells[idx].is_none() { return Ok(()); }
        if self.generation[idx] != generation { return Ok(()); }
        let cell = self.cells[idx].take().unwrap();
        self.bytes_used = self.bytes_used.saturating_sub(cell.len() * HEAP_BYTES_PER_VALUE);
        self.rc[idx] = 0;
        self.free_list.push(slot);
        for v in cell {
            if let Some((s, g)) = v.as_handle() {
                if let Err(e) = self.rc_dec(s, g, pool) {
                    debug_assert!(false, "force_free cascade rc_dec failed: {}", e);
                }
            } else if let Some(box_idx) = v.as_box() {
                pool.dec_cascade(box_idx, self);
            }
        }
        Ok(())
    }

    pub fn live_count(&self) -> usize {
        self.cells.iter().filter(|c| c.is_some()).count()
    }

    // Drop every slot and bookkeeping vector — used by run_module_inner at
    // entry so consecutive runs start from a clean heap regardless of
    // whatever state the previous run left behind.
    pub fn clear(&mut self) {
        self.cells.clear();
        self.rc.clear();
        self.generation.clear();
        self.free_list.clear();
        self.bytes_used = 0;
    }
}
