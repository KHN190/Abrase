use super::Value;

pub trait Device {
    fn read(&mut self, port: u8) -> Result<Value, String>;
    fn write(&mut self, port: u8, val: Value) -> Result<(), String>;
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
