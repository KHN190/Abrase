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

#[cfg(test)]
mod snapshot_tests {
    use super::*;
    use crate::HandlerFrame;
    use polka::{BytecodeChunk, Chunk, Module};

    fn vm() -> VirtualMachine { VirtualMachine::new() }

    fn encode_handle(slot: u32, gen_: u32) -> u64 {
        ((slot as u64) << 24) | (gen_ as u64)
    }

    #[test]
    fn snapshot_handle_empty_is_empty() {
        assert!(SnapshotHandle::EMPTY.is_empty());
    }

    #[test]
    fn snapshot_handle_zero_count_is_empty() {
        let h = SnapshotHandle { slot: 5, generation: 1, count: 0 };
        assert!(h.is_empty());
    }

    #[test]
    fn snapshot_handle_with_count_is_not_empty() {
        let h = SnapshotHandle { slot: 0, generation: 0, count: 1 };
        assert!(!h.is_empty());
    }

    #[test]
    fn snapshot_zero_reg_count_returns_empty() {
        let mut v = vm();
        let snap = v.snapshot_registers(0, 0).expect("zero reg snapshot");
        assert!(snap.is_empty());
    }

    #[test]
    fn snapshot_window_out_of_range_errors() {
        let mut v = vm();
        let r = v.snapshot_registers(0, 100);
        assert!(r.is_err(), "out-of-range window must error");
        let msg = match r { Err(e) => e, _ => unreachable!() };
        assert!(msg.contains("exceeds register array"), "got: {}", msg);
    }

    #[test]
    fn snapshot_captures_int_values() {
        let mut v = vm();
        v.ensure_registers(4);
        v.registers[0] = 11;
        v.registers[1] = 22;
        v.registers[2] = 33;
        v.registers[3] = 44;
        let snap = v.snapshot_registers(0, 4).expect("snapshot");
        assert_eq!(snap.count, 4);
        let (a, _) = v.heap.ld(snap.slot, snap.generation, 0).unwrap();
        let (b, _) = v.heap.ld(snap.slot, snap.generation, 3).unwrap();
        assert_eq!(a, 11);
        assert_eq!(b, 44);
    }

    #[test]
    fn snapshot_captures_handle_and_rc_incs() {
        let mut v = vm();
        let (slot, gen_) = v.heap_alloc(1);
        let handle = encode_handle(slot, gen_);
        v.ensure_registers(1);
        v.registers[0] = handle;
        v.set_reg_mask_bit(0, true);

        let snap = v.snapshot_registers(0, 1).expect("snapshot");
        assert_eq!(snap.count, 1);
        let (raw, is_h) = v.heap.ld(snap.slot, snap.generation, 0).unwrap();
        assert_eq!(raw, handle);
        assert!(is_h);
        // alloc rc=1 + rc_inc from snapshot capture => rc must allow one more dec
        v.heap.rc_dec(slot, gen_).expect("first dec");
        assert!(v.heap.is_live(slot, gen_), "rc must still be > 0 after one dec");
        v.heap.rc_dec(slot, gen_).expect("second dec");
        assert!(!v.heap.is_live(slot, gen_), "cell freed after balancing dec");
    }

    #[test]
    fn snapshot_off_region_does_not_record() {
        let mut v = vm();
        v.region_push();
        v.ensure_registers(2);
        v.registers[0] = 1;
        v.registers[1] = 2;
        let snap = v.snapshot_registers_off_region(0, 2).expect("snapshot");
        let live_before = v.heap_live_count();
        v.region_pop().expect("region pop");
        // off-region snapshot must survive region pop
        assert!(v.heap.is_live(snap.slot, snap.generation));
        assert_eq!(v.heap_live_count(), live_before);
    }

