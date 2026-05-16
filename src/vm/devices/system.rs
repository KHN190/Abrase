use crate::vm::{Device, Value};

pub const SYSTEM_ID: u8 = 0x00;
pub const PORT_EXIT: u8 = 0x00;
pub const PORT_PANIC: u8 = 0x01;

pub struct SystemDevice {
    pub last_exit: Option<i64>,
}

impl SystemDevice {
    pub fn new() -> Self {
        Self { last_exit: None }
    }
}

impl Device for SystemDevice {
    fn read(&mut self, port: u8) -> Result<Value, String> {
        match port {
            PORT_EXIT => Ok(Value::Int(self.last_exit.unwrap_or(0))),
            _ => Err(format!("system: port {:#x} not readable", port)),
        }
    }

    fn write(&mut self, port: u8, val: Value) -> Result<(), String> {
        match port {
            PORT_EXIT => {
                let code = match val {
                    Value::Int(n) => n,
                    _ => return Err("system.exit expects Int".into()),
                };
                self.last_exit = Some(code);
                Ok(())
            }
            PORT_PANIC => {
                let code = match val {
                    Value::Int(n) => n,
                    _ => return Err("system.panic expects Int".into()),
                };
                Err(format!("panic: code {}", code))
            }
            _ => Err(format!("system: port {:#x} not writable", port)),
        }
    }
}
