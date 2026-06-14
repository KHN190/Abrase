use alloc::{boxed::Box, string::String, vec::Vec};
use crate::builtins::{NativeCtx, NativeRegistry, register_default_builtins};
use crate::devices::{BufferConsole, Console, DeviceTable, CONSOLE_ID, SharedBuf};
use crate::memory::Heap;
use crate::Value;

pub trait AotNatives {
    fn call(&mut self, name: &str, heap: &mut Heap, args: &[Value]) -> Result<(u64, bool), String>;
    fn halted(&self) -> bool { false }
}

pub fn reachable_live_count(h: &Heap, root_raw: u64) -> usize {
    use hashbrown::HashSet;
    let mut seen: HashSet<(u32, u32)> = HashSet::new();
    if root_raw != polka::HANDLE_NONE {
        let (s, g) = Value::from_raw(root_raw).as_handle();
        walk(h, s, g, &mut seen);
    }
    seen.iter().filter(|(s, g)| h.is_live(*s, *g)).count()
}

fn walk(h: &Heap, slot: u32, g: u32, seen: &mut hashbrown::HashSet<(u32, u32)>) {
    if !seen.insert((slot, g)) || !h.is_live(slot, g) { return; }
    let (Ok(data), Ok(mask)) = (h.cell_data(slot, g), h.cell_mask(slot, g)) else { return; };
    let data: Vec<u64> = data.to_vec();
    let mask: Vec<u64> = mask.to_vec();
    for i in 0..data.len() {
        if (mask[i / 64] >> (i % 64)) & 1 == 1 && data[i] != polka::HANDLE_NONE {
            let (cs, cg) = Value::from_raw(data[i]).as_handle();
            walk(h, cs, cg, seen);
        }
    }
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
    fn halted(&self) -> bool { self.halted }
}