    #[test]
    fn snapshot_in_active_region_freed_at_region_pop() {
        let mut v = vm();
        v.region_push();
        v.ensure_registers(2);
        v.registers[0] = 1;
        v.registers[1] = 2;
        let snap = v.snapshot_registers(0, 2).expect("snapshot");
        assert!(v.heap.is_live(snap.slot, snap.generation));
        v.region_pop().expect("region pop");
        assert!(!v.heap.is_live(snap.slot, snap.generation),
            "region-tracked snapshot must be force-freed at pop");
    }

    #[test]
    fn restore_empty_snapshot_is_no_op() {
        let mut v = vm();
        v.ensure_registers(2);
        v.registers[0] = 99;
        let live_before = v.heap_live_count();
        v.restore_registers(0, SnapshotHandle::EMPTY).expect("restore empty");
        assert_eq!(v.registers[0], 99, "empty restore must not touch registers");
        assert_eq!(v.heap_live_count(), live_before);
    }

    #[test]
    fn restore_extends_register_array_when_needed() {
        let mut v = vm();
        v.ensure_registers(1);
        v.registers[0] = 0;
        // Build a snapshot of count=4 by hand using snapshot_registers,
        // then restore at base=10 to force extension.
        v.ensure_registers(4);
        for i in 0..4 { v.registers[i] = (i * 10) as u64; }
        let snap = v.snapshot_registers(0, 4).expect("snapshot");
        v.restore_registers(10, snap).expect("restore");
        assert!(v.registers.len() >= 14);
        assert_eq!(v.registers[10], 0);
        assert_eq!(v.registers[13], 30);
    }

    #[test]
    fn restore_balances_rc_on_handles() {
        let mut v = vm();
        let (old_slot, old_gen) = v.heap_alloc(1);
        let (new_slot, new_gen) = v.heap_alloc(1);
        v.ensure_registers(1);

        // Snapshot containing the new handle.
        v.registers[0] = encode_handle(new_slot, new_gen);
        v.set_reg_mask_bit(0, true);
        let snap = v.snapshot_registers(0, 1).expect("snapshot");

        // Replace register with old handle (so restore must rc_dec old).
        v.heap.rc_dec(new_slot, new_gen).expect("dec captured rc_inc");
        v.registers[0] = encode_handle(old_slot, old_gen);
        v.set_reg_mask_bit(0, true);

        v.restore_registers(0, snap).expect("restore");

        // After restore: old cell rc_dec'd, new cell rc_inc'd.
        // old was rc=1 → freed. new was rc=1 → rc=2 (one from snap, one from restore).
        assert!(!v.heap.is_live(old_slot, old_gen), "old slot must be freed by rc_dec");
        assert!(v.heap.is_live(new_slot, new_gen));
        assert_eq!(v.registers[0], encode_handle(new_slot, new_gen));
        assert!(v.reg_mask_bit(0));
    }

    #[test]
    fn restore_skips_rc_dec_on_stale_old_value() {
        let mut v = vm();
        let (dead_slot, dead_gen) = v.heap_alloc(1);
        v.heap.force_free(dead_slot, dead_gen).expect("kill the cell");

        v.ensure_registers(1);
        // Snapshot a non-handle value (Int).
        v.registers[0] = 42;
        let snap = v.snapshot_registers(0, 1).expect("snapshot");

        // Put a stale handle into register: mask bit on, but cell is dead.
        v.registers[0] = encode_handle(dead_slot, dead_gen);
        v.set_reg_mask_bit(0, true);

        // Restore must not crash on stale rc_dec.
        v.restore_registers(0, snap).expect("restore handles stale gracefully");
        assert_eq!(v.registers[0], 42);
        assert!(!v.reg_mask_bit(0));
    }

    #[test]
    fn write_snapshot_into_cell_empty() {
        let mut v = vm();
        let (cell_slot, cell_gen) = v.heap_alloc(cont_slot::SIZE);
        v.write_snapshot_into_cell(cell_slot, cell_gen, SnapshotHandle::EMPTY)
            .expect("write empty");
        let (slot_raw, _) = v.heap.ld(cell_slot, cell_gen, cont_slot::REGS_SNAPSHOT_SLOT).unwrap();
        let (count_raw, _) = v.heap.ld(cell_slot, cell_gen, cont_slot::REGS_COUNT).unwrap();
        assert_eq!(slot_raw, HANDLE_NONE);
        assert_eq!(count_raw, 0);
    }

