use crate::{BoxPool, BoxedValue, Value};
use std::collections::HashMap;
use std::rc::Rc;

pub type NativeFn = Rc<dyn Fn(&mut BoxPool, &[Value]) -> Result<Value, String>>;

#[derive(Default, Clone)]
pub struct NativeRegistry {
    fns: HashMap<String, NativeFn>,
}

impl NativeRegistry {
    pub fn new() -> Self { Self { fns: HashMap::new() } }

    pub fn register<S: Into<String>>(&mut self, name: S, func: NativeFn) {
        self.fns.insert(name.into(), func);
    }

    pub fn get(&self, name: &str) -> Option<&NativeFn> {
        self.fns.get(name)
    }
}

pub fn register_default_builtins(reg: &mut NativeRegistry) {
    reg.register("__concat", concat_native());
    reg.register("__to_str", to_str_native());
}

fn concat_native() -> NativeFn {
    Rc::new(|pool: &mut BoxPool, args: &[Value]| {
        let a = extract_string(pool, &args[0])
            .ok_or_else(|| format!("__concat: arg0 not a String: {:?}", args[0]))?;
        let b = extract_string(pool, &args[1])
            .ok_or_else(|| format!("__concat: arg1 not a String: {:?}", args[1]))?;
        let mut out = String::with_capacity(a.len() + b.len());
        out.push_str(&a);
        out.push_str(&b);
        let idx = pool.intern(BoxedValue::String(out));
        Ok(Value::from_box(idx))
    })
}

fn to_str_native() -> NativeFn {
    Rc::new(|pool: &mut BoxPool, args: &[Value]| {
        let v = &args[0];
        let s = if let Some(n) = v.as_int() {
            n.to_string()
        } else if let Some(f) = v.as_float() {
            f.to_string()
        } else if let Some(b) = v.as_bool() {
            b.to_string()
        } else if let Some(c) = v.as_char() {
            c.to_string()
        } else if v.is_unit() {
            "()".to_string()
        } else if let Some(s) = extract_string(pool, v) {
            s
        } else {
            return Err(format!("__to_str: cannot convert {:?}", v));
        };
        let idx = pool.intern(BoxedValue::String(s));
        Ok(Value::from_box(idx))
    })
}

fn extract_string(pool: &BoxPool, v: &Value) -> Option<String> {
    let idx = v.as_box()?;
    match pool.get(idx)? {
        BoxedValue::String(s) => Some(s.clone()),
        _ => None,
    }
}
