use super::runtime;
use runtime::{Word, Int, Float, Heap, Value};
use runtime::core_mem::CoreArena;
use runtime::devices::DeviceTable;
use runtime::devices::{console, CONSOLE_ID};
use runtime::value::{alloc_string, read_string, alloc_bytes, read_bytes};
use alloc::collections::BTreeMap;
use alloc::rc::Rc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

pub use runtime::fmath;

pub struct NativeCtx<'a> {
    pub heap: &'a mut Heap,
    pub devices: &'a mut DeviceTable,
    pub core_arena: &'a mut CoreArena,
    pub halted: &'a mut bool,
    pub exit_code: &'a mut Option<Int>,
    pub int32_safe: bool,
}

impl<'a> NativeCtx<'a> {
    #[inline(always)]
    pub fn float(&self, v: Value) -> Float { Float::from_bits(v.raw()) }

    #[inline(always)]
    pub fn float_value(&self, f: Float) -> Value { Value::from_raw(f.to_bits()) }

    pub fn integer(&self, v: Value) -> Int {
        v.as_int()
    }

    fn integer_unary(&self, v: Value, operation: fn(Int) -> Int) -> Value {
        let result = operation(v.as_int());
        Value::from_int(result)
    }

    fn integer_binary(&self, a: Value, b: Value, operation: fn(Int, Int) -> Int) -> Value {
        let result = operation(a.as_int(), b.as_int());
        Value::from_int(result)
    }

    fn float_unary(&self, v: Value, operation: fn(Float) -> Float) -> Value {
        let raw = operation(Float::from_bits(v.raw())).to_bits();
        Value::from_raw(raw)
    }

    fn float_binary(&self, a: Value, b: Value, operation: fn(Float, Float) -> Float) -> Value {
        let raw = operation(Float::from_bits(a.raw()), Float::from_bits(b.raw())).to_bits();
        Value::from_raw(raw)
    }

    fn int_to_float(&self, v: Value) -> Value {
        let raw = (v.as_int() as Float).to_bits();
        Value::from_raw(raw)
    }

    fn unsigned_to_float(&self, n: u32) -> Value {
        let raw = (n as Float).to_bits();
        Value::from_raw(raw)
    }

    fn float_to_int(&self, v: Value) -> Value {
        let result = Float::from_bits(v.raw()) as Int;
        Value::from_int(result)
    }

    fn integer_string(&self, v: Value) -> String {
        v.as_int().to_string()
    }

    fn float_string(&self, v: Value) -> String {
        Float::from_bits(v.raw()).to_string()
    }
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
    reg.register("__str_eq",      str_eq_native());
    reg.register("__str_len",     str_len_native());
    reg.register("__str_byte_at", str_byte_at_native());
    reg.register("__str_slice",   str_slice_native());
    reg.register("__str_to_bytes", str_to_bytes_native());
    reg.register("__bytes_concat", bytes_concat_native());
    reg.register("__bytes_len",   bytes_len_native());
    reg.register("__bytes_byte_at", bytes_byte_at_native());
    reg.register("__bytes_slice", bytes_slice_native());

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

    reg.register("__peek8",     peek_native(1));
    reg.register("__peek32",    peek_native(4));
    reg.register("__peek64",    peek_native(8));
    reg.register("__poke8",     poke_native(1));
    reg.register("__poke32",    poke_native(4));
    reg.register("__poke64",    poke_native(8));
    reg.register("__ptr_add",   ptr_add_native());
    reg.register("__arena_base", arena_base_native());

    reg.register("halt",        halt_native());
    reg.register("abort",       abort_native());

    // Frame-yield intrinsic: do_call intercepts it before the body runs, so this
    // registration only makes the name resolvable like any other runtime native.
    reg.register("__frame_present", frame_present_native());
}

fn peek_native(width: Word) -> NativeFn {
    Rc::new(move |ctx, args| {
        let addr = args[0].raw();
        let v = ctx.core_arena.peek(addr, width)?;
        Ok(plain(Value::from_raw(v)))
    })
}

fn poke_native(width: Word) -> NativeFn {
    Rc::new(move |ctx, args| {
        let addr = args[0].raw();
        let val = args[1].raw();
        ctx.core_arena.poke(addr, width, val)?;
        Ok(plain(Value::from_int(0)))
    })
}

fn ptr_add_native() -> NativeFn {
    Rc::new(|_ctx, args| {
        let addr = args[0].raw();
        let delta = args[1].as_int();
        Ok(plain(Value::from_raw(CoreArena::ptr_add(addr, delta))))
    })
}

fn arena_base_native() -> NativeFn {
    Rc::new(|_ctx, _args| Ok(plain(Value::from_raw(0))))
}

