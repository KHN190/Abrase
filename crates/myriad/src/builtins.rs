use crate::{BoxPool, BoxedValue, Value};
use crate::device::DeviceTable;
use crate::devices::{clock, console, random, CLOCK_ID, CONSOLE_ID, RANDOM_ID};
use std::collections::HashMap;
use std::rc::Rc;

pub struct NativeCtx<'a> {
    pub pool: &'a mut BoxPool,
    pub devices: &'a mut DeviceTable,
    pub halted: &'a mut bool,
    pub exit_code: &'a mut Option<i64>,
}

pub type NativeFn = Rc<dyn for<'a> Fn(&mut NativeCtx<'a>, &[Value]) -> Result<Value, String>>;

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
    // Compiler-internal. Compiler decides fn by type signature.
    reg.register("__concat",    concat_native());
    reg.register("__to_str",    to_str_native());
    reg.register("__float_add", float_add_native());
    reg.register("__float_sub", float_sub_native());
    reg.register("__float_mul", float_mul_native());
    reg.register("__float_div", float_div_native());
    reg.register("__float_lt",  float_lt_native());
    reg.register("__float_neg", float_neg_native());
    reg.register("__float_abs", float_abs_native());
    reg.register("__float_max", float_max_native());
    reg.register("__float_min", float_min_native());
    // Type conversion natives (method bodies for ToF / ToI / ToC / ToS traits).
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

    // Console
    reg.register("print",       print_native());
    reg.register("println",     println_native());

    // Clock
    reg.register("now",         now_native());
    reg.register("sleep_ms",    sleep_ms_native());

    // Random
    reg.register("rand",        rand_native());
    reg.register("srand",       srand_native());

    // Math 
    reg.register("abs",         abs_native());
    reg.register("ceil",        ceil_native());
    reg.register("flr",         flr_native());
    reg.register("cos",         cos_native());
    reg.register("sin",         sin_native());
    reg.register("sqrt",        sqrt_native());
    reg.register("max",         max_native());
    reg.register("min",         min_native());

    // System
    reg.register("halt",        halt_native());
    reg.register("abort",       abort_native());
}

fn concat_native() -> NativeFn {
    Rc::new(|ctx: &mut NativeCtx<'_>, args: &[Value]| {
        let a = extract_string(ctx.pool, &args[0])
            .ok_or_else(|| format!("__concat: arg0 not a String: {:?}", args[0]))?;
        let b = extract_string(ctx.pool, &args[1])
            .ok_or_else(|| format!("__concat: arg1 not a String: {:?}", args[1]))?;
        let mut out = String::with_capacity(a.len() + b.len());
        out.push_str(&a);
        out.push_str(&b);
        let idx = ctx.pool.intern(BoxedValue::String(out));
        Ok(Value::from_box(idx))
    })
}

fn to_str_native() -> NativeFn {
    Rc::new(|ctx: &mut NativeCtx<'_>, args: &[Value]| {
        let v = &args[0];
        // read_int covers both inline TAG_INT and BoxedValue::Int (i48 overflow).
        let s = if let Some(n) = ctx.pool.read_int(*v) {
            n.to_string()
        } else if let Some(f) = v.as_float() {
            f.to_string()
        } else if let Some(b) = v.as_bool() {
            b.to_string()
        } else if let Some(c) = v.as_char() {
            c.to_string()
        } else if v.is_unit() {
            "()".to_string()
        } else if let Some(s) = extract_string(ctx.pool, v) {
            s
        } else {
            return Err(format!("__to_str: cannot convert {:?}", v));
        };
        let idx = ctx.pool.intern(BoxedValue::String(s));
        Ok(Value::from_box(idx))
    })
}

fn float_pair(args: &[Value], op: &str) -> Result<(f64, f64), String> {
    let a = args[0].as_float().ok_or_else(|| format!("{}: arg0 not Float: {:?}", op, args[0]))?;
    let b = args[1].as_float().ok_or_else(|| format!("{}: arg1 not Float: {:?}", op, args[1]))?;
    Ok((a, b))
}

fn float_add_native() -> NativeFn {
    Rc::new(|_ctx, args| {
        let (a, b) = float_pair(args, "__float_add")?;
        Ok(Value::from_float(a + b))
    })
}

fn float_sub_native() -> NativeFn {
    Rc::new(|_ctx, args| {
        let (a, b) = float_pair(args, "__float_sub")?;
        Ok(Value::from_float(a - b))
    })
}

fn float_mul_native() -> NativeFn {
    Rc::new(|_ctx, args| {
        let (a, b) = float_pair(args, "__float_mul")?;
        Ok(Value::from_float(a * b))
    })
}

