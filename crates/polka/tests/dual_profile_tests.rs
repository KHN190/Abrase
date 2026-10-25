use polka::{word32, word64};
use std::{any::TypeId, mem::size_of};

#[test]
fn both_profiles_have_distinct_native_value_types_in_one_binary() {
    assert_eq!(size_of::<word32::Value>(), 4);
    assert_eq!(size_of::<word64::Value>(), 8);
    assert_ne!(TypeId::of::<word32::Value>(), TypeId::of::<word64::Value>());
    let make32: fn(i32) -> word32::Value = word32::Value::from_int;
    let make64: fn(i64) -> word64::Value = word64::Value::from_int;
    let _: u32 = make32(i32::MIN).raw();
    let _: u64 = make64(i64::MIN).raw();
    assert_eq!(make32(i32::MIN).as_int(), i32::MIN);
    assert_eq!(make64(i64::MIN).as_int(), i64::MIN);
}

#[test]
fn both_float_profiles_preserve_native_bits_without_conversion() {
    for bits in [0u32, 0x80000000, 1, 0x7f800001, 0x7fc01234] {
        let value = word32::Value::from_float(f32::from_bits(bits));
        let float: f32 = value.as_float();
        assert_eq!(value.raw(), bits);
        assert_eq!(float.to_bits(), bits);
    }
    for bits in [0u64, 0x8000000000000000, 1, 0x7ff0000000000001, 0x7ff8000000001234] {
        let value = word64::Value::from_float(f64::from_bits(bits));
        let float: f64 = value.as_float();
        assert_eq!(value.raw(), bits);
        assert_eq!(float.to_bits(), bits);
    }
}

#[test]
fn handles_use_their_profiles_own_layout_and_limits() {
    assert_eq!(word32::Value::from_handle(32768, 65535).raw(), 0x8000ffff);
    assert_eq!(word64::Value::from_handle(32768, 65535).raw(), 0x800000ffff);
    assert_eq!(word32::Value::from_handle(32768, 65535).as_handle(), (32768, 65535));
    assert_eq!(word64::Value::from_handle(32768, 65535).as_handle(), (32768, 65535));
    assert!(std::panic::catch_unwind(|| word32::Value::from_handle(65535, 0)).is_err());
    assert!(std::panic::catch_unwind(|| word32::Value::from_handle(1, 65536)).is_err());
    assert_eq!(word64::Value::from_handle(65536, 65536).as_handle(), (65536, 65536));
}
