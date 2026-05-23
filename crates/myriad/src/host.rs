use crate::{Device, VirtualMachine};
use crate::devices::{
    BufferConsole, Console, SystemDevice, StdoutConsole,
    CONSOLE_ID, SYSTEM_ID,
};

// Bundle of core devices installed at VM startup.
pub struct Host {
    pub system: Option<SystemDevice>,
    pub console: Option<Box<dyn Console>>,
    pub extra: Vec<(u8, Box<dyn Device>)>,
}

impl Host {
    pub fn default() -> Self {
        Self {
            system: Some(SystemDevice::new()),
            console: Some(Box::new(StdoutConsole)),
            extra: Vec::new(),
        }
    }

    pub fn headless() -> Self {
        Self {
            system: Some(SystemDevice::new()),
            console: Some(Box::new(BufferConsole::new())),
            extra: Vec::new(),
        }
    }

    pub fn with_console(mut self, c: Box<dyn Console>) -> Self {
        self.console = Some(c); self
    }
    pub fn with_device(mut self, id: u8, d: Box<dyn Device>) -> Self {
        self.extra.push((id, d)); self
    }

    pub fn install_into(self, vm: &mut VirtualMachine) {
        if let Some(d) = self.system  { vm.install_device(SYSTEM_ID,  Box::new(d)); }
        if let Some(d) = self.console { vm.install_device(CONSOLE_ID, Box::new(d)); }
        for (id, dev) in self.extra { vm.install_device(id, dev); }
    }
}
