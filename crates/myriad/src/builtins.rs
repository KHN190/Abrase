use crate::{Heap, Value};
use crate::devices::DeviceTable;
use crate::devices::{console, CONSOLE_ID};
use crate::value::{alloc_string, read_string};
use std::collections::BTreeMap;
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
    fns: BTreeMap<String, NativeFn>,
}

impl NativeRegistry {
    pub fn new() -> Self { Self { fns: BTreeMap::new() } }

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

    reg.register("__int_abs",   abs_native());
    reg.register("ceil",        ceil_native());
    reg.register("flr",         flr_native());
    reg.register("cos",         cos_native());
    reg.register("sin",         sin_native());
    reg.register("sqrt",        sqrt_native());
    reg.register("__int_max",   max_native());
    reg.register("__int_min",   min_native());

    reg.register("halt",        halt_native());
    reg.register("abort",       abort_native());
}

#[inline]
fn plain(v: Value) -> (Value, bool) { (v, false) }
#[inline]
fn handle(v: Value) -> (Value, bool) { (v, true) }

fn concat_native() -> NativeFn {
    Rc::new(|ctx, args| {
        if args[0].is_handle_none() {
            return Err(format!("__concat: arg0 not a String: {:?}", args[0]));
        }
        if args[1].is_handle_none() {
            return Err(format!("__concat: arg1 not a String: {:?}", args[1]));
        }
        let (a_slot, a_gen) = args[0].as_handle();
        let (b_slot, b_gen) = args[1].as_handle();

        // Clamp the user-stated length to the cell's actual payload capacity.
        // Without this, a malicious St on slot 0 could drive copy_nonoverlapping
        // past the cell buffer and into host memory.
        let a_len = {
            let d = ctx.heap.cell_data(a_slot, a_gen)?;
            (d[0] as usize).min(d.len().saturating_sub(1) * 8)
        };
        let b_len = {
            let d = ctx.heap.cell_data(b_slot, b_gen)?;
            (d[0] as usize).min(d.len().saturating_sub(1) * 8)
        };
        let total = a_len + b_len;
        let size = 1 + (total + 7) / 8;

        let (slot, gen_) = ctx.heap.try_alloc(size)?;

        // Source `Box<[u64]>` buffers stay put even if cells Vec reallocs,
        // so these raw pointers remain valid until next free of a/b.
        let a_src = ctx.heap.cell_data(a_slot, a_gen)?[1..].as_ptr() as *const u8;
        let b_src = ctx.heap.cell_data(b_slot, b_gen)?[1..].as_ptr() as *const u8;

        let dst = ctx.heap.cell_data_mut(slot, gen_)?;
        dst[0] = total as u64;
        let dst_ptr = dst[1..].as_mut_ptr() as *mut u8;
        unsafe {
            std::ptr::copy_nonoverlapping(a_src, dst_ptr, a_len);
            std::ptr::copy_nonoverlapping(b_src, dst_ptr.add(a_len), b_len);
        }

        Ok(handle(Value::from_handle(slot, gen_)))
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
        if args[0].is_handle_none() {
            return Err(format!("print: arg0 not a String: {:?}", args[0]));
        }
        let (slot, gen_) = args[0].as_handle();
        let bytes: Vec<u8> = {
            let d = ctx.heap.cell_data(slot, gen_)?;
            // Clamp stated length to actual cell payload capacity.
            let len = (d[0] as usize).min(d.len().saturating_sub(1) * 8);
            let ptr = d[1..].as_ptr() as *const u8;
            unsafe { std::slice::from_raw_parts(ptr, len).to_vec() }
        };
        write_console(ctx.devices, ctx.heap, &bytes, "print")?;
        Ok(plain(Value::ZERO))
    })
}

fn println_native() -> NativeFn {
    Rc::new(|ctx, args| {
        if args[0].is_handle_none() {
            return Err(format!("println: arg0 not a String: {:?}", args[0]));
        }
        let (slot, gen_) = args[0].as_handle();
        let bytes: Vec<u8> = {
            let d = ctx.heap.cell_data(slot, gen_)?;
            let len = (d[0] as usize).min(d.len().saturating_sub(1) * 8);
            let ptr = d[1..].as_ptr() as *const u8;
            unsafe { std::slice::from_raw_parts(ptr, len).to_vec() }
        };
        write_console(ctx.devices, ctx.heap, &bytes, "println")?;
        write_console(ctx.devices, ctx.heap, b"\n", "println")?;
        Ok(plain(Value::ZERO))
    })
}

fn write_console(devices: &mut DeviceTable, heap: &mut Heap, bytes: &[u8], op: &str) -> Result<(), String> {
    let dev = devices.get_mut(CONSOLE_ID)
        .ok_or_else(|| format!("{}: Console device 0x{:02x} not installed", op, CONSOLE_ID))?;
    dev.write_bytes(console::PORT_STDOUT, bytes, heap)
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
        if args[0].is_handle_none() {
            return Err(format!("__string_to_s: arg0 not String: {:?}", args[0]));
        }
        let (slot, gen_) = args[0].as_handle();
        ctx.heap.rc_inc(slot, gen_)?;
        Ok(handle(args[0]))
    })
}

fn unit_to_s_native() -> NativeFn {
    Rc::new(|ctx, _args| {
        let v = alloc_string(ctx.heap, "()")?;
        Ok(handle(v))
    })
}

fn ceil_native() -> NativeFn {
    Rc::new(|_ctx, args| Ok(plain(Value::from_float(args[0].as_float().ceil()))))
}

fn flr_native() -> NativeFn {
    Rc::new(|_ctx, args| Ok(plain(Value::from_float(args[0].as_float().floor()))))
}

fn cos_native()  -> NativeFn { Rc::new(|_ctx, args| Ok(plain(Value::from_float(args[0].as_float().cos())))) }
fn sin_native()  -> NativeFn { Rc::new(|_ctx, args| Ok(plain(Value::from_float(args[0].as_float().sin())))) }
fn sqrt_native() -> NativeFn { Rc::new(|_ctx, args| Ok(plain(Value::from_float(args[0].as_float().sqrt())))) }
