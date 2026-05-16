use crate::vm::{Device, Value};

pub const SYSTEM_ID: u8 = 0x00;
pub const PORT_VERSION: u8 = 0x00;
pub const PORT_HALT: u8 = 0x01;
pub const PORT_PANIC: u8 = 0x02;
pub const PORT_FLAGS: u8 = 0x03;

pub const SPEC_VERSION: i64 = 1i64 << 32;

pub struct SystemDevice {
    pub last_exit: Option<i64>,
    pub flags: i64,
    pub constants: Option<std::rc::Rc<Vec<Value>>>,
}

impl SystemDevice {
    pub fn new() -> Self {
        Self { last_exit: None, flags: 0, constants: None }
    }
}

impl Device for SystemDevice {
    fn read(&mut self, port: u8) -> Result<Value, String> {
        match port {
            PORT_VERSION => Ok(Value::Int(SPEC_VERSION)),
            PORT_FLAGS => Ok(Value::Int(self.flags)),
            _ => Ok(Value::Int(0)),
        }
    }

    fn write(&mut self, port: u8, val: Value) -> Result<(), String> {
        match port {
            PORT_HALT => {
                let code = as_int(val, "system.halt")?;
                self.last_exit = Some(code & 0xFFFF_FFFF);
                Ok(())
            }
            PORT_PANIC => {
                let idx = as_int(val, "system.panic")? as usize;
                let msg = self.constants.as_ref()
                    .and_then(|c| c.get(idx))
                    .and_then(|v| if let Value::String(s) = v { Some(s.clone()) } else { None })
                    .unwrap_or_else(|| format!("panic (pool idx {})", idx));
                Err(format!("panic: {}", msg))
            }
            _ => Ok(()),
        }
    }
}

fn as_int(v: Value, op: &str) -> Result<i64, String> {
    match v { Value::Int(n) => Ok(n), other => Err(format!("{}: expected Int, got {:?}", op, other)) }
}
