use polka::{Register, Module, HANDLE_NONE};
use crate::memory::mask_bit;
use crate::debug::DebugEvent;
use super::super::{VirtualMachine, Value};
use super::register::{split_port, decode_dispatch_key};


impl VirtualMachine {
    pub(super) fn do_dei(&mut self, d: Register, port_reg: Register) -> Result<(), String> {
        let port_val = self.read_i64(port_reg)?;
        let (device_id, port) = split_port(port_val)?;
        match (device_id, port) {
            (polka::DISPATCH_ID, polka::DISPATCH_PORT_LOOKUP) => {
                let (raw, is_handle) = if let Some(frame) = self.handlers.last() {
                    self.heap.ld(frame.cell_slot, frame.cell_gen, crate::cont_slot::DISPATCH_FN_ID)?
                } else {
                    (self.dispatch_last_result.take().unwrap_or(0xFFFF) as u64, false)
                };
                self.write(d, raw, is_handle)
            }
            (polka::DISPATCH_ID, polka::DISPATCH_PORT_ENV) => {
                let (raw, is_handle) = if let Some(frame) = self.handlers.last() {
                    self.heap.ld(frame.cell_slot, frame.cell_gen, crate::cont_slot::DISPATCH_ENV)?
                } else {
                    self.dispatch_last_env.take().unwrap_or((HANDLE_NONE, false))
                };
                let (raw, is_handle) = if raw == HANDLE_NONE && !is_handle { (0u64, false) } else { (raw, is_handle) };
                if is_handle { self.rc_inc_handle(raw)?; }
                self.write(d, raw, is_handle)
            }
            (polka::MODULE_ID, polka::MODULE_PORT_TABLE) => {
                let (raw, is_handle) = (self.module_table_raw, self.module_table_is_handle);
                // Hand the cart a fresh observer; the stored rc keeps the table
                // alive across calls (same contract as a stateful device).
                if is_handle && raw != HANDLE_NONE { self.rc_inc_handle(raw)?; }
                self.write(d, raw, is_handle)
            }
            _ => {
                let dev = self.devices.get_mut(device_id)
                    .ok_or_else(|| format!("dei: device {:#04x} not installed", device_id))?;
                let (v, is_handle) = dev.read(port)?;
                if is_handle { self.rc_inc_handle(v.raw())?; }
                self.write(d, v.raw(), is_handle)
            }
        }
    }

