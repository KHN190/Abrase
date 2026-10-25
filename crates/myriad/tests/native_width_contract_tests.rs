use myriad::{word32, word64};

fn call(name: &str, args: &[u64], int32_safe: bool) -> (u64, Option<String>) {
    macro_rules! invoke {
        ($profile:ident) => {{
            let mut registry = $profile::NativeRegistry::new();
            $profile::builtins::register_default_builtins(&mut registry);
            let mut heap = $profile::Heap::new();
            let mut devices = $profile::DeviceTable::new();
            let mut arena = $profile::CoreArena::new();
            let mut halted = false; let mut exit_code = None;
            let mut ctx = $profile::NativeCtx { heap: &mut heap, devices: &mut devices, core_arena: &mut arena,
                halted: &mut halted, exit_code: &mut exit_code, int32_safe };
            let arguments: Vec<_> = args.iter().map(|&raw| $profile::Value::from_raw(raw as $profile::Word)).collect();
            let (value, is_handle) = registry.get(name).unwrap()(&mut ctx, &arguments).unwrap();
            let text = if is_handle {
                let text = $profile::read_string(&heap, value).expect("string result");
                let (slot, generation) = value.as_handle(); heap.rc_dec(slot, generation).unwrap(); Some(text)
            } else { None };
            assert_eq!(heap.live_count(), 0);
            (value.raw() as u64, text)
        }};
    }
    if int32_safe { invoke!(word32) } else { invoke!(word64) }
}

fn integer_result(name: &str, args: &[u64], int32_safe: bool) -> i64 {
    let raw = call(name, args, int32_safe).0;
    if int32_safe { raw as u32 as i32 as i64 } else { raw as i64 }
}

fn convert(raw: u64, int32_safe: bool) -> i64 {
    integer_result("__float_to_i", &[raw], int32_safe)
}

#[test]
fn float32_to_int_saturates_at_i32_bounds() {
    for value in [f32::INFINITY, f32::NEG_INFINITY, f32::MAX, -f32::MAX,
                  2147483648.0, -2147483904.0] {
        assert_eq!(convert(u64::from(value.to_bits()), true), i64::from(value as i32), "{value}");
    }
}

#[test]
fn float32_to_int_truncates_and_maps_nan_to_zero() {
    for value in [f32::NAN, 0.0, -0.0, 3.9, -3.9, f32::from_bits(1), -2147483648.0] {
        assert_eq!(convert(u64::from(value.to_bits()), true), i64::from(value as i32), "{value}");
    }
}

#[test]
fn float64_to_int_retains_i64_bounds_and_large_values() {
    for value in [f64::INFINITY, f64::NEG_INFINITY, f64::MAX, -f64::MAX,
                  2147483648.0, -2147483904.0, f64::NAN, 3.9, -3.9] {
        assert_eq!(convert(value.to_bits(), false), value as i64, "{value}");
    }
}

#[test]
fn integer_natives_use_the_selected_signed_width() {
    for raw in [i32::MIN as i64 as u64, 0x80000000, u64::MAX, 0xffffffff, 0, i32::MAX as u64] {
        let n = raw as i32;
        assert_eq!(integer_result("__int_abs", &[raw], true), i64::from(n.wrapping_abs()));
        assert_eq!(integer_result("__int_max", &[raw, 0], true), i64::from(n.max(0)));
        assert_eq!(integer_result("__int_min", &[raw, 0], true), i64::from(n.min(0)));
        assert_eq!(integer_result("__int_abs", &[raw], false), (raw as i64).wrapping_abs());
    }
}

