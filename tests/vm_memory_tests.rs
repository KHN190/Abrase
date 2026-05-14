// Value variants, clone/eq behaviour.
use ect::vm::Value;

#[test]
fn test_value_int_eq() {
    assert_eq!(Value::Int(1), Value::Int(1));
    assert_ne!(Value::Int(1), Value::Int(2));
}

#[test]
fn test_value_bool_variants() {
    assert_eq!(Value::Bool(true), Value::Bool(true));
    assert_ne!(Value::Bool(true), Value::Bool(false));
}

#[test]
fn test_value_tuple_eq() {
    let a = Value::Tuple(vec![Value::Int(1), Value::Bool(true)]);
    let b = Value::Tuple(vec![Value::Int(1), Value::Bool(true)]);
    assert_eq!(a, b);
}

#[test]
fn test_value_clone() {
    let v = Value::String("hello".into());
    assert_eq!(v.clone(), v);
}

#[test]
fn test_value_record_tag_distinguishes_variants() {
    let a = Value::Record { tag: 0, fields: vec![Value::Int(1)] };
    let b = Value::Record { tag: 1, fields: vec![Value::Int(1)] };
    assert_ne!(a, b);
}

#[test]
fn test_value_float_eq() {
    assert_eq!(Value::Float(3.14), Value::Float(3.14));
    assert_ne!(Value::Float(3.14), Value::Float(2.71));
}

#[test]
fn test_value_unit_eq() {
    assert_eq!(Value::Unit, Value::Unit);
}

#[test]
fn test_value_string_eq() {
    assert_eq!(Value::String("hello".into()), Value::String("hello".into()));
    assert_ne!(Value::String("hello".into()), Value::String("world".into()));
}

#[test]
fn test_value_cross_type_inequality() {
    assert_ne!(Value::Int(1), Value::Bool(true));
    assert_ne!(Value::Int(1), Value::Float(1.0));
    assert_ne!(Value::Bool(false), Value::Unit);
    assert_ne!(Value::String("1".into()), Value::Int(1));
}

#[test]
fn test_value_tuple_inequality() {
    let a = Value::Tuple(vec![Value::Int(1), Value::Bool(true)]);
    let b = Value::Tuple(vec![Value::Int(1), Value::Bool(false)]);
    assert_ne!(a, b);
}

#[test]
fn test_value_record_fields_distinguish() {
    let a = Value::Record { tag: 0, fields: vec![Value::Int(1)] };
    let b = Value::Record { tag: 0, fields: vec![Value::Int(2)] };
    assert_ne!(a, b);
}
