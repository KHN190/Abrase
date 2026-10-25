#![cfg(feature = "word32")]

use polka::{Value, HANDLE_NONE, HANDLE_SLOT_MAX};

#[test]
fn values_use_four_bytes_and_native_numeric_arguments() {
    assert_eq!(core::mem::size_of::<Value>(), 4);
    let value = Value::from_int(i32::MIN);
    let _: u32 = value.raw();
    let integer: i32 = value.as_int();
    assert_eq!(integer, i32::MIN);
    let float: f32 = Value::from_float(-0.0f32).as_float();
    assert_eq!(float.to_bits(), (-0.0f32).to_bits());
    assert_eq!(HANDLE_NONE, u32::MAX);
}

#[test]
fn logical_handles_preserve_high_slot_bit_and_generation() {
    assert_eq!(HANDLE_SLOT_MAX, 65534);
    for slot in [1, 32767, 32768, HANDLE_SLOT_MAX] {
        for generation in [0, 1, 65535] {
            let value = Value::from_handle(slot, generation);
            assert_eq!(value.as_handle(), (slot, generation));
            assert!(!value.is_handle_none());
        }
    }
}

#[test]
fn float_bits_round_trip_without_widening() {
    for bits in [0, 0x80000000, 1, 0x7f800001, 0x7fc01234, 0xff800000] {
        let value = Value::from_float(f32::from_bits(bits));
        assert_eq!(value.raw(), bits);
        assert_eq!(value.as_float().to_bits(), bits);
    }
}

#[test]
fn overflowing_handle_fields_are_rejected_instead_of_aliased() {
    for (slot, generation) in [(65535, 0), (65536, 0), (1, 65536)] {
        assert!(std::panic::catch_unwind(|| Value::from_handle(slot, generation)).is_err());
    }
}
