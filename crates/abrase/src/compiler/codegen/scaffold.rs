// Low-level codegen helpers: register allocation, opcodes, constants, jumps, result-wrapping.

use crate::bytecode::{OpCode, Register, FRAME_REGS};
use crate::compiler::Compiler;
use crate::compiler::effects;
use crate::myriad::Value;

pub(in crate::compiler) fn to_u16(n: usize, what: &str) -> Result<u16, String> {
    u16::try_from(n).map_err(|_| format!("{} exceeds u16 range (got {}, max {})", what, n, u16::MAX))
}

pub(in crate::compiler) fn to_u8(n: usize, what: &str) -> Result<u8, String> {
    u8::try_from(n).map_err(|_| format!("{} exceeds u8 range (got {}, max {})", what, n, u8::MAX))
}

impl Compiler {
    pub(in crate::compiler) fn alloc_register(&mut self) -> Result<Register, String> {
        if (self.next_reg as usize) >= FRAME_REGS {
            return Err(format!(
                "Register overflow (per-frame budget is {})",
                FRAME_REGS
            ));
        }
        let reg = Register(self.next_reg as u8);
        self.next_reg += 1;
        Ok(reg)
    }

    pub(in crate::compiler) fn emit(&mut self, op: OpCode) {
        self.code.push(op);
    }

    // Peephole: if the last emitted op writes to `old`, retarget it to `new`.
    pub(in crate::compiler) fn try_redirect_last_dest(&mut self, old: Register, new: Register) -> bool {
        if old == new { return true; }
        let Some(last) = self.code.last_mut() else { return false; };
        let d: &mut Register = match last {
            OpCode::Add(d, _, _) | OpCode::Sub(d, _, _) | OpCode::Mul(d, _, _) |
            OpCode::Div(d, _, _) | OpCode::Mod(d, _, _) | OpCode::Neg(d, _) |
            OpCode::FAdd(d, _, _) | OpCode::FSub(d, _, _) | OpCode::FMul(d, _, _) | OpCode::FDiv(d, _, _) |
            OpCode::Eq(d, _, _) | OpCode::Neq(d, _, _) |
            OpCode::Lt(d, _, _) | OpCode::Gt(d, _, _) | OpCode::Lte(d, _, _) | OpCode::Gte(d, _, _) |
            OpCode::FLt(d, _, _) |
            OpCode::And(d, _, _) | OpCode::Or(d, _, _) | OpCode::Xor(d, _, _) |
            OpCode::Shl(d, _, _) | OpCode::Shr(d, _, _) |
            OpCode::PushConst(d, _) |
            OpCode::Copy(d, _) | OpCode::Move(d, _) |
            OpCode::Ld(d, _, _) | OpCode::LdIdx(d, _, _) |
            OpCode::Ref(d, _) |
            OpCode::AddImm(d, _, _) | OpCode::SubImm(d, _, _) |
            OpCode::Alloc(d, _) |
            OpCode::Call(d, _) | OpCode::CallReg(d, _) => d,
            _ => return false,
        };
        if *d == old {
            *d = new;
            true
        } else {
            false
        }
    }

    pub(in crate::compiler) fn add_constant(&mut self, val: Value) -> Result<u16, String> {
        if let Some(i) = self.constants.iter().position(|c| *c == val) {
            return Ok(i as u16);
        }
        if self.constants.len() >= u16::MAX as usize {
            return Err("Constant pool overflow (max 65535 entries)".to_string());
        }
        self.constants.push(val);
        Ok((self.constants.len() - 1) as u16)
    }

    pub(in crate::compiler) fn add_string_constant(&mut self, s: &str) -> Result<u16, String> {
        let str_idx = if let Some(i) = self.string_constants.iter().position(|c| c == s) {
            i as u32
        } else {
            if self.string_constants.len() >= u32::MAX as usize {
                return Err("String constant pool overflow".to_string());
            }
            self.string_constants.push(s.to_string());
            (self.string_constants.len() - 1) as u32
        };
        self.add_constant(Value::from_str_const(str_idx))
    }

    pub(in crate::compiler) fn emit_region_push(&mut self) -> Result<(), String> {
        self.emit_region_marker(crate::bytecode::REGION_PORT_PUSH)
    }

    pub(in crate::compiler) fn emit_region_pop(&mut self) -> Result<(), String> {
        self.emit_region_marker(crate::bytecode::REGION_PORT_POP)
    }

    fn emit_region_marker(&mut self, port: u8) -> Result<(), String> {
        let port_val = ((crate::bytecode::REGION_ID as i64) << 8) | (port as i64);
        let val_idx = self.add_constant(Value::from_int(0))?;
        let port_idx = self.add_constant(Value::from_int(port_val))?;
        let val_reg = self.alloc_register()?;
        let port_reg = self.alloc_register()?;
        self.emit(OpCode::PushConst(val_reg, val_idx));
        self.emit(OpCode::PushConst(port_reg, port_idx));
        self.emit(OpCode::Deo(val_reg, port_reg));
        Ok(())
    }

    pub(in crate::compiler) fn rel_offset(&self, target_pc: usize, branch_pc: usize) -> Result<i16, String> {
        let off = target_pc as isize - (branch_pc as isize + 1);
        i16::try_from(off).map_err(|_| format!("Branch offset {} exceeds 16-bit range", off))
    }

    pub(in crate::compiler) fn patch_jz_at(&mut self, branch_pc: usize, target_pc: usize) -> Result<(), String> {
        let off = self.rel_offset(target_pc, branch_pc)?;
        if let OpCode::Jz(r, _) = self.code[branch_pc] {
            self.code[branch_pc] = OpCode::Jz(r, off);
        }
        Ok(())
    }

    pub(in crate::compiler) fn patch_jmp_at(&mut self, branch_pc: usize, target_pc: usize) -> Result<(), String> {
        let off = self.rel_offset(target_pc, branch_pc)?;
        if matches!(self.code[branch_pc], OpCode::Jmp(_)) {
            self.code[branch_pc] = OpCode::Jmp(off);
        }
        Ok(())
    }

    pub(in crate::compiler) fn wrap_ok(&mut self, value: Register) -> Result<Register, String> {
        self.wrap_result(value, effects::OK_TAG)
    }

    pub(in crate::compiler) fn wrap_err(&mut self, value: Register) -> Result<Register, String> {
        self.wrap_result(value, effects::ERR_TAG)
    }

    fn wrap_result(&mut self, value: Register, tag: u32) -> Result<Register, String> {
        let dest = self.alloc_register()?;
        self.emit(OpCode::Alloc(dest, 2));
        let tag_reg = self.alloc_register()?;
        let idx = self.add_constant(Value::from_int(tag as i64))?;
        self.emit(OpCode::PushConst(tag_reg, idx));
        self.emit(OpCode::St(tag_reg, dest, 0));
        self.emit(OpCode::St(value, dest, 1));
        Ok(dest)
    }
}
