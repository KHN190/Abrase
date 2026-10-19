// Float register bits depend on the cart's int32 flag. Only polka's codec may
// touch the bit pattern; every VM / native / AOT site goes through it.

const BUILTINS: &str = include_str!("../src/builtins.rs");
const REGISTER: &str = include_str!("../src/interpreter/register.rs");
const AOT: &str = include_str!("../src/aot.rs");

fn offenders(src: &str, needles: &[&str]) -> Vec<String> {
    src.lines()
        .enumerate()
        .filter(|(_, l)| needles.iter().any(|n| l.contains(n)))
        .map(|(i, l)| format!("{}: {}", i + 1, l.trim()))
        .collect()
}

#[test]
fn natives_never_decode_floats_outside_native_ctx() {
    let bad = offenders(BUILTINS, &[".as_float()", "Value::from_float(", "from_float_f32(", "f64::from_bits", "f32::from_bits", ".to_bits()"]);
    assert!(bad.is_empty(), "builtins.rs must use NativeCtx::float / float_value:\n{}", bad.join("\n"));
}

#[test]
fn vm_registers_never_decode_floats_inline() {
    let bad = offenders(REGISTER, &["f64::from_bits", "f32::from_bits", ".to_bits()"]);
    assert!(bad.is_empty(), "register.rs must use polka::decode_float / encode_float:\n{}", bad.join("\n"));
}

#[test]
fn aot_host_forwards_int32_flag_to_natives() {
    assert!(AOT.contains("int32_safe: self.int32_safe"), "AotHost must pass its int32 flag into NativeCtx");
}

#[test]
fn codec_round_trips_in_both_modes() {
    for &f in &[0.0f64, -1.5, 2.5, 3.0, 1e-3, 12345.678] {
        assert_eq!(polka::decode_float(polka::encode_float(f, false), false), f);
        assert_eq!(polka::decode_float(polka::encode_float(f, true), true), f as f32 as f64);
    }
}
