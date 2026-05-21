use polka::{Value, HANDLE_NONE, HANDLE_SLOT_MAX};

#[test]
fn value_is_8_bytes() {
    assert_eq!(std::mem::size_of::<Value>(), 8);
}

#[test]
fn int_round_trip() {
    for n in [0i64, 1, -1, 42, -42, i64::MAX, i64::MIN] {
        assert_eq!(Value::from_int(n).as_int(), n);
    }
}

#[test]
fn float_round_trip() {
    for f in [0.0, 1.5, -3.14, f64::INFINITY, -f64::INFINITY] {
        assert_eq!(Value::from_float(f).as_float(), f);
    }
}

#[test]
fn nan_float() {
    let v = Value::from_float(f64::NAN);
    assert!(v.as_float().is_nan());
}

#[test]
fn handle_round_trip() {
    let v = Value::from_handle(0xABCDEF, 0x123456);
    assert_eq!(v.as_handle(), (0xABCDEF, 0x123456));
}

#[test]
fn handle_none_distinct() {
    assert!(Value::NONE.is_handle_none());
    assert!(!Value::from_handle(0, 0).is_handle_none());
}

#[test]
fn bool_round_trip() {
    assert_eq!(Value::from_bool(true).as_bool(), true);
    assert_eq!(Value::from_bool(false).as_bool(), false);
    assert_eq!(Value::TRUE.as_bool(), true);
    assert_eq!(Value::FALSE.as_bool(), false);
    // Any non-zero raw is truthy.
    assert!(Value::from_raw(42).as_bool());
}

#[test]
fn char_round_trip() {
    for c in ['A', 'z', '0', '\n', 'é', '中', '🚀'] {
        assert_eq!(Value::from_char(c).as_char(), Some(c));
    }
}

#[test]
fn char_invalid_codepoint_returns_none() {
    // Surrogate range and beyond-Unicode values do not map to char.
    assert_eq!(Value::from_raw(0xD800).as_char(), None);
    assert_eq!(Value::from_raw(0x11_0000).as_char(), None);
}

#[test]
fn raw_round_trip() {
    for n in [0u64, 1, 0xDEADBEEF, u64::MAX] {
        assert_eq!(Value::from_raw(n).raw(), n);
    }
}

#[test]
fn const_aliases() {
    assert_eq!(Value::ZERO.raw(), 0);
    assert_eq!(Value::UNIT.raw(), 0);
    assert_eq!(Value::FALSE.raw(), 0);
    assert_eq!(Value::TRUE.raw(), 1);
    assert_eq!(Value::NONE.raw(), HANDLE_NONE);
}

#[test]
fn handle_slot_max_round_trips() {
    let v = Value::from_handle(HANDLE_SLOT_MAX, 0);
    assert_eq!(v.as_handle(), (HANDLE_SLOT_MAX, 0));
}

#[test]
fn handle_slot_overflow_truncates_to_24_bits() {
    let v = Value::from_handle(u32::MAX, u32::MAX);
    let (s, g) = v.as_handle();
    assert_eq!(s, 0x00FF_FFFF);
    assert_eq!(g, 0x00FF_FFFF);
}

#[test]
fn debug_format_distinguishes_none() {
    assert_eq!(format!("{:?}", Value::NONE), "None");
    let s = format!("{:?}", Value::from_int(0x42));
    assert!(s.starts_with("Value(0x"));
}

#[test]
fn equality_by_raw_bits() {
    assert_eq!(Value::ZERO, Value::FALSE);
    assert_eq!(Value::ZERO, Value::UNIT);
    assert_ne!(Value::ZERO, Value::TRUE);
    assert_ne!(Value::from_int(1), Value::from_float(1.0));
}
