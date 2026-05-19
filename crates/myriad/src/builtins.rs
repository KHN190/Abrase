use crate::{Heap, Value};
use crate::device::DeviceTable;
use crate::devices::{clock, console, random, CLOCK_ID, CONSOLE_ID, RANDOM_ID};
use crate::value::{alloc_string, read_string};
use std::collections::HashMap;
use std::rc::Rc;

pub struct NativeCtx<'a> {
    pub heap: &'a mut Heap,
    pub devices: &'a mut DeviceTable,
    pub halted: &'a mut bool,
    pub exit_code: &'a mut Option<i64>,
}

// (raw, is_handle) — interpreter updates caller's frame mask bit accordingly.
pub type NativeFn = Rc<dyn for<'a> Fn(&mut NativeCtx<'a>, &[Value]) -> Result<(Value, bool), String>>;

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
    reg.register("__concat",    concat_native());
    reg.register("__to_str",    to_str_native());
    reg.register("__float_abs", float_abs_native());
    reg.register("__float_max", float_max_native());
    reg.register("__float_min", float_min_native());
    reg.register("__int_to_f",    int_to_f_native());
    reg.register("__char_to_f",   char_to_f_native());
    reg.register("__bool_to_f",   bool_to_f_native());
    reg.register("__float_to_i",  float_to_i_native());
    reg.register("__char_to_i",   char_to_i_native());
    reg.register("__bool_to_i",   bool_to_i_native());
    reg.register("__int_to_c",    int_to_c_native());
    reg.register("__int_to_s",    int_to_s_native());
    reg.register("__float_to_s",  float_to_s_native());
    reg.register("__bool_to_s",   bool_to_s_native());
    reg.register("__char_to_s",   char_to_s_native());
    reg.register("__string_to_s", string_to_s_native());
    reg.register("__unit_to_s",   unit_to_s_native());

    reg.register("print",       print_native());
    reg.register("println",     println_native());

    reg.register("now",         now_native());
    reg.register("sleep_ms",    sleep_ms_native());

    reg.register("rand",        rand_native());
    reg.register("srand",       srand_native());

    reg.register("abs",         abs_native());
    reg.register("ceil",        ceil_native());
    reg.register("flr",         flr_native());
    reg.register("cos",         cos_native());
    reg.register("sin",         sin_native());
    reg.register("sqrt",        sqrt_native());
    reg.register("max",         max_native());
    reg.register("min",         min_native());

    reg.register("halt",        halt_native());
    reg.register("abort",       abort_native());
}

#[inline]
fn plain(v: Value) -> (Value, bool) { (v, false) }
#[inline]
fn handle(v: Value) -> (Value, bool) { (v, true) }

fn concat_native() -> NativeFn {
    Rc::new(|ctx, args| {
        let a = read_string(ctx.heap, args[0])
            .ok_or_else(|| format!("__concat: arg0 not a String: {:?}", args[0]))?;
        let b = read_string(ctx.heap, args[1])
            .ok_or_else(|| format!("__concat: arg1 not a String: {:?}", args[1]))?;
        let mut out = String::with_capacity(a.len() + b.len());
        out.push_str(&a);
        out.push_str(&b);
        let v = alloc_string(ctx.heap, &out)?;
        Ok(handle(v))
    })
}

fn to_str_native() -> NativeFn {
    // Fallback when compile-time dispatch can't determine type (effect-op returns, etc).
    Rc::new(|ctx, args| {
        if let Some(s) = read_string(ctx.heap, args[0]) {
            let v = alloc_string(ctx.heap, &s)?;
            return Ok(handle(v));
        }
        let v = alloc_string(ctx.heap, &args[0].as_int().to_string())?;
        Ok(handle(v))
    })
}

fn print_native() -> NativeFn {
    Rc::new(|ctx, args| {
        let s = read_string(ctx.heap, args[0])
            .ok_or_else(|| format!("print: arg0 not a String: {:?}", args[0]))?;
        write_console(ctx.devices, s.as_bytes(), "print")?;
        Ok(plain(Value::ZERO))
    })
}

