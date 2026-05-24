use crate::{Device, Value};
use crate::memory::Heap;

pub const SYSTEM_ID: u8 = 0x00;
pub const PORT_VERSION_MAJOR: u8 = 0x00;
pub const PORT_HALT: u8 = 0x01;
pub const PORT_PANIC: u8 = 0x02;
pub const PORT_FLAGS: u8 = 0x03;
pub const PORT_VERSION_MINOR: u8 = 0x04;
pub const PORT_VERSION_PATCH: u8 = 0x05;

pub const SPEC_MAJOR: i64 = 2;
pub const SPEC_MINOR: i64 = 0;
pub const SPEC_PATCH: i64 = 1;

pub struct SystemDevice {
    pub flags: i64,
}

impl SystemDevice {
    pub fn new() -> Self {
        Self { flags: 0 }
    }
}

impl Device for SystemDevice {
    fn read(&mut self, port: u8) -> Result<(Value, bool), String> {
        let v = match port {
            PORT_VERSION_MAJOR => Value::from_int(SPEC_MAJOR),
            PORT_VERSION_MINOR => Value::from_int(SPEC_MINOR),
            PORT_VERSION_PATCH => Value::from_int(SPEC_PATCH),
            PORT_FLAGS => Value::from_int(self.flags),
            _ => Value::from_int(0),
        };
        Ok((v, false))
    }

    fn write(&mut self, _port: u8, val: Value, is_handle: bool, heap: &mut Heap)
        -> Result<(), String>
    {
        if is_handle { heap.rc_dec_handle(val.raw())?; }
        Ok(())
    }
}
