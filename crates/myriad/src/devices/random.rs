use std::cell::Cell;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::{Device, Value};

pub const RANDOM_ID: u8 = 0x70;
pub const PORT_BYTE: u8 = 0x00;
pub const PORT_U64: u8 = 0x01;
pub const PORT_SEED: u8 = 0x02;

pub struct RandomDevice {
    state: Cell<u64>,
}

impl RandomDevice {
    pub fn new() -> Self {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0xdeadbeef);
        Self { state: Cell::new(nanos | 1) }
    }
    pub fn seeded(seed: u64) -> Self { Self { state: Cell::new(seed | 1) } }

    fn next_u64(&self) -> u64 {
        let mut x = self.state.get();
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state.set(x);
        x
    }
}

impl Device for RandomDevice {
    fn read(&mut self, port: u8) -> Result<Value, String> {
        match port {
            PORT_BYTE => Ok(Value::from_int((self.next_u64() & 0xFF) as i64)),
            PORT_U64 => Ok(Value::from_int(self.next_u64() as i64)),
            _ => Ok(Value::from_int(0)),
        }
    }

    fn write(&mut self, port: u8, val: Value) -> Result<(), String> {
        if port == PORT_SEED {
            if let Some(n) = val.as_int() {
                self.state.set((n as u64) | 1);
            }
        }
        Ok(())
    }
}
