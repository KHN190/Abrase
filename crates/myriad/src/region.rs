use super::memory::Heap;

pub struct RegionTable {
    stack: Vec<Vec<(u32, u32)>>,
}

impl RegionTable {
    pub fn new() -> Self {
        Self { stack: Vec::new() }
    }

    pub fn push(&mut self) {
        self.stack.push(Vec::new());
    }

    #[inline]
    pub fn is_active(&self) -> bool { !self.stack.is_empty() }

    #[inline]
    pub fn record_alloc(&mut self, slot: u32, generation: u32) {
        if let Some(top) = self.stack.last_mut() {
            top.push((slot, generation));
        }
    }

    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    pub fn pop_and_release(&mut self, heap: &mut Heap) -> Result<(), String> {
        let allocs = self.stack.pop()
            .ok_or_else(|| "region_pop: no active region".to_string())?;
        for (slot, generation) in allocs {
            if let Err(e) = heap.force_free(slot, generation) {
                debug_assert!(false, "region pop force_free failed: {}", e);
            }
        }
        Ok(())
    }

    pub fn clear(&mut self) {
        self.stack.clear();
    }

    pub fn forget(&mut self, slot: u32, generation: u32) -> bool {
        // record_alloc only ever pushes to the top region, so a live cell can
        // appear in at most that one. Scan top only.
        let Some(top) = self.stack.last_mut() else { return false; };
        let before = top.len();
        top.retain(|(s, g)| !(*s == slot && *g == generation));
        top.len() < before
    }
}
