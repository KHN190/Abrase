// Native codecs depend on the cart profile; opcode arithmetic uses native-width floats.

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
    let natives = BUILTINS.split("pub type NativeFn").nth(1).expect("native entry points");
    let bad = offenders(natives, &[".as_float()", "Value::from_float(", "from_float_f32(", "f64::from_bits", "f32::from_bits", ".to_bits()"]);
    assert!(bad.is_empty(), "native entry points must use NativeCtx numeric operations:\n{}", bad.join("\n"));
}

#[test]
fn vm_arithmetic_never_uses_the_widening_compatibility_codec() {
    let bad = offenders(REGISTER, &["polka::decode_float", "polka::encode_float"]);
    assert!(bad.is_empty(), "opcode arithmetic must keep the selected native float width:\n{}", bad.join("\n"));
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
