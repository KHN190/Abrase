use std::cell::RefCell;
use std::rc::Rc;
use crate::{Device, Heap, Value};

pub const HOSTFUNC_ID: u8 = 0xF0;
pub const PORT_ARG: u8 = 0x18;
pub const PORT_TRIGGER: u8 = 0x1F;
pub const PORT_RESULT: u8 = 0x1E;

// Host fn signature. Args are raw u64 — type is implicit from the registered
// fn's declared signature (mask info lives caller-side in the polka frame).
// Return is (raw, is_handle).
pub type HostImpl = Rc<dyn Fn(&mut Heap, &[u64]) -> Result<(u64, bool), String>>;

pub struct HostFuncDevice {
    table: Vec<HostImpl>,
    arg_buf: RefCell<Vec<u64>>,
    last_result: RefCell<Option<(u64, bool)>>,
}

impl HostFuncDevice {
    pub fn new() -> Self {
        Self {
            table: Vec::new(),
            arg_buf: RefCell::new(Vec::new()),
            last_result: RefCell::new(None),
        }
    }

    pub fn register(&mut self, func: HostImpl) -> u16 {
        let id = self.table.len() as u16;
        self.table.push(func);
        id
    }

    pub fn len(&self) -> usize { self.table.len() }

    // is_handle bit of the last result (used by interpreter to set caller mask).
    pub fn last_result_is_handle(&self) -> bool {
        self.last_result.borrow().map(|(_, h)| h).unwrap_or(false)
    }
}

impl Device for HostFuncDevice {
    fn read(&mut self, port: u8) -> Result<Value, String> {
        match port {
            PORT_RESULT => {
                self.last_result.borrow().map(|(v, _)| Value::from_raw(v))
                    .ok_or_else(|| "hostfunc: no result available".to_string())
            }
            _ => Ok(Value::from_int(0)),
        }
    }

    fn write(&mut self, port: u8, val: Value) -> Result<(), String> {
        match port {
            PORT_ARG => {
                self.arg_buf.borrow_mut().push(val.raw());
                Ok(())
            }
            PORT_TRIGGER => Err("hostfunc.trigger: requires Heap — use write_with_heap".to_string()),
            _ => Ok(()),
        }
    }

    fn write_with_heap(
        &mut self,
        port: u8,
        val: Value,
        heap: &mut Heap,
    ) -> Result<(), String> {
        if port != PORT_TRIGGER { return self.write(port, val); }
        let fn_id = val.as_int();
        if fn_id < 0 || (fn_id as usize) >= self.table.len() {
            return Err(format!("hostfunc.trigger: unknown fn_id {}", fn_id));
        }
        let f = self.table[fn_id as usize].clone();
        let args = std::mem::take(&mut *self.arg_buf.borrow_mut());
        let result = f(heap, &args)?;
        *self.last_result.borrow_mut() = Some(result);
        Ok(())
    }
}
