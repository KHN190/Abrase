use super::memory::Heap;
use super::value::BoxPool;

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

    pub fn record_alloc(&mut self, slot: u32, generation: u32) {
        if let Some(top) = self.stack.last_mut() {
            top.push((slot, generation));
        }
    }

    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    pub fn pop_and_release(&mut self, heap: &mut Heap, pool: &mut BoxPool) -> Result<(), String> {
        let allocs = self.stack.pop()
            .ok_or_else(|| "region_pop: no active region".to_string())?;
        for (slot, generation) in allocs {
            if let Err(e) = heap.force_free(slot, generation, pool) {
                debug_assert!(false, "region pop force_free failed: {}", e);
            }
        }
        Ok(())
    }

    // Drop all bookkeeping without force-freeing. Called by run_module_inner
    // at entry so leftover frames from a prior (aborted or buggy) run don't
    // see slots reused by the new run as their own.
    pub fn clear(&mut self) {
        self.stack.clear();
    }

    // Remove a (slot, generation) from every active region. Used by
    // break/return/throw codegen to "promote" the carried value past the
    // upcoming region pops so its heap cell isn't force-freed. A handle can
    // only have been recorded in one region (the topmost at alloc time), but
    // walking the whole stack is cheap and removes the need for codegen to
    // know which depth it lived in.
    pub fn forget(&mut self, slot: u32, generation: u32) {
        for region in &mut self.stack {
            region.retain(|(s, g)| !(*s == slot && *g == generation));
        }
    }
}