    pub(super) fn do_deo(&mut self, module: &Module, src: Register, port_reg: Register) -> Result<(), String> {
        let (raw, is_handle) = self.read(src)?;
        let port_val = self.read_i64(port_reg)?;
        let (device_id, port) = split_port(port_val)?;
        match (device_id, port) {
            (0x00, 0x01) => {
                if !is_handle { self.exit_code = Some((raw as i64) & 0xFFFF_FFFF); }
                self.halted = true;
                Ok(())
            }
            (0x00, 0x02) => {
                let msg = if is_handle && raw != HANDLE_NONE {
                    let v = Value::from_raw(raw);
                    crate::value::read_string(&self.heap, v).unwrap_or_else(|| format!("(handle {:?})", v))
                } else {
                    format!("{}", raw as i64)
                };
                Err(format!("panic: {}", msg))
            }
            (polka::DISPATCH_ID, polka::DISPATCH_PORT_LOOKUP) => {
                let key = decode_dispatch_key(raw)?;
                let mut init_mask = vec![0u64; (crate::cont_slot::SIZE + 63) / 64];
                init_mask[0] = crate::cont_slot::INIT_MASK_WORD0;
                let (cell_slot, cell_gen) = self.checked_heap_alloc_with_mask(crate::cont_slot::SIZE, &init_mask)?;
                self.region_record_alloc(cell_slot, cell_gen);
                self.heap.st(cell_slot, cell_gen, crate::cont_slot::SUSPEND_PC, (self.pc - 1) as u64, false)?;
                self.heap.st(cell_slot, cell_gen, crate::cont_slot::SUSPEND_BASE, self.base_reg as u64, false)?;
                self.heap.st(cell_slot, cell_gen, crate::cont_slot::SUSPEND_FUNC, self.current_func as u64, false)?;
                self.heap.st(cell_slot, cell_gen, crate::cont_slot::ALIVE, 1, false)?;

                let reg_count = self.current_fn_reg_count(module);
                let snapshot = self.snapshot_registers(self.base_reg, reg_count)?;
                self.write_snapshot_into_cell(cell_slot, cell_gen, snapshot)?;

                if let Some(handler_frame) = self.handlers.last_mut() {
                    handler_frame.cells_allocated.push((cell_slot, cell_gen));
                    handler_frame.cell_slot = cell_slot;
                    handler_frame.cell_gen = cell_gen;
                }
                let (fn_id, env) = self.resolve_dispatch(key);
                self.heap.st(cell_slot, cell_gen, crate::cont_slot::DISPATCH_FN_ID, fn_id as u64, false)?;
                let (env_raw, env_is_handle) = env.unwrap_or((HANDLE_NONE, false));
                if env_is_handle { self.rc_inc_handle(env_raw)?; }
                self.heap.st(cell_slot, cell_gen, crate::cont_slot::DISPATCH_ENV, env_raw, env_is_handle)?;
                self.dispatch_last_result = Some(fn_id);
                self.dispatch_last_env = env;
                Ok(())
            }
            (polka::DISPATCH_ID, polka::DISPATCH_PORT_POP_HANDLER) => {
                let frame = self.handlers.pop()
                    .ok_or("dispatch.pop_handler: no active handler frame")?;
                let n_cells = frame.cells_allocated.len();
                frame.release_cells(&mut self.heap, &mut self.region_table)?;
                self.trace_frame_event("HANDLER pop", format_args!("released {} cont cells", n_cells));
                Ok(())
            }
            (polka::DISPATCH_ID, polka::DISPATCH_PORT_RETURN_FN) => {
                let fn_id = raw as i64;
                let handler = self.handlers.last_mut()
                    .ok_or("dispatch.return_fn: no active handler frame")?;
                handler.pending_return_arm_fn = Some(fn_id as usize);
                Ok(())
            }
            (polka::DISPATCH_ID, polka::DISPATCH_PORT_RETURN_ENV) => {
                let handler = self.handlers.last_mut()
                    .ok_or("dispatch.return_env: no active handler frame")?;
                handler.pending_return_arm_env = raw;
                handler.pending_return_arm_env_is_handle = is_handle;
                Ok(())
            }
            (polka::REGION_ID, polka::REGION_PORT_PUSH) => { self.region_push(); Ok(()) }
            (polka::REGION_ID, polka::REGION_PORT_POP)  => self.region_pop(),
            (polka::REGION_ID, polka::REGION_PORT_FORGET) => {
                if is_handle && raw != HANDLE_NONE {
                    let (slot, gen_) = Self::decode_handle(raw);
                    let mut visited: std::collections::BTreeSet<(u32, u32)> = std::collections::BTreeSet::new();
                    self.deep_forget(slot, gen_, &mut visited)?;
                }
                Ok(())
            }
            (polka::REGION_ID, _) => Err(format!("region: unknown port {:#x}", port)),
            (polka::MODULE_ID, polka::MODULE_PORT_TABLE) => {
                // Module storage takes ownership of one rc; release the prior
                // table if being replaced.
                if self.module_table_is_handle && self.module_table_raw != HANDLE_NONE {
                    self.rc_dec_handle(self.module_table_raw)?;
                }
                if is_handle && raw != HANDLE_NONE { self.rc_inc_handle(raw)?; }
                self.module_table_raw = raw;
                self.module_table_is_handle = is_handle;
                Ok(())
            }
            _ => {
                if is_handle { self.rc_inc_handle(raw)?; }
                let dev = self.devices.get_mut(device_id)
                    .ok_or_else(|| format!("deo: device {:#04x} not installed", device_id))?;
                dev.write(port, Value::from_raw(raw), is_handle, &mut self.heap)
            }
        }
    }

