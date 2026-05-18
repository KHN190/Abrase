use crate::{Device, Value};

pub const SYSTEM_ID: u8 = 0x00;
pub const PORT_VERSION: u8 = 0x00;
pub const PORT_HALT: u8 = 0x01;
pub const PORT_PANIC: u8 = 0x02;
pub const PORT_FLAGS: u8 = 0x03;

pub const SPEC_VERSION: i64 = 1i64 << 32;

pub struct SystemDevice {
    pub flags: i64,
}

impl SystemDevice {
    pub fn new() -> Self {
        Self { flags: 0 }
    }
}

impl Device for SystemDevice {
    fn read(&mut self, port: u8) -> Result<Value, String> {
        match port {
            PORT_VERSION => Ok(Value::from_int(SPEC_VERSION)),
            PORT_FLAGS => Ok(Value::from_int(self.flags)),
            _ => Ok(Value::from_int(0)),
        }
    }

    // Writes to halt (0x01) and panic (0x02) are intercepted by the interpreter
    // before reaching here, since they are VM control flow, not device I/O.
    fn write(&mut self, _port: u8, _val: Value) -> Result<(), String> {
        Ok(())
    }
}