fn float_div_native() -> NativeFn {
    Rc::new(|_ctx, args| {
        let (a, b) = float_pair(args, "__float_div")?;
        Ok(Value::from_float(a / b))
    })
}

fn float_lt_native() -> NativeFn {
    Rc::new(|_ctx, args| {
        let (a, b) = float_pair(args, "__float_lt")?;
        let r = if a.is_nan() || b.is_nan() { false } else { a < b };
        Ok(Value::from_bool(r))
    })
}

fn float_neg_native() -> NativeFn {
    Rc::new(|_ctx, args| {
        let f = args[0].as_float().ok_or_else(|| format!("__float_neg: arg0 not Float: {:?}", args[0]))?;
        Ok(Value::from_float(-f))
    })
}

fn print_native() -> NativeFn {
    Rc::new(|ctx: &mut NativeCtx<'_>, args: &[Value]| {
        let s = extract_string(ctx.pool, &args[0])
            .ok_or_else(|| format!("print: arg0 not a String: {:?}", args[0]))?;
        write_console(ctx.devices, s.as_bytes(), "print")?;
        Ok(Value::UNIT)
    })
}

fn println_native() -> NativeFn {
    Rc::new(|ctx: &mut NativeCtx<'_>, args: &[Value]| {
        let s = extract_string(ctx.pool, &args[0])
            .ok_or_else(|| format!("println: arg0 not a String: {:?}", args[0]))?;
        write_console(ctx.devices, s.as_bytes(), "println")?;
        write_console(ctx.devices, b"\n", "println")?;
        Ok(Value::UNIT)
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

// Monotonic milliseconds since VM start. Derived from mono_ns / 1_000_000.
fn now_native() -> NativeFn {
    Rc::new(|ctx: &mut NativeCtx<'_>, _args: &[Value]| {
        let dev = ctx.devices.get_mut(CLOCK_ID)
            .ok_or_else(|| format!("now: Clock device 0x{:02x} not installed", CLOCK_ID))?;
        let v = dev.read(clock::PORT_MONO_NS)?;
        let ns = v.as_int().ok_or_else(|| format!("now: clock returned non-int {:?}", v))?;
        Ok(Value::from_int(ns / 1_000_000))
    })
}

fn sleep_ms_native() -> NativeFn {
    Rc::new(|ctx: &mut NativeCtx<'_>, args: &[Value]| {
        let ms = args[0].as_int()
            .ok_or_else(|| format!("sleep_ms: arg0 not Int: {:?}", args[0]))?;
        let dev = ctx.devices.get_mut(CLOCK_ID)
            .ok_or_else(|| format!("sleep_ms: Clock device 0x{:02x} not installed", CLOCK_ID))?;
        dev.write(clock::PORT_SLEEP_MS, Value::from_int(ms))?;
        Ok(Value::UNIT)
    })
}

// f64 in [0, 1). Random device's PORT_U64 narrows to 47 bits at the port
// boundary (i48 inline-Value limit); 47-bit precision is way past f64-in-[0,1)
// usability (24 bits is enough for graphics).
fn rand_native() -> NativeFn {
    Rc::new(|ctx: &mut NativeCtx<'_>, _args: &[Value]| {
        let n = read_random_u64(ctx.devices)?;
        let f = (n as f64) / ((1u64 << 47) as f64);
        Ok(Value::from_float(f))
    })
}

// Seed from a Float (intended [0, 1) but any non-NaN works).
fn srand_native() -> NativeFn {
    Rc::new(|ctx: &mut NativeCtx<'_>, args: &[Value]| {
        let f = args[0].as_float()
            .ok_or_else(|| format!("srand: arg0 not Float: {:?}", args[0]))?;
        let bits = f.to_bits();
        let folded = (bits ^ (bits >> 32)) & ((1u64 << 47) - 1);
        let dev = ctx.devices.get_mut(RANDOM_ID)
            .ok_or_else(|| format!("srand: Random device 0x{:02x} not installed", RANDOM_ID))?;
        dev.write(random::PORT_SEED, Value::from_int(folded as i64))?;
        Ok(Value::UNIT)
    })
}

fn read_random_u64(devices: &mut DeviceTable) -> Result<u64, String> {
    let dev = devices.get_mut(RANDOM_ID)
        .ok_or_else(|| format!("rand: Random device 0x{:02x} not installed", RANDOM_ID))?;
    let v = dev.read(random::PORT_U64)?;
    let n = v.as_int().ok_or_else(|| format!("rand: device returned non-int {:?}", v))?;
    Ok(n as u64)
}

