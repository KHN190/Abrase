use alloc::{boxed::Box, string::String, vec::Vec};
pub mod system;
pub mod console;

pub use system::{SystemDevice, SYSTEM_ID};
pub use console::{Console, BufferConsole, CONSOLE_ID, SharedBuf};
use polka::Value;
use crate::memory::Heap;

pub trait Device {
    /// Read a port. Returns (value, is_handle). For non-handle ports the tag
    /// is `false`. For state-style ports that store cell handles, the device
    /// returns the stored handle bits with `is_handle = true`; the runtime
    /// gives the cart a fresh observer (rc_inc) on return.
    fn read(&mut self, port: u8) -> Result<(Value, bool), String>;

    /// Write a port. `is_handle` reflects the source register's tag. For
    /// handle writes the runtime has already rc_inc'd the value, so the
    /// device receives a fresh observer to either store (consuming the rc)
    /// or discard (releasing rc via `heap.rc_dec`). State devices that
    /// overwrite a previously-stored handle must rc_dec the old one through
    /// `heap`.
    fn write(&mut self, port: u8, val: Value, is_handle: bool, heap: &mut Heap)
        -> Result<(), String>;

    // Bulk byte write — host-side optimization path for stream-oriented devices
    // (console, file, network). Default forwards to per-byte `write`; override
    // to issue a single syscall. Not exposed via DEI/DEO.
    fn write_bytes(&mut self, port: u8, bytes: &[u8], heap: &mut Heap) -> Result<(), String> {
        for &b in bytes {
            self.write(port, Value::from_int(b as i64), false, heap)?;
        }
        Ok(())
    }
}

pub struct DeviceTable {
    slots: Vec<Option<Box<dyn Device>>>,
}

impl DeviceTable {
    pub fn new() -> Self {
        let mut slots: Vec<Option<Box<dyn Device>>> = Vec::with_capacity(256);
        for _ in 0..256 { slots.push(None); }
        Self { slots }
    }

    pub fn install(&mut self, id: u8, dev: Box<dyn Device>) {
        self.slots[id as usize] = Some(dev);
    }

    pub fn get_mut(&mut self, id: u8) -> Option<&mut Box<dyn Device>> {
        self.slots[id as usize].as_mut()
    }

    pub fn take(&mut self, id: u8) -> Option<Box<dyn Device>> {
        self.slots[id as usize].take()
    }
}