fn frame_present_native() -> NativeFn {
    Rc::new(|_ctx, _args| Err("__frame_present must be intercepted as a frame yield".into()))
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
            (d[0] as usize).min(d.len().saturating_sub(1) * runtime::WORD_BYTES)
        };
        let b_len = {
            let d = ctx.heap.cell_data(b_slot, b_gen)?;
            (d[0] as usize).min(d.len().saturating_sub(1) * runtime::WORD_BYTES)
        };
        let total = a_len + b_len;
        let size = 1 + total.div_ceil(runtime::WORD_BYTES);

        let (slot, gen_) = ctx.heap.try_alloc(size)?;

        // Source `Box<[Word]>` buffers stay put even if cells Vec reallocs,
        // so these raw pointers remain valid until next free of a/b.
        let a_src = ctx.heap.cell_data(a_slot, a_gen)?[1..].as_ptr() as *const u8;
        let b_src = ctx.heap.cell_data(b_slot, b_gen)?[1..].as_ptr() as *const u8;

        let dst = ctx.heap.cell_data_mut(slot, gen_)?;
        dst[0] = total as Word;
        let dst_ptr = dst[1..].as_mut_ptr() as *mut u8;
        unsafe {
            core::ptr::copy_nonoverlapping(a_src, dst_ptr, a_len);
            core::ptr::copy_nonoverlapping(b_src, dst_ptr.add(a_len), b_len);
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
        let v = alloc_string(ctx.heap, &ctx.integer_string(args[0]))?;
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
            let len = (d[0] as usize).min(d.len().saturating_sub(1) * runtime::WORD_BYTES);
            let ptr = d[1..].as_ptr() as *const u8;
            unsafe { core::slice::from_raw_parts(ptr, len).to_vec() }
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
            let len = (d[0] as usize).min(d.len().saturating_sub(1) * runtime::WORD_BYTES);
            let ptr = d[1..].as_ptr() as *const u8;
            unsafe { core::slice::from_raw_parts(ptr, len).to_vec() }
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
        *ctx.exit_code = Some(code as u32 as Int);
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
    Rc::new(|ctx, args| Ok(plain(ctx.integer_unary(args[0], Int::wrapping_abs))))
}

fn max_native() -> NativeFn {
    Rc::new(|ctx, args| Ok(plain(ctx.integer_binary(args[0], args[1], Int::max))))
}

fn min_native() -> NativeFn {
    Rc::new(|ctx, args| Ok(plain(ctx.integer_binary(args[0], args[1], Int::min))))
}

fn float_abs_native() -> NativeFn {
    Rc::new(|ctx, args| Ok(plain(ctx.float_unary(args[0], fmath::abs))))
}

fn float_max_native() -> NativeFn {
    Rc::new(|ctx, args| Ok(plain(ctx.float_binary(args[0], args[1], fmath::fmax))))
}

fn float_min_native() -> NativeFn {
    Rc::new(|ctx, args| Ok(plain(ctx.float_binary(args[0], args[1], fmath::fmin))))
}

fn int_to_f_native() -> NativeFn {
    Rc::new(|ctx, args| Ok(plain(ctx.int_to_float(args[0]))))
}

fn char_to_f_native() -> NativeFn {
    Rc::new(|ctx, args| {
        let c = args[0].as_char().ok_or_else(|| format!("__char_to_f: arg0 not Char: {:?}", args[0]))?;
        Ok(plain(ctx.unsigned_to_float(c as u32)))
    })
}

fn bool_to_f_native() -> NativeFn {
    Rc::new(|ctx, args| Ok(plain(ctx.unsigned_to_float(u32::from(args[0].as_bool())))))
}

fn float_to_i_native() -> NativeFn {
    Rc::new(|ctx, args| Ok(plain(ctx.float_to_int(args[0]))))
}

fn char_to_i_native() -> NativeFn {
    Rc::new(|_ctx, args| {
        let c = args[0].as_char().ok_or_else(|| format!("__char_to_i: arg0 not Char: {:?}", args[0]))?;
        Ok(plain(Value::from_int(c as Int)))
    })
}

fn bool_to_i_native() -> NativeFn {
    Rc::new(|_ctx, args| Ok(plain(Value::from_int(if args[0].as_bool() { 1 } else { 0 }))))
}

fn int_to_c_native() -> NativeFn {
    Rc::new(|ctx, args| {
        let n = ctx.integer(args[0]);
        let u = u32::try_from(n).map_err(|_| format!("abort: invalid codepoint {}", n))?;
        let c = char::from_u32(u).ok_or_else(|| format!("abort: invalid codepoint U+{:X}", u))?;
        Ok(plain(Value::from_char(c)))
    })
}

fn int_to_s_native() -> NativeFn {
    Rc::new(|ctx, args| {
        let v = alloc_string(ctx.heap, &ctx.integer_string(args[0]))?;
        Ok(handle(v))
    })
}

