use super::{Value, BoxPool};

pub trait Device {
    fn read(&mut self, port: u8) -> Result<Value, String>;
    fn write(&mut self, port: u8, val: Value) -> Result<(), String>;
    // Default forwards to write(). Override when the device needs the box
    // pool (e.g. to invoke a registered host fn whose signature takes it).
    fn write_with_pool(
        &mut self,
        port: u8,
        val: Value,
        _pool: &mut BoxPool,
    ) -> Result<(), String> {
        self.write(port, val)
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

    pub fn has(&self, id: u8) -> bool {
        self.slots[id as usize].is_some()
    }

    pub fn get_mut(&mut self, id: u8) -> Option<&mut Box<dyn Device>> {
        self.slots[id as usize].as_mut()
    }

    pub fn take(&mut self, id: u8) -> Option<Box<dyn Device>> {
        self.slots[id as usize].take()
    }
}
