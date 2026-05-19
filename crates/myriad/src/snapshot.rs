use polka::{Chunk, Module, HANDLE_NONE};
use crate::memory::mask_bit;
use crate::{cont_slot, VirtualMachine};

#[derive(Clone, Copy)]
pub struct SnapshotHandle {
    pub slot: u32,
    pub generation: u32,
    pub count: usize,
}

impl SnapshotHandle {
    pub const EMPTY: Self = Self { slot: 0, generation: 0, count: 0 };
    pub fn is_empty(&self) -> bool { self.count == 0 }
}

impl VirtualMachine {
    pub(crate) fn current_fn_reg_count(&self, module: &Module) -> usize {
        match module.functions.get(self.current_func) {
            Some(Chunk::Bytecode(b)) => b.reg_count,
            _ => 0,
        }
    }

    pub(crate) fn snapshot_registers(
        &mut self,
        base: usize,
        reg_count: usize,
    ) -> Result<SnapshotHandle, String> {
        let snap = self.snapshot_registers_inner(base, reg_count)?;
        if !snap.is_empty() {
            self.region_record_alloc(snap.slot, snap.generation);
        }
        Ok(snap)
    }

    pub(crate) fn snapshot_registers_off_region(
        &mut self,
        base: usize,
        reg_count: usize,
    ) -> Result<SnapshotHandle, String> {
        self.snapshot_registers_inner(base, reg_count)
    }

    fn snapshot_registers_inner(
        &mut self,
        base: usize,
        reg_count: usize,
    ) -> Result<SnapshotHandle, String> {
        if reg_count == 0 { return Ok(SnapshotHandle::EMPTY); }
        let end = base + reg_count;
        if end > self.registers.len() {
            return Err(format!(
                "snapshot_registers: window [{}..{}] exceeds register array (len {})",
                base, end, self.registers.len()
            ));
        }
        self.mem_charge(reg_count.saturating_mul(8))?;
        let mut init_mask = vec![0u64; (reg_count + 63) / 64];
        for i in 0..reg_count {
            if self.reg_mask_bit(base + i) {
                init_mask[i / 64] |= 1u64 << (i % 64);
            }
        }
        let (slot, generation) = self.heap.try_alloc_with_mask(reg_count, &init_mask)?;
        for i in 0..reg_count {
            let v = self.registers[base + i];
            let is_handle = mask_bit(&init_mask, i);
            if is_handle && v != HANDLE_NONE {
                let s = ((v >> 24) & 0x00FF_FFFF) as u32;
                let g = (v & 0x00FF_FFFF) as u32;
                self.heap.rc_inc(s, g)?;
            }
            self.heap.st(slot, generation, i, v, is_handle)?;
        }
        Ok(SnapshotHandle { slot, generation, count: reg_count })
    }

    pub(crate) fn restore_registers(
        &mut self,
        base: usize,
        snapshot: SnapshotHandle,
    ) -> Result<(), String> {
        if snapshot.is_empty() { return Ok(()); }
        let needed = base + snapshot.count;
        if needed > self.registers.len() {
            self.registers.resize(needed, HANDLE_NONE);
            let mask_words_needed = (needed + 63) / 64;
            if mask_words_needed > self.register_mask.len() {
                self.register_mask.resize(mask_words_needed, 0);
            }
        }
        for i in 0..snapshot.count {
            let (new_val, new_is_handle) = self.heap.ld(snapshot.slot, snapshot.generation, i)?;
            let abs = base + i;
            let old_val = self.registers[abs];
            let old_is_handle = self.reg_mask_bit(abs);
            let stale = if old_is_handle && old_val != HANDLE_NONE {
                let s = ((old_val >> 24) & 0x00FF_FFFF) as u32;
                let g = (old_val & 0x00FF_FFFF) as u32;
                !self.heap.is_live(s, g)
            } else { false };
            if old_is_handle && !stale && old_val != HANDLE_NONE {
                let s = ((old_val >> 24) & 0x00FF_FFFF) as u32;
                let g = (old_val & 0x00FF_FFFF) as u32;
                self.heap.rc_dec(s, g)?;
            }
            if new_is_handle && new_val != HANDLE_NONE {
                let s = ((new_val >> 24) & 0x00FF_FFFF) as u32;
                let g = (new_val & 0x00FF_FFFF) as u32;
                self.heap.rc_inc(s, g)?;
            }
            self.registers[abs] = new_val;
            self.set_reg_mask_bit(abs, new_is_handle);
        }
        Ok(())
    }

    pub(crate) fn write_snapshot_into_cell(
        &mut self,
        cell_slot: u32,
        cell_gen: u32,
        snapshot: SnapshotHandle,
    ) -> Result<(), String> {
        if snapshot.is_empty() {
            self.heap.st(cell_slot, cell_gen, cont_slot::REGS_SNAPSHOT_SLOT, HANDLE_NONE, true)?;
            self.heap.st(cell_slot, cell_gen, cont_slot::REGS_COUNT, 0, false)?;
            return Ok(());
        }
        let handle = polka::Value::from_handle(snapshot.slot, snapshot.generation).raw();
        self.heap.st(cell_slot, cell_gen, cont_slot::REGS_SNAPSHOT_SLOT, handle, true)?;
        self.heap.st(cell_slot, cell_gen, cont_slot::REGS_COUNT, snapshot.count as u64, false)?;
        Ok(())
    }
}
