use alloc::{boxed::Box, string::String, vec::Vec};
use crate::builtins::{NativeCtx, NativeRegistry, register_default_builtins};
use crate::devices::{BufferConsole, Console, DeviceTable, CONSOLE_ID, SharedBuf};
use crate::memory::Heap;
use crate::Value;

pub trait AotNatives {
    fn call(&mut self, name: &str, heap: &mut Heap, args: &[Value]) -> Result<(u64, bool), String>;
}

pub struct AotHost {
    devices: DeviceTable,
    registry: NativeRegistry,
    halted: bool,
    exit_code: Option<i64>,
    stdout: SharedBuf,
}

impl Default for AotHost {
    fn default() -> Self { Self::new() }
}

impl AotHost {
    pub fn new() -> Self {
        let console = BufferConsole::new();
        let stdout = console.out_buf.clone();
        let mut devices = DeviceTable::new();
        devices.install(CONSOLE_ID, Box::new(Box::new(console) as Box<dyn Console>));
        let mut registry = NativeRegistry::new();
        register_default_builtins(&mut registry);
        Self { devices, registry, halted: false, exit_code: None, stdout }
    }

    pub fn halted(&self) -> bool { self.halted }
    pub fn exit_code(&self) -> Option<i64> { self.exit_code }

    pub fn take_stdout(&self) -> Vec<u8> {
        self.stdout.borrow().clone()
    }
}

impl AotNatives for AotHost {
    fn call(&mut self, name: &str, heap: &mut Heap, args: &[Value]) -> Result<(u64, bool), String> {
        let f = self.registry.get(name)
            .ok_or_else(|| alloc::format!("unknown native fn {}", name))?
            .clone();
        let mut ctx = NativeCtx {
            heap,
            devices: &mut self.devices,
            halted: &mut self.halted,
            exit_code: &mut self.exit_code,
        };
        let (v, h) = f(&mut ctx, args)?;
        Ok((v.raw(), h))
    }
}
