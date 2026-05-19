use std::cell::Cell;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::{Device, Value};

pub const RANDOM_ID: u8 = 0x70;
pub const PORT_BYTE: u8 = 0x00;
pub const PORT_U64: u8 = 0x01;
pub const PORT_SEED: u8 = 0x02;

pub trait Random {
    fn next_byte(&mut self) -> u8;
    fn next_u64(&mut self) -> u64;
    fn seed(&mut self, seed: u64);
}

impl Device for Box<dyn Random> {
    fn read(&mut self, port: u8) -> Result<Value, String> {
        match port {
            PORT_BYTE => Ok(Value::from_int(self.next_byte() as i64)),
            PORT_U64 => Ok(Value::from_raw(self.next_u64())),
            _ => Ok(Value::from_int(0)),
        }
    }

    fn write(&mut self, port: u8, val: Value) -> Result<(), String> {
        if port == PORT_SEED {
            self.seed(val.raw());
        }
        Ok(())
    }
}

struct Xorshift { state: Cell<u64> }

impl Xorshift {
    fn new(seed: u64) -> Self { Self { state: Cell::new(seed | 1) } }
    fn next(&self) -> u64 {
        let mut x = self.state.get();
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state.set(x);
        x
    }
}

pub struct SystemRandom { inner: Xorshift }

impl SystemRandom {
    pub fn new() -> Self {
        let seed = SystemTime::now().duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0xdeadbeef);
        Self { inner: Xorshift::new(seed) }
    }
}

impl Random for SystemRandom {
    fn next_byte(&mut self) -> u8 { (self.inner.next() & 0xFF) as u8 }
    fn next_u64(&mut self) -> u64 { self.inner.next() }
    fn seed(&mut self, seed: u64) { self.inner.state.set(seed | 1); }
}

pub struct SeededRandom { inner: Xorshift }

impl SeededRandom {
    pub fn new(seed: u64) -> Self { Self { inner: Xorshift::new(seed) } }
}

impl Random for SeededRandom {
    fn next_byte(&mut self) -> u8 { (self.inner.next() & 0xFF) as u8 }
    fn next_u64(&mut self) -> u64 { self.inner.next() }
    fn seed(&mut self, seed: u64) { self.inner.state.set(seed | 1); }
}
