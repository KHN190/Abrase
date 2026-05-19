use std::cell::Cell;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use crate::{Device, Value};

pub const CLOCK_ID: u8 = 0x60;
pub const PORT_WALL_MS: u8 = 0x00;
pub const PORT_MONO_NS: u8 = 0x01;
pub const PORT_SLEEP_MS: u8 = 0x02;

pub trait Clock {
    fn wall_ms(&mut self) -> Result<i64, String>;
    fn mono_ns(&mut self) -> Result<i64, String>;
    fn sleep_ms(&mut self, ms: u64) -> Result<(), String>;
}

impl Device for Box<dyn Clock> {
    fn read(&mut self, port: u8) -> Result<Value, String> {
        match port {
            PORT_WALL_MS => Ok(Value::from_int(self.wall_ms()?)),
            PORT_MONO_NS => Ok(Value::from_int(self.mono_ns()?)),
            _ => Ok(Value::from_int(0)),
        }
    }

    fn write(&mut self, port: u8, val: Value) -> Result<(), String> {
        if port == PORT_SLEEP_MS {
            let n = val.as_int();
            let ms = if n >= 0 { n as u64 } else { 0 };
            self.sleep_ms(ms)?;
        }
        Ok(())
    }
}

pub struct SystemClock {
    epoch: Instant,
}

impl SystemClock {
    pub fn new() -> Self { Self { epoch: Instant::now() } }
}

impl Clock for SystemClock {
    fn wall_ms(&mut self) -> Result<i64, String> {
        let d = SystemTime::now().duration_since(UNIX_EPOCH)
            .map_err(|e| format!("clock.wall_ms: system clock before UNIX epoch: {}", e))?;
        i64::try_from(d.as_millis())
            .map_err(|_| "clock.wall_ms: timestamp overflows i64".to_string())
    }
    fn mono_ns(&mut self) -> Result<i64, String> {
        i64::try_from(self.epoch.elapsed().as_nanos())
            .map_err(|_| "clock.mono_ns: elapsed overflows i64".to_string())
    }
    fn sleep_ms(&mut self, ms: u64) -> Result<(), String> {
        std::thread::sleep(Duration::from_millis(ms));
        Ok(())
    }
}

pub struct MockClock {
    wall_ms: Cell<i64>,
    mono_ns: Cell<i64>,
}

impl MockClock {
    pub fn new() -> Self { Self { wall_ms: Cell::new(0), mono_ns: Cell::new(0) } }

    pub fn at(wall_ms: i64) -> Self {
        Self { wall_ms: Cell::new(wall_ms), mono_ns: Cell::new(0) }
    }

    pub fn advance(&self, ms: u64) {
        self.wall_ms.set(self.wall_ms.get().saturating_add(ms as i64));
        self.mono_ns.set(self.mono_ns.get().saturating_add((ms as i64).saturating_mul(1_000_000)));
    }
}

impl Clock for MockClock {
    fn wall_ms(&mut self) -> Result<i64, String> { Ok(self.wall_ms.get()) }
    fn mono_ns(&mut self) -> Result<i64, String> { Ok(self.mono_ns.get()) }
    fn sleep_ms(&mut self, ms: u64) -> Result<(), String> {
        self.advance(ms);
        Ok(())
    }
}