fn println_native() -> NativeFn {
    Rc::new(|ctx, args| {
        let s = read_string(ctx.heap, args[0])
            .ok_or_else(|| format!("println: arg0 not a String: {:?}", args[0]))?;
        write_console(ctx.devices, s.as_bytes(), "println")?;
        write_console(ctx.devices, b"\n", "println")?;
        Ok(plain(Value::ZERO))
    })
}

fn write_console(devices: &mut DeviceTable, bytes: &[u8], op: &str) -> Result<(), String> {
    let dev = devices.get_mut(CONSOLE_ID)
        .ok_or_else(|| format!("{}: Console device 0x{:02x} not installed", op, CONSOLE_ID))?;
    for &b in bytes {
        dev.write(console::PORT_STDOUT, Value::from_int(b as i64))?;
    }
    Ok(())
}

fn now_native() -> NativeFn {
    Rc::new(|ctx, _args| {
        let dev = ctx.devices.get_mut(CLOCK_ID)
            .ok_or_else(|| format!("now: Clock device 0x{:02x} not installed", CLOCK_ID))?;
        let v = dev.read(clock::PORT_MONO_NS)?;
        Ok(plain(Value::from_int(v.as_int() / 1_000_000)))
    })
}

fn sleep_ms_native() -> NativeFn {
    Rc::new(|ctx, args| {
        let ms = args[0].as_int();
        let dev = ctx.devices.get_mut(CLOCK_ID)
            .ok_or_else(|| format!("sleep_ms: Clock device 0x{:02x} not installed", CLOCK_ID))?;
        dev.write(clock::PORT_SLEEP_MS, Value::from_int(ms))?;
        Ok(plain(Value::ZERO))
    })
}

fn rand_native() -> NativeFn {
    Rc::new(|ctx, _args| {
        let n = read_random_u64(ctx.devices)?;
        let m = n & ((1u64 << 53) - 1);
        let f = (m as f64) / ((1u64 << 53) as f64);
        Ok(plain(Value::from_float(f)))
    })
}

fn srand_native() -> NativeFn {
    Rc::new(|ctx, args| {
        let f = args[0].as_float();
        let bits = f.to_bits();
        let folded = bits ^ (bits >> 32);
        let dev = ctx.devices.get_mut(RANDOM_ID)
            .ok_or_else(|| format!("srand: Random device 0x{:02x} not installed", RANDOM_ID))?;
        dev.write(random::PORT_SEED, Value::from_raw(folded))?;
        Ok(plain(Value::ZERO))
    })
}

fn read_random_u64(devices: &mut DeviceTable) -> Result<u64, String> {
    let dev = devices.get_mut(RANDOM_ID)
        .ok_or_else(|| format!("rand: Random device 0x{:02x} not installed", RANDOM_ID))?;
    let v = dev.read(random::PORT_U64)?;
    Ok(v.raw())
}

fn halt_native() -> NativeFn {
    Rc::new(|ctx, args| {
        let code = args[0].as_int();
        *ctx.exit_code = Some(code & 0xFFFF_FFFF);
        *ctx.halted = true;
        Ok(plain(Value::ZERO))
    })
}

fn abort_native() -> NativeFn {
    Rc::new(|ctx, args| {
        let msg = read_string(ctx.heap, args[0])
            .ok_or_else(|| format!("abort: arg0 not a String: {:?}", args[0]))?;
        Err(format!("abort: {}", msg))
    })
}

fn abs_native() -> NativeFn {
    Rc::new(|_ctx, args| {
        let n = args[0].as_int();
        Ok(plain(Value::from_int(n.wrapping_abs())))
    })
}

fn max_native() -> NativeFn {
    Rc::new(|_ctx, args| {
        let a = args[0].as_int();
        let b = args[1].as_int();
        Ok(plain(Value::from_int(a.max(b))))
    })
}

fn min_native() -> NativeFn {
    Rc::new(|_ctx, args| {
        let a = args[0].as_int();
        let b = args[1].as_int();
        Ok(plain(Value::from_int(a.min(b))))
    })
}

fn float_abs_native() -> NativeFn {
    Rc::new(|_ctx, args| Ok(plain(Value::from_float(args[0].as_float().abs()))))
}

fn float_max_native() -> NativeFn {
    Rc::new(|_ctx, args| Ok(plain(Value::from_float(args[0].as_float().max(args[1].as_float())))))
}

