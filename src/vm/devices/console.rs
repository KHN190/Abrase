use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;
use crate::vm::{Device, Value};

pub const CONSOLE_ID: u8 = 0x10;
pub const PORT_WRITE_BYTE: u8 = 0x18;
pub const PORT_ERROR_BYTE: u8 = 0x19;
pub const PORT_WRITE_STRING: u8 = 0x1A;
pub const PORT_ERROR_STRING: u8 = 0x1B;

pub trait Console {
    fn out(&mut self, bytes: &[u8]) -> Result<(), String>;
    fn err(&mut self, bytes: &[u8]) -> Result<(), String>;
}

impl Device for Box<dyn Console> {
    fn read(&mut self, port: u8) -> Result<Value, String> {
        Err(format!("console: port {:#x} not readable", port))
    }

    fn write(&mut self, port: u8, val: Value) -> Result<(), String> {
        match port {
            PORT_WRITE_BYTE => self.out(&[byte_of(val)?]),
            PORT_ERROR_BYTE => self.err(&[byte_of(val)?]),
            PORT_WRITE_STRING => self.out(bytes_of_string(val)?.as_ref()),
            PORT_ERROR_STRING => self.err(bytes_of_string(val)?.as_ref()),
            _ => Err(format!("console: port {:#x} not writable", port)),
        }
    }
}

fn byte_of(val: Value) -> Result<u8, String> {
    match val {
        Value::Int(n) => Ok((n & 0xFF) as u8),
        v => Err(format!("console: expected Int for byte port, got {:?}", v)),
    }
}

fn bytes_of_string(val: Value) -> Result<Vec<u8>, String> {
    match val {
        Value::String(s) => Ok(s.into_bytes()),
        v => Err(format!("console: expected String for string port, got {:?}", v)),
    }
}

pub type SharedBuf = Rc<RefCell<Vec<u8>>>;

pub struct BufferConsole {
    pub out_buf: SharedBuf,
    pub err_buf: SharedBuf,
}

impl BufferConsole {
    pub fn new() -> Self {
        Self {
            out_buf: Rc::new(RefCell::new(Vec::new())),
            err_buf: Rc::new(RefCell::new(Vec::new())),
        }
    }
    pub fn handles(&self) -> (SharedBuf, SharedBuf) {
        (self.out_buf.clone(), self.err_buf.clone())
    }
}

impl Console for BufferConsole {
    fn out(&mut self, bytes: &[u8]) -> Result<(), String> {
        self.out_buf.borrow_mut().extend_from_slice(bytes);
        Ok(())
    }
    fn err(&mut self, bytes: &[u8]) -> Result<(), String> {
        self.err_buf.borrow_mut().extend_from_slice(bytes);
        Ok(())
    }
}

pub struct StdoutConsole;

impl Console for StdoutConsole {
    fn out(&mut self, bytes: &[u8]) -> Result<(), String> {
        std::io::stdout().write_all(bytes)
            .map_err(|e| format!("console.out: {}", e))
    }
    fn err(&mut self, bytes: &[u8]) -> Result<(), String> {
        std::io::stderr().write_all(bytes)
            .map_err(|e| format!("console.err: {}", e))
    }
}
