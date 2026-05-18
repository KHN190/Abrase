// host code must never panic on user input.
use myriad::{BoxPool, BoxedValue, Value};
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

// `print` is now a built-in native (registered by myriad/builtins.rs and the
// abrase compiler's register_builtins). User fns can't redefine it — this test
// is intentionally inverted from its older form.
#[test]
fn user_cannot_define_print() {
    let src = r#"
        fn print(s: String) -> Int { 42 }
        fn main() -> Int { 0 }
    "#;
    let mut rt = abrase_cli::host::Runtime::new();
    let err = rt.eval(src).expect_err("must reject user fn shadowing builtin `print`");
    assert!(err.contains("print"),
        "error should mention print; got: {}", err);
}

// device_in and device_out are mandatory host fns. User fns cannot reuse
// these names — compile_module rejects the decl.
#[test]
fn user_cannot_shadow_device_in() {
    let src = r#"
        fn device_in(port: Int, data: Int) -> Unit { () }
        fn main() -> Int { 0 }
    "#;
    let mut rt = abrase_cli::host::Runtime::new();
    let err = rt.eval(src).expect_err("must reject user fn shadowing `device_in`");
    assert!(err.contains("device_in"),
        "error should mention device_in; got: {}", err);
}

// End-to-end: .abe source calling device_in / device_out lowers to Deo / Dei
// against the Runtime's installed devices. Stdout = port 0x10_01 (4097).
#[test]
fn device_in_writes_byte_to_console() {
    let src = r#"
        fn main() -> Int {
            device_in(4097, 65);  // 'A' to stdout
            0
        }
    "#;
    let (mut rt, console) = abrase_cli::host::Runtime::new_for_tests();
    let (out_handle, _) = console.handles();
    let v = rt.eval(src).expect("device_in to console must succeed");
    assert_eq!(v, Value::from_int(0));
    let buf = out_handle.borrow();
    assert_eq!(&buf[..], b"A", "stdout should contain 'A'; got {:?}", &buf[..]);
}

#[test]
fn device_out_reads_back_dispatch_state() {
    // 0xE0_00 is the dispatch device's lookup port. Without a prior write,
    // device_out should return DISPATCH_NO_MATCH (0xFFFF)
    let src = r#"
        fn main() -> Int { device_out(57344) }
    "#;
    let mut rt = abrase_cli::host::Runtime::new();
    let result = rt.eval(src).expect("dispatch without prior lookup returns NO_MATCH");
    assert_eq!(result, Value::from_int(0xFFFF), "dispatch with no handler should return DISPATCH_NO_MATCH (0xFFFF)");
}

#[test]
fn user_cannot_shadow_device_out() {
    let src = r#"
        fn device_out(port: Int) -> Int { 0 }
        fn main() -> Int { 0 }
    "#;
    let mut rt = abrase_cli::host::Runtime::new();
    let err = rt.eval(src).expect_err("must reject user fn shadowing `device_out`");
    assert!(err.contains("device_out"),
        "error should mention device_out; got: {}", err);
}
