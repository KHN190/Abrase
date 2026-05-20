use crate::{Device, VirtualMachine};
use crate::devices::{
    BufferConsole, Clock, Console, MockClock, Random, SeededRandom,
    SystemClock, SystemDevice, SystemRandom, StdoutConsole,
    CLOCK_ID, CONSOLE_ID, RANDOM_ID, SYSTEM_ID,
};

// Bundle of devices installed at VM startup.
pub struct Host {
    pub system: Option<SystemDevice>,
    pub console: Option<Box<dyn Console>>,
    pub clock: Option<Box<dyn Clock>>,
    pub random: Option<Box<dyn Random>>,
    pub extra: Vec<(u8, Box<dyn Device>)>,
}

impl Host {
    pub fn default() -> Self {
        Self {
            system: Some(SystemDevice::new()),
            console: Some(Box::new(StdoutConsole)),
            clock: Some(Box::new(SystemClock::new())),
            random: Some(Box::new(SystemRandom::new())),
            extra: Vec::new(),
        }
    }

    pub fn headless() -> Self {
        Self {
            system: Some(SystemDevice::new()),
            console: Some(Box::new(BufferConsole::new())),
            clock: Some(Box::new(MockClock::new())),
            random: Some(Box::new(SeededRandom::new(0xC0FFEE))),
            extra: Vec::new(),
        }
    }

    pub fn with_console(mut self, c: Box<dyn Console>) -> Self {
        self.console = Some(c); self
    }
    pub fn with_clock(mut self, c: Box<dyn Clock>) -> Self {
        self.clock = Some(c); self
    }
    pub fn with_random(mut self, r: Box<dyn Random>) -> Self {
        self.random = Some(r); self
    }
    pub fn with_device(mut self, id: u8, d: Box<dyn Device>) -> Self {
        self.extra.push((id, d)); self
    }

    pub fn install_into(self, vm: &mut VirtualMachine) {
        if let Some(d) = self.system  { vm.install_device(SYSTEM_ID,  Box::new(d)); }
        if let Some(d) = self.console { vm.install_device(CONSOLE_ID, Box::new(d)); }
        if let Some(d) = self.clock   { vm.install_device(CLOCK_ID,   Box::new(d)); }
        if let Some(d) = self.random  { vm.install_device(RANDOM_ID,  Box::new(d)); }
        for (id, dev) in self.extra { vm.install_device(id, dev); }
    }
}
