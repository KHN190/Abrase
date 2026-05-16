use std::cell::RefCell;
use std::io::{Read, Write};
use std::rc::Rc;
use crate::myriad::{Device, Value};

pub const CONSOLE_ID: u8 = 0x10;
pub const PORT_STDIN: u8 = 0x00;
pub const PORT_STDOUT: u8 = 0x01;
pub const PORT_STDERR: u8 = 0x02;
pub const PORT_FLUSH: u8 = 0x03;

pub trait Console {
    fn read_byte(&mut self) -> Result<Option<u8>, String>;
    fn write_stdout(&mut self, byte: u8) -> Result<(), String>;
    fn write_stderr(&mut self, byte: u8) -> Result<(), String>;
    fn flush(&mut self) -> Result<(), String>;
}

impl Device for Box<dyn Console> {
    fn read(&mut self, port: u8) -> Result<Value, String> {
        match port {
            PORT_STDIN => match self.read_byte()? {
                Some(b) => Ok(Value::from_int(b as i64)),
                None => Ok(Value::from_int(-1)),
            },
            _ => Ok(Value::from_int(0)),
        }
    }

    fn write(&mut self, port: u8, val: Value) -> Result<(), String> {
        let n = match val.as_int() {
            Some(n) => n,
            None => return Ok(()),
        };
        match port {
            PORT_STDOUT => self.write_stdout((n & 0xFF) as u8),
            PORT_STDERR => self.write_stderr((n & 0xFF) as u8),
            PORT_FLUSH => self.flush(),
            _ => Ok(()),
        }
    }
}

pub type SharedBuf = Rc<RefCell<Vec<u8>>>;

pub struct BufferConsole {
    pub out_buf: SharedBuf,
    pub err_buf: SharedBuf,
    pub stdin_buf: SharedBuf,
}

impl BufferConsole {
    pub fn new() -> Self {
        Self {
            out_buf: Rc::new(RefCell::new(Vec::new())),
            err_buf: Rc::new(RefCell::new(Vec::new())),
            stdin_buf: Rc::new(RefCell::new(Vec::new())),
        }
    }
    pub fn handles(&self) -> (SharedBuf, SharedBuf) {
        (self.out_buf.clone(), self.err_buf.clone())
    }
    pub fn stdin_handle(&self) -> SharedBuf {
        self.stdin_buf.clone()
    }
}

impl Console for BufferConsole {
    fn read_byte(&mut self) -> Result<Option<u8>, String> {
        let mut buf = self.stdin_buf.borrow_mut();
        if buf.is_empty() { Ok(None) } else { Ok(Some(buf.remove(0))) }
    }
    fn write_stdout(&mut self, byte: u8) -> Result<(), String> {
        self.out_buf.borrow_mut().push(byte); Ok(())
    }
    fn write_stderr(&mut self, byte: u8) -> Result<(), String> {
        self.err_buf.borrow_mut().push(byte); Ok(())
    }
    fn flush(&mut self) -> Result<(), String> { Ok(()) }
}

pub struct StdoutConsole;

impl Console for StdoutConsole {
    fn read_byte(&mut self) -> Result<Option<u8>, String> {
        let mut byte = [0u8];
        match std::io::stdin().read(&mut byte) {
            Ok(0) => Ok(None),
            Ok(_) => Ok(Some(byte[0])),
            Err(e) => Err(format!("console.stdin: {}", e)),
        }
    }
    fn write_stdout(&mut self, byte: u8) -> Result<(), String> {
        std::io::stdout().write_all(&[byte])
            .map_err(|e| format!("console.stdout: {}", e))
    }
    fn write_stderr(&mut self, byte: u8) -> Result<(), String> {
        std::io::stderr().write_all(&[byte])
            .map_err(|e| format!("console.stderr: {}", e))
    }
    fn flush(&mut self) -> Result<(), String> {
        std::io::stdout().flush().map_err(|e| format!("flush: {}", e))?;
        std::io::stderr().flush().map_err(|e| format!("flush: {}", e))
    }
}