#[test]
fn int_char_bool_to_float_follow_the_selected_float_width() {
    for raw in [0, 1, 0xffffffff, 0x80000000, i32::MAX as u64, 16777217, u64::MAX] {
        assert_eq!(call("__int_to_f", &[raw], true).0, ((raw as i32) as f32).to_bits() as u64);
        assert_eq!(call("__int_to_f", &[raw], false).0, ((raw as i64) as f64).to_bits());
    }
    for code in [0, 65, 0x10ffff] {
        assert_eq!(call("__char_to_f", &[code], true).0, (code as f32).to_bits() as u64);
        assert_eq!(call("__char_to_f", &[code], false).0, (code as f64).to_bits());
        assert_eq!(call("__char_to_i", &[code], true).0, code);
    }
    for raw in [0, 1, u64::MAX] {
        let n = u8::from(raw != 0);
        assert_eq!(call("__bool_to_f", &[raw], true).0, (n as f32).to_bits() as u64);
        assert_eq!(call("__bool_to_f", &[raw], false).0, (n as f64).to_bits());
        assert_eq!(call("__bool_to_i", &[raw], true).0, n as u64);
    }
}

#[test]
fn numeric_strings_use_the_selected_precision_and_signed_width() {
    for value in [0.1f32, -0.0, f32::from_bits(1), f32::MAX, f32::INFINITY, f32::NEG_INFINITY, f32::NAN] {
        assert_eq!(call("__float_to_s", &[value.to_bits() as u64], true).1.unwrap(), value.to_string());
    }
    for value in [0.1f64, -0.0, f64::from_bits(1), f64::MAX, f64::INFINITY, f64::NAN] {
        assert_eq!(call("__float_to_s", &[value.to_bits()], false).1.unwrap(), value.to_string());
    }
    for raw in [0, 0xffffffff, 0x80000000, u64::MAX] {
        assert_eq!(call("__int_to_s", &[raw], true).1.unwrap(), (raw as i32).to_string());
        assert_eq!(call("__to_str", &[raw], true).1.unwrap(), (raw as i32).to_string());
        assert_eq!(call("__int_to_s", &[raw], false).1.unwrap(), (raw as i64).to_string());
    }
}

#[test]
fn float32_unary_math_matches_f32_functions_including_nan_payloads() {
    let operations: [(&str, fn(f32) -> f32); 4] = [
        ("__float_abs", libm::fabsf), ("flr", libm::floorf),
        ("ceil", libm::ceilf), ("sqrt", libm::sqrtf),
    ];
    for (name, operation) in operations {
        for raw in [0u32, 0x80000000, 1, 0x007fffff, 0x00800000, 0x3f800000, 0xbf800000,
            0x7f7fffff, 0xff7fffff, 0x7f800000, 0xff800000, 0x7f800001, 0xff800001, 0x7fc01234] {
            let expected = operation(f32::from_bits(raw));
            let got = call(name, &[raw as u64], true).0;
            if expected.is_nan() && name == "sqrt" { assert!(f32::from_bits(got as u32).is_nan()); }
            else { assert_eq!(got, expected.to_bits() as u64, "{name}: {raw:#x}"); }
        }
    }
}

#[test]
fn float32_min_max_keep_nan_and_zero_rules_of_the_selected_library() {
    for (name, operation) in [("__float_min", libm::fminf as fn(f32, f32) -> f32), ("__float_max", libm::fmaxf)] {
        for a in [0u32, 0x80000000, 1, 0x7f800000, 0x7fc01234] {
            for b in [0u32, 0x80000000, 1, 0xff800000, 0x7fc05678] {
                let expected = operation(f32::from_bits(a), f32::from_bits(b));
                let got = call(name, &[a as u64, b as u64], true).0;
                if expected.is_nan() { assert!(f32::from_bits(got as u32).is_nan()); }
                else { assert_eq!(got, expected.to_bits() as u64, "{name}: {a:#x}, {b:#x}"); }
            }
        }
    }
}

#[test]
fn default_float_math_keeps_f64_results() {
    let operations: [(&str, fn(f64) -> f64); 4] = [
        ("__float_abs", libm::fabs), ("flr", libm::floor), ("ceil", libm::ceil), ("sqrt", libm::sqrt),
    ];
    for (name, operation) in operations {
        for value in [0.0f64, -0.0, f64::from_bits(1), 0.1, 2.0, f64::MAX, f64::INFINITY] {
            assert_eq!(call(name, &[value.to_bits()], false).0, operation(value).to_bits());
        }
    }
}