    pub(super) fn do_resume(&mut self, module: &Module, dest_reg: Register, val_reg: Register) -> Result<(), String> {
        let (cell_slot, cell_gen) = {
            let h = self.handlers.last()
                .ok_or("Resume outside an active handler frame")?;
            (h.cell_slot, h.cell_gen)
        };
        let (alive_raw, saved_base, saved_func, suspended_snapshot) = {
            let data = self.heap.cell_data(cell_slot, cell_gen)?;
            let mask = self.heap.cell_mask(cell_slot, cell_gen)?;
            debug_assert!(data.len() >= crate::cont_slot::SIZE);
            let alive = data[crate::cont_slot::ALIVE];
            let saved_base = data[crate::cont_slot::SUSPEND_BASE] as usize;
            let saved_func = data[crate::cont_slot::SUSPEND_FUNC] as usize;
            let snap_raw = data[crate::cont_slot::REGS_SNAPSHOT_SLOT];
            let snap_is_handle = mask_bit(mask, crate::cont_slot::REGS_SNAPSHOT_SLOT);
            let snap = if !snap_is_handle || snap_raw == HANDLE_NONE {
                crate::snapshot::SnapshotHandle::EMPTY
            } else {
                let (slot, generation) = Self::decode_handle(snap_raw);
                let count = data[crate::cont_slot::REGS_COUNT] as usize;
                crate::snapshot::SnapshotHandle { slot, generation, count }
            };
            (alive, saved_base, saved_func, snap)
        };
        match alive_raw {
            1 => {}
            0 => return Err("resume: continuation already consumed".to_string()),
            n => return Err(format!("resume: continuation cell corrupted (alive slot = {})", n)),
        }

        self.heap.st(cell_slot, cell_gen, crate::cont_slot::ALIVE, 0, false)?;

        let (val_raw, val_is_handle) = self.read(val_reg)?;

        let arm_call_frame = self.frames.pop()
            .ok_or("resume: no arm-call frame on stack")?;
        let yield_result_abs = arm_call_frame.dest_reg;
        let resume_ip = arm_call_frame.ip;

        let arm_reg_count = self.current_fn_reg_count(module);
        let arm_snapshot = self.snapshot_registers_off_region(self.base_reg, arm_reg_count)?;

        let arm_resume_dest_abs = self.base_reg + dest_reg.to_usize();
        let insert_at = {
            let body_idx = self.handlers.last().and_then(|h| h.body_frame_index)
                .ok_or("resume: active handler has no body frame index")?;
            let mut i = body_idx + 1;
            while i < self.frames.len() && self.frames[i].is_arm_continuation() {
                i += 1;
            }
            i
        };
        let new_frame = super::Frame::arm_continuation(
            self.current_func,
            self.pc,
            self.base_reg,
            arm_resume_dest_abs,
            arm_snapshot.slot,
            arm_snapshot.generation,
            arm_snapshot.count,
        );
        if insert_at == self.frames.len() {
            self.frames.push(new_frame);
        } else {
            self.frames.insert(insert_at, new_frame);
        }

        self.base_reg = saved_base;
        self.current_func = saved_func;
        self.pc = resume_ip;
        self.restore_registers(saved_base, suspended_snapshot)?;

        if !suspended_snapshot.is_empty() {
            self.heap.st(cell_slot, cell_gen, crate::cont_slot::REGS_SNAPSHOT_SLOT, HANDLE_NONE, true)?;
            self.heap.rc_dec(suspended_snapshot.slot, suspended_snapshot.generation)?;
        }

        if yield_result_abs >= self.registers.len() {
            return Err(format!(
                "resume: yield dest abs {} out of registers (len {})",
                yield_result_abs, self.registers.len()));
        }
        self.write_abs(yield_result_abs, val_raw, val_is_handle);

        if self.debug_sink.is_some() {
            let event = DebugEvent::Resume {
                saved_pc: resume_ip,
                saved_base,
                cell_dest: yield_result_abs - saved_base,
                val: Value::from_raw(val_raw),
                handler_dest: dest_reg.to_usize(),
                alive: Value::from_raw(alive_raw),
                depth: self.handlers.len(),
            };
            self.emit_debug(&event);
        }
        Ok(())
    }

    pub(crate) fn deep_forget(
        &mut self,
        slot: u32,
        generation: u32,
        visited: &mut std::collections::BTreeSet<(u32, u32)>,
    ) -> Result<(), String> {
        if !visited.insert((slot, generation)) { return Ok(()); }
        if !self.region_table.forget(slot, generation) { return Ok(()); }
        let size = match self.heap.size(slot, generation) {
            Ok(n) => n,
            Err(_) => return Ok(()),
        };
        for off in 0..size {
            if let Ok((raw, is_handle)) = self.heap.ld(slot, generation, off) {
                if is_handle && raw != HANDLE_NONE {
                    let (s, g) = Self::decode_handle(raw);
                    self.deep_forget(s, g, visited)?;
                }
            }
        }
        Ok(())
    }
}