fn halt_native() -> NativeFn {
    Rc::new(|ctx: &mut NativeCtx<'_>, args: &[Value]| {
        let code = args[0].as_int()
            .ok_or_else(|| format!("halt: arg0 not Int: {:?}", args[0]))?;
        *ctx.exit_code = Some(code & 0xFFFF_FFFF);
        *ctx.halted = true;
        Ok(Value::UNIT)
    })
}

// Unrecoverable abort with stderr-bound message. Distinct from `halt`: halt is
// a clean exit; abort is "this should never happen" — propagates as Err past
// any effect handler.
fn abort_native() -> NativeFn {
    Rc::new(|ctx: &mut NativeCtx<'_>, args: &[Value]| {
        let msg = extract_string(ctx.pool, &args[0])
            .ok_or_else(|| format!("abort: arg0 not a String: {:?}", args[0]))?;
        Err(format!("abort: {}", msg))
    })
}

fn abs_native() -> NativeFn {
    Rc::new(|ctx: &mut NativeCtx<'_>, args: &[Value]| {
        let n = ctx.pool.read_int(args[0])
            .ok_or_else(|| format!("abs: arg0 not Int: {:?}", args[0]))?;
        Ok(ctx.pool.intern_int(n.wrapping_abs()))
    })
}

fn max_native() -> NativeFn {
    Rc::new(|ctx: &mut NativeCtx<'_>, args: &[Value]| {
        let a = ctx.pool.read_int(args[0])
            .ok_or_else(|| format!("max: arg0 not Int: {:?}", args[0]))?;
        let b = ctx.pool.read_int(args[1])
            .ok_or_else(|| format!("max: arg1 not Int: {:?}", args[1]))?;
        Ok(ctx.pool.intern_int(a.max(b)))
    })
}

fn min_native() -> NativeFn {
    Rc::new(|ctx: &mut NativeCtx<'_>, args: &[Value]| {
        let a = ctx.pool.read_int(args[0])
            .ok_or_else(|| format!("min: arg0 not Int: {:?}", args[0]))?;
        let b = ctx.pool.read_int(args[1])
            .ok_or_else(|| format!("min: arg1 not Int: {:?}", args[1]))?;
        Ok(ctx.pool.intern_int(a.min(b)))
    })
}

fn float_abs_native() -> NativeFn {
    Rc::new(|_ctx, args| {
        let f = args[0].as_float()
            .ok_or_else(|| format!("__float_abs: arg0 not Float: {:?}", args[0]))?;
        Ok(Value::from_float(f.abs()))
    })
}

fn float_max_native() -> NativeFn {
    Rc::new(|_ctx, args| {
        let (a, b) = float_pair(args, "__float_max")?;
        Ok(Value::from_float(a.max(b)))
    })
}

fn float_min_native() -> NativeFn {
    Rc::new(|_ctx, args| {
        let (a, b) = float_pair(args, "__float_min")?;
        Ok(Value::from_float(a.min(b)))
    })
}

// Type conversion natives.

fn int_to_f_native() -> NativeFn {
    Rc::new(|ctx, args| {
        let n = ctx.pool.read_int(args[0])
            .ok_or_else(|| format!("__int_to_f: arg0 not Int: {:?}", args[0]))?;
        Ok(Value::from_float(n as f64))
    })
}

fn char_to_f_native() -> NativeFn {
    Rc::new(|_ctx, args| {
        let c = args[0].as_char()
            .ok_or_else(|| format!("__char_to_f: arg0 not Char: {:?}", args[0]))?;
        Ok(Value::from_float(c as u32 as f64))
    })
}

fn bool_to_f_native() -> NativeFn {
    Rc::new(|_ctx, args| {
        let b = args[0].as_bool()
            .ok_or_else(|| format!("__bool_to_f: arg0 not Bool: {:?}", args[0]))?;
        Ok(Value::from_float(if b { 1.0 } else { 0.0 }))
    })
}

fn float_to_i_native() -> NativeFn {
    Rc::new(|ctx, args| {
        let f = args[0].as_float()
            .ok_or_else(|| format!("__float_to_i: arg0 not Float: {:?}", args[0]))?;
        Ok(ctx.pool.intern_int(f as i64))
    })
}

fn char_to_i_native() -> NativeFn {
    Rc::new(|ctx, args| {
        let c = args[0].as_char()
            .ok_or_else(|| format!("__char_to_i: arg0 not Char: {:?}", args[0]))?;
        Ok(ctx.pool.intern_int(c as i64))
    })
}

fn bool_to_i_native() -> NativeFn {
    Rc::new(|_ctx, args| {
        let b = args[0].as_bool()
            .ok_or_else(|| format!("__bool_to_i: arg0 not Bool: {:?}", args[0]))?;
        Ok(Value::from_int(if b { 1 } else { 0 }))
    })
}

