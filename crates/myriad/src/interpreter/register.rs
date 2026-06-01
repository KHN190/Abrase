use polka::{BytecodeChunk, Chunk, Register, Module, FRAME_REGS, HANDLE_NONE};
use crate::memory::mask_bit;
use crate::value::alloc_string;
use super::super::VirtualMachine;
use super::MAX_RAM;


impl VirtualMachine {
    pub(crate) fn branch(&mut self, bc: &BytecodeChunk, offset: i16) -> Result<(), String> {
        let new_pc = (self.pc as isize) + (offset as isize);
        if new_pc < 0 || (new_pc as usize) > bc.code.len() {
            return Err(format!(
                "branch: pc {} out of range [0, {}]",
                new_pc, bc.code.len()
            ));
        }
        self.pc = new_pc as usize;
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn abs(&self, r: Register) -> usize {
        let abs = self.base_reg + r.to_usize();
        debug_assert!(abs < self.registers.len(),
            "r{} (abs {}) out of register window (len {})",
            r.0, abs, self.registers.len());
        abs
    }

    #[inline(always)]
    pub(crate) fn ensure_registers(&mut self, needed: usize) {
        if needed > self.registers.len() {
            self.registers.resize(needed, HANDLE_NONE);
        }
        let mask_words_needed = (needed + 63) / 64;
        if mask_words_needed > self.register_mask.len() {
            self.register_mask.resize(mask_words_needed, 0);
        }
    }

    #[inline(always)]
    pub(crate) fn read_abs_raw(&self, abs: usize) -> u64 {
        debug_assert!(abs < self.registers.len());
        unsafe { *self.registers.get_unchecked(abs) }
    }

    #[inline(always)]
    pub(crate) fn write_abs_raw(&mut self, abs: usize, v: u64) {
        debug_assert!(abs < self.registers.len());
        unsafe { *self.registers.get_unchecked_mut(abs) = v; }
    }

    #[inline(always)]
    pub(crate) fn reg_mask_bit(&self, abs: usize) -> bool {
        let w = abs / 64;
        let b = abs % 64;
        self.register_mask.get(w).map_or(false, |x| (x >> b) & 1 == 1)
    }

    #[inline(always)]
    pub(crate) fn set_reg_mask_bit(&mut self, abs: usize, on: bool) {
        let w = abs / 64;
        let b = abs % 64;
        if let Some(x) = self.register_mask.get_mut(w) {
            if on { *x |= 1u64 << b; } else { *x &= !(1u64 << b); }
        }
    }

    #[inline(always)]
    pub(crate) fn write_abs(&mut self, abs: usize, raw: u64, is_handle: bool) {
        self.write_abs_raw(abs, raw);
        self.set_reg_mask_bit(abs, is_handle);
    }

    #[inline(always)]
    pub(crate) fn take_abs(&mut self, abs: usize) -> (u64, bool) {
        let v = self.read_abs_raw(abs);
        let h = self.reg_mask_bit(abs);
        self.write_abs_raw(abs, HANDLE_NONE);
        if h { self.set_reg_mask_bit(abs, false); }
        (v, h)
    }

    #[inline(always)]
    pub(crate) fn read(&self, r: Register) -> Result<(u64, bool), String> {
        let abs = self.abs(r);
        let v = self.read_abs_raw(abs);
        Ok((v, self.reg_mask_bit(abs)))
    }

    #[inline(always)]
    pub(crate) fn read_at(&self, r: Register) -> (u64, bool) {
        let abs = self.abs(r);
        (self.read_abs_raw(abs), self.reg_mask_bit(abs))
    }

    #[inline(always)]
    pub(crate) fn read_raw(&self, r: Register) -> Result<u64, String> {
        Ok(self.read_abs_raw(self.abs(r)))
    }

    #[inline(always)]
    pub(crate) fn take(&mut self, r: Register) -> Result<(u64, bool), String> {
        let abs = self.abs(r);
        let v = self.read_abs_raw(abs);
        let h = self.reg_mask_bit(abs);
        self.write_abs_raw(abs, HANDLE_NONE);
        self.set_reg_mask_bit(abs, false);
        Ok((v, h))
    }

    #[inline(always)]
    pub(crate) fn write(&mut self, r: Register, raw: u64, is_handle: bool) -> Result<(), String> {
        let abs = self.abs(r);
        self.write_abs(abs, raw, is_handle);
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn read_i64(&self, r: Register) -> Result<i64, String> {
        Ok(self.read_raw(r)? as i64)
    }

    #[inline(always)]
    pub(crate) fn read_f64(&self, r: Register) -> Result<f64, String> {
        let raw = self.read_raw(r)?;
        if self.int32_safe {
            Ok(f32::from_bits(raw as u32) as f64)
        } else {
            Ok(f64::from_bits(raw))
        }
    }

    #[inline(always)]
    pub(crate) fn narrow_float_bits(&self, v: f64) -> u64 {
        if self.int32_safe {
            (v as f32).to_bits() as u64
        } else {
            v.to_bits()
        }
    }

    #[inline(always)]
    pub(crate) fn read_handle(&self, r: Register) -> Result<(u32, u32), String> {
        let abs = self.abs(r);
        let v = self.read_abs_raw(abs);
        if !self.reg_mask_bit(abs) {
            return Err(format!("expected handle, got plain {:#x} in r{}", v, r.0));
        }
        if v == HANDLE_NONE {
            return Err(format!("expected handle, got None in r{}", r.0));
        }
        Ok(Self::decode_handle(v))
    }

    #[inline(always)]
    pub(crate) fn decode_handle(raw: u64) -> (u32, u32) {
        (((raw >> 24) & 0x00FF_FFFF) as u32, (raw & 0x00FF_FFFF) as u32)
    }

    pub(crate) fn check_handle_tags(&self, where_: &str) -> Result<(), String> {
        for abs in 0..self.registers.len() {
            if !self.reg_mask_bit(abs) { continue; }
            let raw = self.read_abs_raw(abs);
            if raw == HANDLE_NONE { continue; }
            let (s, g) = Self::decode_handle(raw);
            if !self.heap.is_live(s, g) {
                return Err(format!(
                    "handle-tag check [{}]: register {} tagged handle but points to dead/stale slot {} gen {}",
                    where_, abs, s, g));
            }
        }
        for (slot, gen_, _rc, data, handles) in self.heap.live_cells() {
            for (i, h) in handles.iter().enumerate() {
                if !*h || data[i] == HANDLE_NONE { continue; }
                let (s, g) = Self::decode_handle(data[i]);
                if !self.heap.is_live(s, g) {
                    return Err(format!(
                        "handle-tag check [{}]: cell slot {} gen {} offset {} tagged handle but points to dead slot {} gen {}",
                        where_, slot, gen_, i, s, g));
                }
            }
        }
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn rc_inc_handle(&mut self, raw: u64) -> Result<(), String> {
        if raw == HANDLE_NONE { return Ok(()); }
        let (s, g) = Self::decode_handle(raw);
        self.heap.rc_inc(s, g)
    }

    #[inline(always)]
    pub(crate) fn rc_dec_handle(&mut self, raw: u64) -> Result<(), String> {
        if raw == HANDLE_NONE { return Ok(()); }
        let (s, g) = Self::decode_handle(raw);
        self.heap.rc_dec(s, g)?;
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn mem_used(&self) -> usize {
        self.heap.bytes_used()
    }

    pub(crate) fn mem_charge(&mut self, bytes: usize) -> Result<(), String> {
        let projected = self.mem_used().saturating_add(bytes);
        if projected <= MAX_RAM { return Ok(()); }
        Err(format!(
            "out of memory: requested {} bytes, {} / {} already in use",
            bytes, self.mem_used(), MAX_RAM
        ))
    }

    #[inline(always)]
    pub(crate) fn checked_heap_alloc(&mut self, size: usize) -> Result<(u32, u32), String> {
        self.mem_charge(size.saturating_mul(8))?;
        self.heap.try_alloc(size)
    }

    #[inline(always)]
    pub(crate) fn checked_heap_alloc_with_mask(&mut self, size: usize, init_mask: &[u64]) -> Result<(u32, u32), String> {
        self.mem_charge(size.saturating_mul(8))?;
        self.heap.try_alloc_with_mask(size, init_mask)
    }

    #[inline(always)]
    pub(crate) fn bin_i64<F: Fn(i64, i64) -> i64>(&mut self, d: Register, a: Register, b: Register, f: F) -> Result<(), String> {
        let x = self.read_i64(a)?;
        let y = self.read_i64(b)?;
        self.write(d, f(x, y) as u64, false)
    }

    #[inline(always)]
    pub(crate) fn bin_f64<F: Fn(f64, f64) -> f64>(&mut self, d: Register, a: Register, b: Register, f: F) -> Result<(), String> {
        let x = self.read_f64(a)?;
        let y = self.read_f64(b)?;
        let bits = self.narrow_float_bits(f(x, y));
        self.write(d, bits, false)
    }

    #[inline(always)]
    pub(crate) fn bin_i64_checked<F: Fn(i64, i64) -> Option<i64>>(
        &mut self, d: Register, a: Register, b: Register, msg: &str, f: F,
    ) -> Result<(), String> {
        let x = self.read_i64(a)?;
        let y = self.read_i64(b)?;
        let r = f(x, y).ok_or_else(|| msg.to_string())?;
        self.write(d, r as u64, false)
    }

    #[inline(always)]
    pub(crate) fn bin_eq(&mut self, d: Register, a: Register, b: Register, negate: bool) -> Result<(), String> {
        let xa = self.read_raw(a)?;
        let xb = self.read_raw(b)?;
        let r = (xa == xb) ^ negate;
        self.write(d, bool_u64(r), false)
    }

    #[inline(always)]
    pub(crate) fn bin_i64_cmp<F: Fn(i64, i64) -> bool>(&mut self, d: Register, a: Register, b: Register, f: F) -> Result<(), String> {
        let x = self.read_i64(a)?;
        let y = self.read_i64(b)?;
        self.write(d, bool_u64(f(x, y)), false)
    }

    pub(super) fn resolve_constants(&mut self, module: &Module) -> Result<(), String> {
        self.resolved_constants.clear();
        self.resolved_const_mask.clear();
        self.resolved_constants.reserve(module.functions.len());
        self.resolved_const_mask.reserve(module.functions.len());
        for chunk in &module.functions {
            let (vals, mask): (Vec<u64>, Vec<u64>) = match chunk {
                Chunk::Bytecode(bc) => {
                    let mut vals = bc.constants.clone();
                    let mask = bc.const_mask.clone();
                    // For each handle-bit constant, resolve the string-pool index it
                    // carries into an actual heap handle (rc=1, module-lifetime).
                    for i in 0..vals.len() {
                        if !mask_bit(&mask, i) { continue; }
                        let sidx = vals[i] as usize;
                        let s = bc.string_constants.get(sidx)
                            .cloned().unwrap_or_default();
                        let needed = s.len() + 16; // crude upper bound
                        if self.mem_used().saturating_add(needed) > MAX_RAM {
                            return Err(format!(
                                "out of memory at module load: string constant {} bytes",
                                s.len()
                            ));
                        }
                        let v = alloc_string(&mut self.heap, &s)?;
                        vals[i] = v.raw();
                        let (slot, gen_) = v.as_handle();
                        self.string_const_handles.push((slot, gen_));
                    }
                    (vals, mask)
                }
                Chunk::Native(_) => (Vec::new(), Vec::new()),
            };
            self.resolved_constants.push(vals);
            self.resolved_const_mask.push(mask);
        }
        self.resolved_natives.clear();
        self.resolved_natives.reserve(module.functions.len());
        for chunk in &module.functions {
            let entry = match chunk {
                Chunk::Native(n) => self.natives.get(&n.name).cloned(),
                Chunk::Bytecode(_) => None,
            };
            self.resolved_natives.push(entry);
        }
        Ok(())
    }

    pub(super) fn resolve_dispatch(&self, key: u16) -> (u16, Option<(u64, bool)>) {
        let effect_id = (key >> 8) as u16;
        let op_id = (key & 0xFF) as usize;
        for h in self.handlers.iter().rev() {
            if h.effect_id != effect_id { continue; }
            if let Some(slot) = h.dispatch_table_slot {
                if let Ok((raw, _)) = self.heap.ld(slot, h.dispatch_table_gen, op_id * 2) {
                    let n = raw as i64;
                    if (0..=0xFFFF).contains(&n) {
                        let env = self.heap.ld(slot, h.dispatch_table_gen, op_id * 2 + 1).ok();
                        return (n as u16, env);
                    }
                }
            }
        }
        (polka::DISPATCH_NO_MATCH, None)
    }
}

#[inline(always)]
pub(crate) fn bool_u64(b: bool) -> u64 { if b { 1 } else { 0 } }

pub(crate) fn decode_dispatch_key(raw: u64) -> Result<u16, String> {
    let n = raw as i64;
    if (0..=0xFFFF).contains(&n) { Ok(n as u16) }
    else { Err(format!("dispatch.lookup: bad key {}", n)) }
}

pub(crate) fn split_port(port_val: i64) -> Result<(u8, u8), String> {
    if !(0..=0xFFFF).contains(&port_val) {
        return Err(format!("device port {:#x} out of 16-bit range", port_val));
    }
    let device_id = ((port_val >> 8) & 0xFF) as u8;
    let port = (port_val & 0xFF) as u8;
    Ok((device_id, port))
}

pub(crate) fn validate_module_register_budget(module: &Module) -> Result<(), String> {
    for (i, chunk) in module.functions.iter().enumerate() {
        if let Chunk::Bytecode(b) = chunk {
            if b.reg_count > FRAME_REGS {
                return Err(format!(
                    "module load: fn {} has reg_count {} > frame budget {}",
                    i, b.reg_count, FRAME_REGS
                ));
            }
            if b.param_count > b.reg_count {
                return Err(format!(
                    "module load: fn {} has param_count {} > reg_count {}",
                    i, b.param_count, b.reg_count
                ));
            }
        }
    }
    Ok(())
}
