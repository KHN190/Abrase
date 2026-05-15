// Value variants, clone/eq behaviour.
use abrase::vm::Value;

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

#[test]
fn test_value_char_eq() {
    assert_eq!(Value::Char('a'), Value::Char('a'));
    assert_ne!(Value::Char('a'), Value::Char('b'));
}

#[test]
fn test_value_array_eq() {
    let a = Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
    let b = Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
    assert_eq!(a, b);
}

#[test]
fn test_value_array_ne() {
    let a = Value::Array(vec![Value::Int(1), Value::Int(2)]);
    let b = Value::Array(vec![Value::Int(1), Value::Int(3)]);
    assert_ne!(a, b);
}

#[test]
fn test_value_array_different_length() {
    let a = Value::Array(vec![Value::Int(1), Value::Int(2)]);
    let b = Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
    assert_ne!(a, b);
}

#[test]
fn test_value_closure_eq() {
    let a = Value::Closure {
        func_id: 0,
        env: vec![Value::Int(42)],
    };
    let b = Value::Closure {
        func_id: 0,
        env: vec![Value::Int(42)],
    };
    assert_eq!(a, b);
}

#[test]
fn test_value_closure_ne_different_func_id() {
    let a = Value::Closure {
        func_id: 0,
        env: vec![Value::Int(42)],
    };
    let b = Value::Closure {
        func_id: 1,
        env: vec![Value::Int(42)],
    };
    assert_ne!(a, b);
}

#[test]
fn test_value_closure_ne_different_env() {
    let a = Value::Closure {
        func_id: 0,
        env: vec![Value::Int(42)],
    };
    let b = Value::Closure {
        func_id: 0,
        env: vec![Value::Int(99)],
    };
    assert_ne!(a, b);
}

#[test]
fn test_value_reference_eq() {
    let a = Value::Reference(Box::new(Value::Int(42)));
    let b = Value::Reference(Box::new(Value::Int(42)));
    assert_eq!(a, b);
}

#[test]
fn test_value_reference_ne() {
    let a = Value::Reference(Box::new(Value::Int(42)));
    let b = Value::Reference(Box::new(Value::Int(99)));
    assert_ne!(a, b);
}

#[test]
fn test_value_reference_nested() {
    let a = Value::Reference(Box::new(Value::Reference(Box::new(Value::Int(42)))));
    let b = Value::Reference(Box::new(Value::Reference(Box::new(Value::Int(42)))));
    assert_eq!(a, b);
}

#[test]
fn test_value_mixed_types_inequality() {
    assert_ne!(Value::Char('a'), Value::Int(97));
    assert_ne!(Value::Array(vec![Value::Int(1)]), Value::Tuple(vec![Value::Int(1)]));
    assert_ne!(Value::Closure { func_id: 0, env: vec![] }, Value::Int(0));
    assert_ne!(Value::Reference(Box::new(Value::Int(1))), Value::Int(1));
}
