use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use crate::myriad::{Device, Value};

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
                let d = SystemTime::now().duration_since(UNIX_EPOCH)
                    .map_err(|e| format!("clock.wall_ms: system clock before UNIX epoch: {}", e))?;
                let ms = i64::try_from(d.as_millis())
                    .map_err(|_| "clock.wall_ms: timestamp overflows i64".to_string())?;
                Ok(Value::from_int(ms))
            }
            PORT_MONO_NS => {
                let ns = i64::try_from(self.epoch.elapsed().as_nanos())
                    .map_err(|_| "clock.mono_ns: elapsed overflows i64".to_string())?;
                Ok(Value::from_int(ns))
            }
            _ => Ok(Value::from_int(0)),
        }
    }

    fn write(&mut self, port: u8, val: Value) -> Result<(), String> {
        if port == PORT_SLEEP_MS {
            let ms = match val.as_int() { Some(n) if n >= 0 => n as u64, _ => 0 };
            std::thread::sleep(Duration::from_millis(ms));
        }
        Ok(())
    }
}
