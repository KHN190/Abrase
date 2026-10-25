use super::super::runtime;
use runtime::{Word, Int, Float, HANDLE_NONE};
use alloc::{string::{String, ToString}, vec::Vec};
use polka::{BytecodeChunk, Chunk, Register, Module, OpCode, FRAME_REGS};
use runtime::value::{alloc_string, alloc_bytes};
use runtime::memory::mask_bit;
use super::super::VirtualMachine;
use super::MAX_RAM;


macro_rules! eval_integer {
    ($narrow:expr, $left:expr, $right:expr, |$x:ident, $y:ident| $body:expr) => {{
        let ($x, $y) = ($left as Int, $right as Int);
        $body
    }};
}

macro_rules! eval_float {
    ($narrow:expr, $left:expr, $right:expr, |$x:ident, $y:ident| $body:expr) => {{
        let ($x, $y) = (Float::from_bits($left), Float::from_bits($right));
        $body
    }};
}

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
        let mask_words_needed = needed.div_ceil(Word::BITS as usize);
        if mask_words_needed > self.register_mask.len() {
            self.register_mask.resize(mask_words_needed, 0);
        }
    }

    #[inline(always)]
    pub(crate) fn read_abs_raw(&self, abs: usize) -> Word {
        debug_assert!(abs < self.registers.len());
        unsafe { *self.registers.get_unchecked(abs) }
    }

    #[inline(always)]
    pub(crate) fn write_abs_raw(&mut self, abs: usize, v: Word) {
        debug_assert!(abs < self.registers.len());
        unsafe { *self.registers.get_unchecked_mut(abs) = v; }
    }

    #[inline(always)]
    pub(crate) fn reg_mask_bit(&self, abs: usize) -> bool {
        let w = abs / Word::BITS as usize;
        let b = abs % Word::BITS as usize;
        self.register_mask.get(w).map_or(false, |x| (x >> b) & 1 == 1)
    }

    #[inline(always)]
    pub(crate) fn set_reg_mask_bit(&mut self, abs: usize, on: bool) {
        let w = abs / Word::BITS as usize;
        let b = abs % Word::BITS as usize;
        if let Some(x) = self.register_mask.get_mut(w) {
            if on { *x |= (1 as Word) << b; } else { *x &= !((1 as Word) << b); }
        }
    }

    #[inline(always)]
    pub(crate) fn write_abs(&mut self, abs: usize, raw: Word, is_handle: bool) {
        self.write_abs_raw(abs, raw);
        self.set_reg_mask_bit(abs, is_handle);
    }

    #[inline(always)]
    pub(crate) fn take_abs(&mut self, abs: usize) -> (Word, bool) {
        let v = self.read_abs_raw(abs);
        let h = self.reg_mask_bit(abs);
        self.write_abs_raw(abs, HANDLE_NONE);
        if h { self.set_reg_mask_bit(abs, false); }
        (v, h)
    }

    #[inline(always)]
    pub(crate) fn read(&self, r: Register) -> Result<(Word, bool), String> {
        let abs = self.abs(r);
        let v = self.read_abs_raw(abs);
        Ok((v, self.reg_mask_bit(abs)))
    }

    #[inline(always)]
    pub(crate) fn read_at(&self, r: Register) -> (Word, bool) {
        let abs = self.abs(r);
        (self.read_abs_raw(abs), self.reg_mask_bit(abs))
    }

    #[inline(always)]
    pub(crate) fn read_raw(&self, r: Register) -> Result<Word, String> {
        Ok(self.read_abs_raw(self.abs(r)))
    }

    #[inline(always)]
    pub(crate) fn take(&mut self, r: Register) -> Result<(Word, bool), String> {
        let abs = self.abs(r);
        let v = self.read_abs_raw(abs);
        let h = self.reg_mask_bit(abs);
        self.write_abs_raw(abs, HANDLE_NONE);
        self.set_reg_mask_bit(abs, false);
        Ok((v, h))
    }

    #[inline(always)]
    pub(crate) fn write(&mut self, r: Register, raw: Word, is_handle: bool) -> Result<(), String> {
        let abs = self.abs(r);
        self.write_abs(abs, raw, is_handle);
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn read_int(&self, r: Register) -> Result<Int, String> {
        Ok(self.read_raw(r)? as Int)
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
    pub(crate) fn decode_handle(raw: Word) -> (u32, u32) {
        runtime::memory::handle_parts(raw)
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
    pub(crate) fn rc_inc_handle(&mut self, raw: Word) -> Result<(), String> {
        if raw == HANDLE_NONE { return Ok(()); }
        let (s, g) = Self::decode_handle(raw);
        self.heap.rc_inc(s, g)
    }

    #[inline(always)]
    pub(crate) fn rc_dec_handle(&mut self, raw: Word) -> Result<(), String> {
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
    pub(crate) fn checked_heap_alloc_with_mask(&mut self, size: usize, init_mask: &[Word]) -> Result<(u32, u32), String> {
        self.mem_charge(size.saturating_mul(8))?;
        self.heap.try_alloc_with_mask(size, init_mask)
    }

    #[inline(always)]
    pub(crate) fn exec_integer_op(&mut self, op: &OpCode) -> Result<(), String> {
        use OpCode::*;
        let (d, a, b) = match *op {
            Add(d,a,b)|Sub(d,a,b)|Mul(d,a,b)|Div(d,a,b)|Mod(d,a,b)
            |Lt(d,a,b)|Gt(d,a,b)|Lte(d,a,b)|Gte(d,a,b)
            |And(d,a,b)|Or(d,a,b)|Xor(d,a,b)|Shl(d,a,b)|Shr(d,a,b) => (d,a,b),
            Neg(d,a)|AddImm(d,a,_)|SubImm(d,a,_) => (d,a,a),
            _ => unreachable!("integer opcode"),
        };
        let (left, right) = (self.read_raw(a)?, self.read_raw(b)?);
        let raw = eval_integer!(self.int32_safe, left, right, |x, y| match *op {
            Add(..) => x.wrapping_add(y) as Int as Word,
            Sub(..) => x.wrapping_sub(y) as Int as Word,
            Mul(..) => x.wrapping_mul(y) as Int as Word,
            Div(..) => x.checked_div(y).ok_or_else(|| "div by zero".to_string())? as Int as Word,
            Mod(..) => x.checked_rem(y).ok_or_else(|| "mod by zero".to_string())? as Int as Word,
            Neg(..) => x.wrapping_neg() as Int as Word,
            AddImm(_,_,imm) => x.wrapping_add(imm as _) as Int as Word,
            SubImm(_,_,imm) => x.wrapping_sub(imm as _) as Int as Word,
            Lt(..) => bool_word(x < y),
            Gt(..) => bool_word(x > y),
            Lte(..) => bool_word(x <= y),
            Gte(..) => bool_word(x >= y),
            And(..) => (x & y) as Int as Word,
            Or(..) => (x | y) as Int as Word,
            Xor(..) => (x ^ y) as Int as Word,
            Shl(..) => x.wrapping_shl(y as u32) as Int as Word,
            Shr(..) => x.wrapping_shr(y as u32) as Int as Word,
            _ => unreachable!("integer opcode"),
        });
        self.write(d, raw, false)
    }

    #[inline(always)]
    pub(crate) fn exec_float_op(&mut self, op: &OpCode) -> Result<(), String> {
        use OpCode::*;
        let (d, a, b) = match *op {
            FAdd(d,a,b)|FSub(d,a,b)|FMul(d,a,b)|FDiv(d,a,b)|FLt(d,a,b)|FEq(d,a,b) => (d,a,b),
            FNeg(d,a) => (d,a,a),
            _ => unreachable!("float opcode"),
        };
        let (left, right) = (self.read_raw(a)?, self.read_raw(b)?);
        let raw = eval_float!(self.int32_safe, left, right, |x, y| match *op {
            FAdd(..) => (x + y).to_bits() as Word,
            FSub(..) => (x - y).to_bits() as Word,
            FMul(..) => (x * y).to_bits() as Word,
            FDiv(..) => (x / y).to_bits() as Word,
            FNeg(..) => (-x).to_bits() as Word,
            FLt(..) => bool_word(x < y),
            FEq(..) => bool_word(x == y),
            _ => unreachable!("float opcode"),
        });
        self.write(d, raw, false)
    }

    #[inline(always)]
    pub(crate) fn bin_eq(&mut self, d: Register, a: Register, b: Register, negate: bool) -> Result<(), String> {
        let xa = self.read_raw(a)?;
        let xb = self.read_raw(b)?;
        let r = (xa == xb) ^ negate;
        self.write(d, bool_word(r), false)
    }

    pub(super) fn resolve_constants(&mut self, module: &Module) -> Result<(), String> {
        self.resolved_constants.clear();
        self.resolved_const_mask.clear();
        self.resolved_constants.reserve(module.functions.len());
        self.resolved_const_mask.reserve(module.functions.len());
        for chunk in &module.functions {
            let (vals, mask): (Vec<Word>, Vec<Word>) = match chunk {
                Chunk::Bytecode(bc) => {
                    let mut vals = Vec::with_capacity(bc.constants.len());
                    let mut mask = vec![0 as Word; bc.constants.len().div_ceil(Word::BITS as usize)];
                    for i in 0..bc.constants.len() {
                        if bc.const_is_handle(i as u16) {
                            mask[i / Word::BITS as usize] |= (1 as Word) << (i % Word::BITS as usize);
                        }
                    }
                    for (i, &constant) in bc.constants.iter().enumerate() {
                        if !mask_bit(&mask, i) {
                            vals.push(constant_word(constant)?);
                            continue;
                        }
                        let (is_bytes, index) = constant_pool(constant)?;
                        let v = if is_bytes {
                            let b = bc.bytes_constants.get(index).cloned().unwrap_or_default();
                            if self.mem_used().saturating_add(b.len() + 16) > MAX_RAM {
                                return Err(format!("out of memory at module load: bytes constant {} bytes", b.len()));
                            }
                            alloc_bytes(&mut self.heap, &b)?
                        } else {
                            let s = bc.string_constants.get(index).cloned().unwrap_or_default();
                            if self.mem_used().saturating_add(s.len() + 16) > MAX_RAM {
                                return Err(format!("out of memory at module load: string constant {} bytes", s.len()));
                            }
                            alloc_string(&mut self.heap, &s)?
                        };
                        vals.push(v.raw());
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
        self.resolved_aot.clear();
        self.resolved_aot.reserve(module.functions.len());
        for chunk in &module.functions {
            let (entry, aot) = match chunk {
                Chunk::Native(n) => match self.aot_fns.get(&n.name).cloned() {
                    Some(f) => (None, Some(f)),
                    None => (Some(self.natives.get(&n.name).cloned()
                        .ok_or_else(|| format!("unresolved import: {}", n.name))?), None),
                },
                Chunk::Bytecode(_) => (None, None),
            };
            self.resolved_natives.push(entry);
            self.resolved_aot.push(aot);
        }
        Ok(())
    }

    pub(super) fn resolve_dispatch(&self, key: u16) -> (u16, Option<(Word, bool)>) {
        let effect_id = (key >> 8) as u16;
        let op_id = (key & 0xFF) as usize;
        for h in self.handlers.iter().rev() {
            if h.effect_id != effect_id { continue; }
            if let Some(slot) = h.dispatch_table_slot {
                if let Ok((raw, _)) = self.heap.ld(slot, h.dispatch_table_gen, op_id * 2) {
                    let n = raw as Int;
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
pub(crate) fn bool_word(b: bool) -> Word { if b { 1 } else { 0 } }

pub(crate) fn decode_dispatch_key(raw: Word) -> Result<u16, String> {
    let n = raw as Int;
    if (0..=0xFFFF).contains(&n) { Ok(n as u16) }
    else { Err(format!("dispatch.lookup: bad key {}", n)) }
}

pub(crate) fn split_port(port_val: Int) -> Result<(u8, u8), String> {
    if !(0..=0xFFFF).contains(&port_val) {
        return Err(format!("device port {:#x} out of 16-bit range", port_val));
    }
    let device_id = ((port_val >> 8) & 0xFF) as u8;
    let port = (port_val & 0xFF) as u8;
    Ok((device_id, port))
}

pub(crate) fn max_reg(op: &polka::OpCode) -> Option<u8> {
    use polka::OpCode::*;
    let m3 = |a: &Register, b: &Register, c: &Register| a.0.max(b.0).max(c.0);
    let m2 = |a: &Register, b: &Register| a.0.max(b.0);
    match op {
        Add(a,b,c)|Sub(a,b,c)|Mul(a,b,c)|Div(a,b,c)|Mod(a,b,c)
        |FAdd(a,b,c)|FSub(a,b,c)|FMul(a,b,c)|FDiv(a,b,c)|FLt(a,b,c)|FEq(a,b,c)
        |Eq(a,b,c)|Neq(a,b,c)|Lt(a,b,c)|Gt(a,b,c)|Lte(a,b,c)|Gte(a,b,c)
        |And(a,b,c)|Or(a,b,c)|Xor(a,b,c)|Shl(a,b,c)|Shr(a,b,c)
        |LdIdx(a,b,c)|StIdx(a,b,c)|Raise(a,b,c) => Some(m3(a,b,c)),
        Neg(a,b)|FNeg(a,b)|CallReg(a,b)|Copy(a,b)|Move(a,b)|Dei(a,b)|Deo(a,b)|Resume(a,b)
        |Ld(a,b,_)|St(a,b,_)|AddImm(a,b,_)|SubImm(a,b,_) => Some(m2(a,b)),
        Ret(a)|Drop(a)|Call(a,_)|PushConst(a,_)|Alloc(a,_)|Handle(a,_)|Jz(a,_)|Jnz(a,_) => Some(a.0),
        Jmp(_) => None,
    }
}


fn constant_word(raw: polka::Constant) -> Result<Word, String> {
    runtime::decode_constant(raw).map_err(String::from)
}

fn constant_pool(raw: polka::Constant) -> Result<(bool, usize), String> {
    Ok((polka::constant_is_bytes(raw), polka::constant_pool_index(raw).map_err(String::from)?))
}

pub(crate) fn validate_module_profile(module: &Module) -> Result<(), String> {
    if (module.flags & polka::CART_FLAG_INT32_SAFE != 0) != runtime::IS_INT32 {
        return Err(String::from("module numeric profile mismatch: INT32_SAFE requires word32"));
    }
    Ok(())
}

pub(crate) fn validate_module_register_budget(module: &Module) -> Result<(), String> {
    validate_module_profile(module)?;
    for (i, chunk) in module.functions.iter().enumerate() {
        if let Chunk::Bytecode(b) = chunk {
            if b.constants.len() > u16::MAX as usize {
                return Err(String::from("module constant pool exceeds format limit"));
            }
            for (index, &constant) in b.constants.iter().enumerate() {
                if b.const_is_handle(index as u16) {
                    constant_pool(constant)?;
                } else {
                    constant_word(constant)?;
                }
            }
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
            // Memory-safety bound is the physical frame window (FRAME_REGS), not
            // reg_count: ensure_registers reserves FRAME_REGS+slack per frame, so any
            // r < FRAME_REGS is in-bounds. (r in [reg_count, FRAME_REGS) only clobbers
            // a not-yet-pushed callee frame — cart-correctness, not a memory hole.)
            for (pc, op) in b.code.iter().enumerate() {
                if let Some(r) = max_reg(op) {
                    if (r as usize) >= FRAME_REGS {
                        return Err(format!(
                            "module load: fn {} op {} references r{} >= frame budget {}",
                            i, pc, r, FRAME_REGS
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}
