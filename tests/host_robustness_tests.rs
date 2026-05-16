// host code must never panic on user input.
use abrase::myriad::{BoxPool, BoxedValue, Value};
use std::rc::Rc;

fn println_like(pool: &mut BoxPool, args: &[Value]) -> Result<Value, String> {
    let idx = args[0].as_box()
        .ok_or_else(|| format!("println: internal: args[0] not a Box ({:?})", args[0]))?;
    let s = match pool.get(idx) {
        Some(BoxedValue::String(s)) => s,
        other => return Err(format!("println: internal: box holds {:?}", other)),
    };
    Ok(Value::from_int(s.len() as i64))
}

#[test]
fn println_host_returns_err_on_non_box_arg() {
    let mut pool = BoxPool::new();
    let result = println_like(&mut pool, &[Value::from_int(42)]);
    assert!(result.is_err(), "must return Err, not panic");
    assert!(result.unwrap_err().contains("not a Box"));
}

#[test]
fn println_host_returns_err_on_wrong_box_type() {
    let mut pool = BoxPool::new();
    let idx = pool.intern(BoxedValue::Closure { func_id: 0, env_slot: 0, env_gen: 0 });
    let result = println_like(&mut pool, &[Value::from_box(idx)]);
    assert!(result.is_err(), "must return Err, not panic");
    assert!(result.unwrap_err().contains("box holds"));
}

#[test]
fn println_host_succeeds_on_string_box() {
    let mut pool = BoxPool::new();
    let idx = pool.intern(BoxedValue::String("hi".into()));
    let result = println_like(&mut pool, &[Value::from_box(idx)]);
    assert_eq!(result, Ok(Value::from_int(2)));
}

#[test]
fn hostimpl_signature_compatibility() {
    let f: Rc<dyn Fn(&mut BoxPool, &[Value]) -> Result<Value, String>>
        = Rc::new(println_like);
    let mut pool = BoxPool::new();
    let res = f(&mut pool, &[Value::UNIT]);
    assert!(res.is_err()); // UNIT is not a Box
}

#[test]
fn user_defined_print_shadows_builtin() {
    // 用户用同名 fn 覆盖内置 print:这里返回 42 而非内置的 Unit。
    let src = r#"
        fn print(s: String) -> Int { 42 }
        fn main() -> Int { print("ignored") }
    "#;
    let mut rt = abrase::myriad::host::Runtime::new();
    let v = rt.eval(src).expect("eval should succeed");
    assert_eq!(v, Value::from_int(42), "user-defined `print` must shadow built-in");
}

#[test]
fn builtin_print_runs_when_not_shadowed() {
    // 不定义同名 fn 时,调用内置 print(返回 Unit)。
    let src = r#"
        fn main() -> Int { print("hi"); 7 }
    "#;
    let mut rt = abrase::myriad::host::Runtime::new();
    let v = rt.eval(src).expect("eval should succeed");
    assert_eq!(v, Value::from_int(7));
}
