// NaN-boxed Value: scalar equality and tag round-trips.
use abrase::vm::Value;

#[test]
fn test_value_int_eq() {
    assert_eq!(Value::from_int(1), Value::from_int(1));
    assert_ne!(Value::from_int(1), Value::from_int(2));
}

#[test]
fn test_value_bool_variants() {
    assert_eq!(Value::from_bool(true), Value::from_bool(true));
    assert_ne!(Value::from_bool(true), Value::from_bool(false));
}

#[test]
fn test_value_float_eq() {
    assert_eq!(Value::from_float(3.14), Value::from_float(3.14));
    assert_ne!(Value::from_float(3.14), Value::from_float(2.71));
}

#[test]
fn test_value_unit_eq() {
    assert_eq!(Value::UNIT, Value::UNIT);
}

#[test]
fn test_value_char_eq() {
    assert_eq!(Value::from_char('a'), Value::from_char('a'));
    assert_ne!(Value::from_char('a'), Value::from_char('b'));
}

#[test]
fn test_value_cross_type_inequality() {
    assert_ne!(Value::from_int(1), Value::from_bool(true));
    assert_ne!(Value::from_int(1), Value::from_float(1.0));
    assert_ne!(Value::from_bool(false), Value::UNIT);
}

#[test]
fn test_handle_round_trip() {
    let v = Value::from_handle(42, 7);
    assert_eq!(v.as_handle(), Some((42, 7)));
    assert!(v.is_handle());
}

#[test]
fn test_none_unit_distinct() {
    assert_ne!(Value::NONE, Value::UNIT);
    assert!(Value::NONE.is_none());
    assert!(Value::UNIT.is_unit());
}

#[test]
fn test_value_size_8_bytes() {
    assert_eq!(std::mem::size_of::<Value>(), 8);
}
