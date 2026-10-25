#[cfg(not(all(target_os = "none", target_pointer_width = "32")))]
pub type Constant = u64;

#[cfg(all(target_os = "none", target_pointer_width = "32"))]
pub type Constant = Constant32;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Constant32 {
    low: u32,
    high: u32,
}

impl Constant32 {
    pub fn from_le_bytes(bytes: [u8; 8]) -> Self {
        Self {
            low: u32::from_le_bytes(bytes[..4].try_into().unwrap()),
            high: u32::from_le_bytes(bytes[4..].try_into().unwrap()),
        }
    }

    pub fn to_le_bytes(self) -> [u8; 8] {
        let mut bytes = [0; 8];
        bytes[..4].copy_from_slice(&self.low.to_le_bytes());
        bytes[4..].copy_from_slice(&self.high.to_le_bytes());
        bytes
    }

    pub fn to_word(self) -> Result<u32, &'static str> {
        if self.high == 0 || (self.high == u32::MAX && self.low & 0x80000000 != 0) {
            Ok(self.low)
        } else {
            Err("constant exceeds 32-bit word")
        }
    }

    pub fn is_bytes(self) -> bool { self.high & 0x80000000 != 0 }

    pub fn pool_index(self) -> Result<usize, &'static str> {
        if self.high & 0x7fffffff == 0 {
            Ok(self.low as usize)
        } else {
            Err("constant pool index exceeds 32 bits")
        }
    }
}

#[inline]
pub fn constant_word32(value: Constant) -> Result<u32, &'static str> {
    Constant32::from_le_bytes(value.to_le_bytes()).to_word()
}

#[inline]
pub fn constant_is_bytes(value: Constant) -> bool {
    Constant32::from_le_bytes(value.to_le_bytes()).is_bytes()
}

#[inline]
pub fn constant_pool_index(value: Constant) -> Result<usize, &'static str> {
    Constant32::from_le_bytes(value.to_le_bytes()).pool_index()
}