fn float_to_s_native() -> NativeFn {
    Rc::new(|ctx, args| {
        let v = alloc_string(ctx.heap, &ctx.float_string(args[0]))?;
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

fn str_len_native() -> NativeFn {
    Rc::new(|ctx, args| {
        let n = read_string(ctx.heap, args[0]).map(|s| s.len()).unwrap_or(0);
        Ok(plain(Value::from_int(n as Int)))
    })
}

fn str_byte_at_native() -> NativeFn {
    Rc::new(|ctx, args| {
        let i = args[1].as_int();
        let s = read_string(ctx.heap, args[0]).unwrap_or_default();
        let bytes = s.as_bytes();
        if i < 0 || i as usize >= bytes.len() {
            return Err(format!("byte_at: index {} out of bounds for length {}", i, bytes.len()));
        }
        Ok(plain(Value::from_int(bytes[i as usize] as Int)))
    })
}

fn str_slice_native() -> NativeFn {
    Rc::new(|ctx, args| {
        let s = read_string(ctx.heap, args[0]).unwrap_or_default();
        let bytes = s.as_bytes();
        let off = args[1].as_int();
        let len = args[2].as_int();
        let start = off.max(0).min(bytes.len() as Int) as usize;
        let take = len.max(0) as usize;
        let end = start.saturating_add(take).min(bytes.len());
        let out = String::from_utf8_lossy(&bytes[start..end]);
        let v = alloc_string(ctx.heap, &out)?;
        Ok(handle(v))
    })
}

fn str_to_bytes_native() -> NativeFn {
    Rc::new(|ctx, args| {
        let b = read_bytes(ctx.heap, args[0]).unwrap_or_default();
        let v = alloc_bytes(ctx.heap, &b)?;
        Ok(handle(v))
    })
}

fn bytes_concat_native() -> NativeFn {
    Rc::new(|ctx, args| {
        let (slot, gen_) = args[1].as_handle();
        let parts: Vec<Word> = ctx.heap.cell_data(slot, gen_)?.to_vec();
        if parts.is_empty() {
            return Err("bytes concat: parts array is empty".to_string());
        }
        let mut out = read_bytes(ctx.heap, args[0]).unwrap_or_default();
        for &word in &parts {
            if let Some(b) = read_bytes(ctx.heap, Value::from_raw(word)) {
                out.extend_from_slice(&b);
            }
        }
        let v = alloc_bytes(ctx.heap, &out)?;
        Ok(handle(v))
    })
}

fn bytes_len_native() -> NativeFn {
    Rc::new(|ctx, args| {
        let n = read_bytes(ctx.heap, args[0]).map(|b| b.len()).unwrap_or(0);
        Ok(plain(Value::from_int(n as Int)))
    })
}

fn bytes_byte_at_native() -> NativeFn {
    Rc::new(|ctx, args| {
        let i = args[1].as_int();
        let b = read_bytes(ctx.heap, args[0]).unwrap_or_default();
        if i < 0 || i as usize >= b.len() {
            return Err(format!("byte_at: index {} out of bounds for length {}", i, b.len()));
        }
        Ok(plain(Value::from_int(b[i as usize] as Int)))
    })
}

fn bytes_slice_native() -> NativeFn {
    Rc::new(|ctx, args| {
        let b = read_bytes(ctx.heap, args[0]).unwrap_or_default();
        let off = args[1].as_int();
        let len = args[2].as_int();
        let start = off.max(0).min(b.len() as Int) as usize;
        let take = len.max(0) as usize;
        let end = start.saturating_add(take).min(b.len());
        let v = alloc_bytes(ctx.heap, &b[start..end])?;
        Ok(handle(v))
    })
}

fn str_eq_native() -> NativeFn {
    Rc::new(|ctx, args| {
        let a = read_string(ctx.heap, args[0]);
        let b = read_string(ctx.heap, args[1]);
        Ok(plain(Value::from_bool(a == b)))
    })
}

fn unit_to_s_native() -> NativeFn {
    Rc::new(|ctx, _args| {
        let v = alloc_string(ctx.heap, "()")?;
        Ok(handle(v))
    })
}

fn ceil_native() -> NativeFn {
    Rc::new(|ctx, args| Ok(plain(ctx.float_unary(args[0], fmath::ceil))))
}

fn flr_native() -> NativeFn {
    Rc::new(|ctx, args| Ok(plain(ctx.float_unary(args[0], fmath::floor))))
}

fn cos_native()  -> NativeFn { Rc::new(|ctx, args| Ok(plain(ctx.float_value(fmath::cos(ctx.float(args[0])))))) }
fn sin_native()  -> NativeFn { Rc::new(|ctx, args| Ok(plain(ctx.float_value(fmath::sin(ctx.float(args[0])))))) }
fn sqrt_native() -> NativeFn { Rc::new(|ctx, args| Ok(plain(ctx.float_unary(args[0], fmath::sqrt)))) }