    #[test]
    fn write_snapshot_into_cell_non_empty() {
        let mut v = vm();
        let (cell_slot, cell_gen) = v.heap_alloc(cont_slot::SIZE);
        let snap = SnapshotHandle { slot: 7, generation: 3, count: 5 };
        v.write_snapshot_into_cell(cell_slot, cell_gen, snap).expect("write");
        let (slot_raw, is_h) = v.heap.ld(cell_slot, cell_gen, cont_slot::REGS_SNAPSHOT_SLOT).unwrap();
        let (count_raw, _) = v.heap.ld(cell_slot, cell_gen, cont_slot::REGS_COUNT).unwrap();
        assert_eq!(slot_raw, encode_handle(7, 3));
        assert!(is_h);
        assert_eq!(count_raw, 5);
    }

    #[test]
    fn current_fn_reg_count_for_bytecode() {
        let v = vm();
        let module = Module {
            functions: vec![Chunk::Bytecode(BytecodeChunk {
                code: Vec::new(),
                constants: Vec::new(),
                const_mask: Vec::new(),
                reg_count: 42,
                param_count: 0,
                string_constants: Vec::new(),
            })],
            entry: 0,
        };
        assert_eq!(v.current_fn_reg_count(&module), 42);
    }

    #[test]
    fn current_fn_reg_count_out_of_range_returns_zero() {
        let mut v = vm();
        v.current_func = 999;
        let module = Module {
            functions: Vec::new(),
            entry: 0,
        };
        assert_eq!(v.current_fn_reg_count(&module), 0);
    }

    #[test]
    fn snapshot_then_restore_round_trip_preserves_values() {
        let mut v = vm();
        v.ensure_registers(3);
        v.registers[0] = 1;
        v.registers[1] = 2;
        v.registers[2] = 3;
        let snap = v.snapshot_registers(0, 3).expect("snap");

        v.registers[0] = 100;
        v.registers[1] = 200;
        v.registers[2] = 300;

        v.restore_registers(0, snap).expect("restore");
        assert_eq!(v.registers[0], 1);
        assert_eq!(v.registers[1], 2);
        assert_eq!(v.registers[2], 3);
    }

    #[test]
    fn snapshot_with_handler_frame_cont_path() {
        // Smoke: handler frame holds (cell_slot, cell_gen); write_snapshot_into_cell
        // writes correctly under typical usage.
        let mut v = vm();
        let (cell_slot, cell_gen) = v.heap_alloc(cont_slot::SIZE);
        v.push_handler(HandlerFrame {
            effect_id: 1,
            dispatch_table_slot: None,
            dispatch_table_gen: 0,
            cell_slot,
            cell_gen,
            cells_allocated: vec![(cell_slot, cell_gen)],
            body_frame_index: None,
            pending_return_arm_fn: None,
            pending_return_arm_env: HANDLE_NONE,
            pending_return_arm_env_is_handle: false,
        });

        v.ensure_registers(2);
        v.registers[0] = 77;
        v.registers[1] = 88;
        let snap = v.snapshot_registers_off_region(0, 2).expect("off-region snap");
        v.write_snapshot_into_cell(cell_slot, cell_gen, snap).expect("write into cont");

        let (snap_raw, snap_is_h) = v.heap.ld(cell_slot, cell_gen, cont_slot::REGS_SNAPSHOT_SLOT).unwrap();
        let (count_raw, _) = v.heap.ld(cell_slot, cell_gen, cont_slot::REGS_COUNT).unwrap();
        assert_eq!(snap_raw, encode_handle(snap.slot, snap.generation));
        assert!(snap_is_h);
        assert_eq!(count_raw, 2);
    }
}
