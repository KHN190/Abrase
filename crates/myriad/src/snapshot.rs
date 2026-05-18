// Register-window snapshots for handler resume.

use polka::{Chunk, Module, Value};
use crate::{cont_slot, memory::HEAP_BYTES_PER_VALUE, VirtualMachine};

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

    // Region-tracked snapshot: lives as long as the surrounding region.
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

    // Off-region snapshot: not tracked by the region stack.
    // Caller is responsible for rc_dec'ing the snapshot on consumption.
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
        self.mem_charge(reg_count.saturating_mul(HEAP_BYTES_PER_VALUE))?;
        let (slot, generation) = self.heap.alloc(reg_count);
        for i in 0..reg_count {
            let val = self.registers[base + i];
            self.value_rc_inc(&val)?;
            self.heap.st(slot, generation, i, val)?;
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
            self.registers.resize(needed, Value::NONE);
        }
        for i in 0..snapshot.count {
            let new_val = self.heap.ld(snapshot.slot, snapshot.generation, i)?;
            let old = std::mem::replace(&mut self.registers[base + i], Value::NONE);
            // OLD may be a stale leftover written by an intermediate frame that
            // borrowed this physical address (arm_continuation: solve's nested
            // calls overlap arm's window). Such a handle's cell was force-freed
            // by region_pop and has no rc to decrement.
            let stale = matches!(old.as_handle(), Some((s, g)) if !self.heap.is_live(s, g));
            if !stale {
                self.value_rc_dec(&old)?;
            }
            self.value_rc_inc(&new_val)?;
            self.registers[base + i] = new_val;
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
            self.heap.st(cell_slot, cell_gen, cont_slot::REGS_SNAPSHOT_SLOT, Value::NONE)?;
            self.heap.st(cell_slot, cell_gen, cont_slot::REGS_COUNT, Value::from_int(0))?;
            return Ok(());
        }
        self.heap.st(cell_slot, cell_gen, cont_slot::REGS_SNAPSHOT_SLOT,
            Value::from_handle(snapshot.slot, snapshot.generation))?;
        self.heap.st(cell_slot, cell_gen, cont_slot::REGS_COUNT,
            Value::from_int(snapshot.count as i64))?;
        Ok(())
    }

}