fn float_min_native() -> NativeFn {
    Rc::new(|_ctx, args| Ok(plain(Value::from_float(args[0].as_float().min(args[1].as_float())))))
}

fn int_to_f_native() -> NativeFn {
    Rc::new(|_ctx, args| Ok(plain(Value::from_float(args[0].as_int() as f64))))
}

fn char_to_f_native() -> NativeFn {
    Rc::new(|_ctx, args| {
        let c = args[0].as_char().ok_or_else(|| format!("__char_to_f: arg0 not Char: {:?}", args[0]))?;
        Ok(plain(Value::from_float(c as u32 as f64)))
    })
}

fn bool_to_f_native() -> NativeFn {
    Rc::new(|_ctx, args| Ok(plain(Value::from_float(if args[0].as_bool() { 1.0 } else { 0.0 }))))
}

fn float_to_i_native() -> NativeFn {
    Rc::new(|_ctx, args| Ok(plain(Value::from_int(args[0].as_float() as i64))))
}

fn char_to_i_native() -> NativeFn {
    Rc::new(|_ctx, args| {
        let c = args[0].as_char().ok_or_else(|| format!("__char_to_i: arg0 not Char: {:?}", args[0]))?;
        Ok(plain(Value::from_int(c as i64)))
    })
}

fn bool_to_i_native() -> NativeFn {
    Rc::new(|_ctx, args| Ok(plain(Value::from_int(if args[0].as_bool() { 1 } else { 0 }))))
}

fn int_to_c_native() -> NativeFn {
    Rc::new(|_ctx, args| {
        let n = args[0].as_int();
        let u = u32::try_from(n).map_err(|_| format!("abort: invalid codepoint {}", n))?;
        let c = char::from_u32(u).ok_or_else(|| format!("abort: invalid codepoint U+{:X}", u))?;
        Ok(plain(Value::from_char(c)))
    })
}

fn int_to_s_native() -> NativeFn {
    Rc::new(|ctx, args| {
        let v = alloc_string(ctx.heap, &args[0].as_int().to_string())?;
        Ok(handle(v))
    })
}

fn float_to_s_native() -> NativeFn {
    Rc::new(|ctx, args| {
        let v = alloc_string(ctx.heap, &args[0].as_float().to_string())?;
        Ok(handle(v))
    })
}

fn bool_to_s_native() -> NativeFn {
    Rc::new(|ctx, args| {
        let v = alloc_string(ctx.heap, &args[0].as_bool().to_string())?;
        Ok(handle(v))
    })
}

fn char_to_s_native() -> NativeFn {
    Rc::new(|ctx, args| {
        let c = args[0].as_char().ok_or_else(|| format!("__char_to_s: arg0 not Char: {:?}", args[0]))?;
        let v = alloc_string(ctx.heap, &c.to_string())?;
        Ok(handle(v))
    })
}

fn string_to_s_native() -> NativeFn {
    Rc::new(|ctx, args| {
        let s = read_string(ctx.heap, args[0])
            .ok_or_else(|| format!("__string_to_s: arg0 not String: {:?}", args[0]))?;
        let v = alloc_string(ctx.heap, &s)?;
        Ok(handle(v))
    })
}

fn unit_to_s_native() -> NativeFn {
    Rc::new(|ctx, _args| {
        let v = alloc_string(ctx.heap, "()")?;
        Ok(handle(v))
    })
}

fn ceil_native() -> NativeFn {
    Rc::new(|_ctx, args| Ok(plain(Value::from_int(args[0].as_float().ceil() as i64))))
}

fn flr_native() -> NativeFn {
    Rc::new(|_ctx, args| Ok(plain(Value::from_int(args[0].as_float().floor() as i64))))
}

fn cos_native()  -> NativeFn { Rc::new(|_ctx, args| Ok(plain(Value::from_float(args[0].as_float().cos())))) }
fn sin_native()  -> NativeFn { Rc::new(|_ctx, args| Ok(plain(Value::from_float(args[0].as_float().sin())))) }
fn sqrt_native() -> NativeFn { Rc::new(|_ctx, args| Ok(plain(Value::from_float(args[0].as_float().sqrt())))) }
