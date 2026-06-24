use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use alloc::format;

// Interpreter-side backing store for the `<core>` raw-memory intrinsics.
// A flat little-endian byte buffer, kept separate from the VM `Heap` so core
// code reimplementing alloc/RC/region never recurses into the heap it replaces.
// Addresses are arena-relative: physical = base + addr, with base owned by the
// host (bare metal) or implicit 0 here. Every access is bounds- and
// alignment-checked; a violation traps (clean halt), never a wild access.
pub struct CoreArena {
    bytes: Vec<u8>,
}

impl CoreArena {
    pub fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    pub fn with_size(n: usize) -> Self {
        Self { bytes: vec![0u8; n] }
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    fn check(&self, addr: u64, width: u64) -> Result<usize, String> {
        if addr & (width - 1) != 0 {
            return Err(format!("core arena: misaligned {}-byte access at {:#x}", width, addr));
        }
        let end = addr
            .checked_add(width)
            .ok_or_else(|| format!("core arena: address overflow at {:#x}", addr))?;
        if end > self.bytes.len() as u64 {
            return Err(format!(
                "core arena: out-of-bounds {}-byte access at {:#x} (len {:#x})",
                width, addr, self.bytes.len()
            ));
        }
        Ok(addr as usize)
    }

    pub fn peek(&self, addr: u64, width: u64) -> Result<u64, String> {
        let off = self.check(addr, width)?;
        let mut v: u64 = 0;
        for i in 0..width as usize {
            v |= (self.bytes[off + i] as u64) << (8 * i);
        }
        Ok(v)
    }

    pub fn poke(&mut self, addr: u64, width: u64, val: u64) -> Result<(), String> {
        let off = self.check(addr, width)?;
        for i in 0..width as usize {
            self.bytes[off + i] = (val >> (8 * i)) as u8;
        }
        Ok(())
    }

    pub fn ptr_add(addr: u64, delta: i64) -> u64 {
        addr.wrapping_add(delta as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_each_width_little_endian() {
        let mut a = CoreArena::with_size(64);
        a.poke(0, 8, 0x0123456789abcdef).unwrap();
        a.poke(16, 4, 0xdeadbeef).unwrap();
        a.poke(32, 1, 0xa5).unwrap();
        assert_eq!(a.peek(0, 8).unwrap(), 0x0123456789abcdef);
        assert_eq!(a.peek(16, 4).unwrap(), 0xdeadbeef);
        assert_eq!(a.peek(32, 1).unwrap(), 0xa5);
        assert_eq!(a.peek(0, 1).unwrap(), 0xef);
    }

    #[test]
    fn poke_truncates_to_width_low_bits() {
        let mut a = CoreArena::with_size(16);
        a.poke(0, 1, 0x1234).unwrap();
        assert_eq!(a.peek(0, 1).unwrap(), 0x34);
        a.poke(4, 4, 0xffff_ffff_ffff_ffff).unwrap();
        assert_eq!(a.peek(4, 4).unwrap(), 0xffff_ffff);
    }

    #[test]
    fn out_of_bounds_traps() {
        let a = CoreArena::with_size(8);
        assert!(a.peek(8, 1).is_err());
        assert!(a.peek(4, 8).is_err());
        assert!(a.peek(u64::MAX, 8).is_err());
    }

    #[test]
    fn misaligned_traps() {
        let mut a = CoreArena::with_size(32);
        assert!(a.peek(1, 4).is_err());
        assert!(a.peek(4, 8).is_err());
        assert!(a.poke(2, 4, 0).is_err());
        assert!(a.peek(1, 1).is_ok());
    }

    #[test]
    fn ptr_add_is_relative_arithmetic() {
        assert_eq!(CoreArena::ptr_add(8, 4), 12);
        assert_eq!(CoreArena::ptr_add(8, -8), 0);
    }
}