fn int_to_c_native() -> NativeFn {
    Rc::new(|ctx, args| {
        let n = ctx.pool.read_int(args[0])
            .ok_or_else(|| format!("__int_to_c: arg0 not Int: {:?}", args[0]))?;
        let u = u32::try_from(n).map_err(|_| format!("abort: invalid codepoint {}", n))?;
        let c = char::from_u32(u).ok_or_else(|| format!("abort: invalid codepoint U+{:X}", u))?;
        Ok(Value::from_char(c))
    })
}

fn intern_str(pool: &mut BoxPool, s: String) -> Value {
    let idx = pool.intern(BoxedValue::String(s));
    Value::from_box(idx)
}

fn int_to_s_native() -> NativeFn {
    Rc::new(|ctx, args| {
        let n = ctx.pool.read_int(args[0])
            .ok_or_else(|| format!("__int_to_s: arg0 not Int: {:?}", args[0]))?;
        Ok(intern_str(ctx.pool, n.to_string()))
    })
}

fn float_to_s_native() -> NativeFn {
    Rc::new(|ctx, args| {
        let f = args[0].as_float()
            .ok_or_else(|| format!("__float_to_s: arg0 not Float: {:?}", args[0]))?;
        Ok(intern_str(ctx.pool, f.to_string()))
    })
}

fn bool_to_s_native() -> NativeFn {
    Rc::new(|ctx, args| {
        let b = args[0].as_bool()
            .ok_or_else(|| format!("__bool_to_s: arg0 not Bool: {:?}", args[0]))?;
        Ok(intern_str(ctx.pool, b.to_string()))
    })
}

fn char_to_s_native() -> NativeFn {
    Rc::new(|ctx, args| {
        let c = args[0].as_char()
            .ok_or_else(|| format!("__char_to_s: arg0 not Char: {:?}", args[0]))?;
        Ok(intern_str(ctx.pool, c.to_string()))
    })
}

fn string_to_s_native() -> NativeFn {
    Rc::new(|ctx, args| {
        let s = extract_string(ctx.pool, &args[0])
            .ok_or_else(|| format!("__string_to_s: arg0 not String: {:?}", args[0]))?;
        Ok(intern_str(ctx.pool, s))
    })
}

fn unit_to_s_native() -> NativeFn {
    Rc::new(|ctx, args| {
        if !args[0].is_unit() {
            return Err(format!("__unit_to_s: arg0 not Unit: {:?}", args[0]));
        }
        Ok(intern_str(ctx.pool, "()".into()))
    })
}

// ceil / flr — Float → Int.
fn ceil_native() -> NativeFn {
    Rc::new(|ctx: &mut NativeCtx<'_>, args: &[Value]| {
        let f = args[0].as_float()
            .ok_or_else(|| format!("ceil: arg0 not Float: {:?}", args[0]))?;
        Ok(ctx.pool.intern_int(f.ceil() as i64))
    })
}

fn flr_native() -> NativeFn {
    Rc::new(|ctx: &mut NativeCtx<'_>, args: &[Value]| {
        let f = args[0].as_float()
            .ok_or_else(|| format!("flr: arg0 not Float: {:?}", args[0]))?;
        Ok(ctx.pool.intern_int(f.floor() as i64))
    })
}

// cos / sin / sqrt — Float → Float.
fn cos_native() -> NativeFn {
    Rc::new(|_ctx: &mut NativeCtx<'_>, args: &[Value]| {
        let f = args[0].as_float()
            .ok_or_else(|| format!("cos: arg0 not Float: {:?}", args[0]))?;
        Ok(Value::from_float(f.cos()))
    })
}

fn sin_native() -> NativeFn {
    Rc::new(|_ctx: &mut NativeCtx<'_>, args: &[Value]| {
        let f = args[0].as_float()
            .ok_or_else(|| format!("sin: arg0 not Float: {:?}", args[0]))?;
        Ok(Value::from_float(f.sin()))
    })
}

fn sqrt_native() -> NativeFn {
    Rc::new(|_ctx: &mut NativeCtx<'_>, args: &[Value]| {
        let f = args[0].as_float()
            .ok_or_else(|| format!("sqrt: arg0 not Float: {:?}", args[0]))?;
        Ok(Value::from_float(f.sqrt()))
    })
}

fn extract_string(pool: &BoxPool, v: &Value) -> Option<String> {
    let idx = v.as_box()?;
    match pool.get(idx)? {
        BoxedValue::String(s) => Some(s.clone()),
        _ => None,
    }
}
