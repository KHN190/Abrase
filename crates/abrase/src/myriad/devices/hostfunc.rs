use std::cell::RefCell;
use std::rc::Rc;
use crate::myriad::{BoxPool, Device, Value};

pub const HOSTFUNC_ID: u8 = 0xF0;
pub const PORT_ARG: u8 = 0x18;
pub const PORT_TRIGGER: u8 = 0x1F;
pub const PORT_RESULT: u8 = 0x1E;

pub type HostImpl = Rc<dyn Fn(&mut BoxPool, &[Value]) -> Result<Value, String>>;

pub struct HostFuncDevice {
    table: Vec<HostImpl>,
    arg_buf: RefCell<Vec<Value>>,
    last_result: RefCell<Option<Value>>,
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
}

impl Device for HostFuncDevice {
    fn read(&mut self, port: u8) -> Result<Value, String> {
        match port {
            PORT_RESULT => {
                self.last_result.borrow().clone()
                    .ok_or_else(|| "hostfunc: no result available".to_string())
            }
            _ => Ok(Value::from_int(0)),
        }
    }

    fn write(&mut self, port: u8, val: Value) -> Result<(), String> {
        match port {
            PORT_ARG => {
                self.arg_buf.borrow_mut().push(val);
                Ok(())
            }
            PORT_TRIGGER => Err("hostfunc.trigger: requires BoxPool — use write_with_pool".to_string()),
            _ => Ok(()),
        }
    }

    fn write_with_pool(&mut self, port: u8, val: Value, pool: &mut BoxPool) -> Result<(), String> {
        if port != PORT_TRIGGER { return self.write(port, val); }
        let fn_id = match val.as_int() {
            Some(n) if n >= 0 => n as usize,
            _ => return Err(format!("hostfunc.trigger: bad fn_id {:?}", val)),
        };
        if fn_id >= self.table.len() {
            return Err(format!("hostfunc.trigger: unknown fn_id {}", fn_id));
        }
        let f = self.table[fn_id].clone();
        // arg_buf entries are borrowed views — caller's source registers keep the rc.
        let args = std::mem::take(&mut *self.arg_buf.borrow_mut());
        let result = f(pool, &args)?;
        let prev = self.last_result.borrow_mut().replace(result);
        if let Some(p) = prev {
            if let Some(idx) = p.as_box() { pool.dec(idx); }
        }
        Ok(())
    }
}
