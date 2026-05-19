// host code must never panic on user input.
use myriad::{Heap, Value, alloc_string, read_string};
use std::rc::Rc;

fn println_like(heap: &mut Heap, args: &[u64]) -> Result<(u64, bool), String> {
    let v = Value::from_raw(args[0]);
    let s = read_string(heap, v)
        .ok_or_else(|| format!("println: internal: args[0] not a String handle ({:?})", v))?;
    Ok((Value::from_int(s.len() as i64).raw(), false))
}

#[test]
fn println_host_returns_err_on_non_handle_arg() {
    let mut heap = Heap::new();
    let result = println_like(&mut heap, &[Value::from_int(42).raw()]);
    assert!(result.is_err(), "must return Err, not panic");
}

#[test]
fn println_host_succeeds_on_string_handle() {
    let mut heap = Heap::new();
    let v = alloc_string(&mut heap, "hi").unwrap();
    let result = println_like(&mut heap, &[v.raw()]);
    assert_eq!(result, Ok((Value::from_int(2).raw(), false)));
}

#[test]
fn hostimpl_signature_compatibility() {
    let f: Rc<dyn Fn(&mut Heap, &[u64]) -> Result<(u64, bool), String>>
        = Rc::new(println_like);
    let mut heap = Heap::new();
    let res = f(&mut heap, &[Value::ZERO.raw()]);
    assert!(res.is_err());
}

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

#[test]
fn device_in_writes_byte_to_console() {
    let src = r#"
        fn main() -> Int {
            device_in(4097, 65);
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
    let src = r#"
        fn main() -> Int { device_out(57344) }
    "#;
    let mut rt = abrase_cli::host::Runtime::new();
    let result = rt.eval(src).expect("dispatch without prior lookup returns NO_MATCH");
    assert_eq!(result, Value::from_int(0xFFFF));
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
