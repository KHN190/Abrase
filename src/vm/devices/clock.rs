use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use crate::vm::{Device, Value};

pub const CLOCK_ID: u8 = 0x60;
pub const PORT_WALL_MS: u8 = 0x00;
pub const PORT_MONO_NS: u8 = 0x01;
pub const PORT_SLEEP_MS: u8 = 0x02;

pub struct ClockDevice {
    epoch: Instant,
}

impl ClockDevice {
    pub fn new() -> Self { Self { epoch: Instant::now() } }
}

impl Device for ClockDevice {
    fn read(&mut self, port: u8) -> Result<Value, String> {
        match port {
            PORT_WALL_MS => {
                let ms = SystemTime::now().duration_since(UNIX_EPOCH)
                    .map(|d| d.as_millis() as i64)
                    .unwrap_or(0);
                Ok(Value::Int(ms))
            }
            PORT_MONO_NS => Ok(Value::Int(self.epoch.elapsed().as_nanos() as i64)),
            _ => Ok(Value::Int(0)),
        }
    }

    fn write(&mut self, port: u8, val: Value) -> Result<(), String> {
        if port == PORT_SLEEP_MS {
            let ms = match val { Value::Int(n) if n >= 0 => n as u64, _ => 0 };
            std::thread::sleep(Duration::from_millis(ms));
        }
        Ok(())
    }
}
