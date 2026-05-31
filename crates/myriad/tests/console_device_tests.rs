use myriad::devices::{BufferConsole, Console, CONSOLE_ID};
use myriad::devices::console::{PORT_STDIN, PORT_STDOUT, PORT_STDERR, PORT_FLUSH};
use myriad::{Device, Heap, Value};

fn device() -> (Box<dyn Device>, std::rc::Rc<std::cell::RefCell<Vec<u8>>>, std::rc::Rc<std::cell::RefCell<Vec<u8>>>, std::rc::Rc<std::cell::RefCell<Vec<u8>>>) {
    let buf = BufferConsole::new();
    let (out, err) = buf.handles();
    let stdin = buf.stdin_handle();
    let d: Box<dyn Device> = Box::new(Box::new(buf) as Box<dyn Console>);
    (d, out, err, stdin)
}

fn h() -> Heap { Heap::new() }

#[test]
fn buffer_console_stdout_per_byte() {
    let (mut d, out, _err, _stdin) = device();
    let mut hp = h();
    d.write(PORT_STDOUT, Value::from_int(b'h' as i64), false, &mut hp).unwrap();
    d.write(PORT_STDOUT, Value::from_int(b'i' as i64), false, &mut hp).unwrap();
    assert_eq!(&*out.borrow(), b"hi");
}

#[test]
fn buffer_console_stderr_routed_separately() {
    let buf = BufferConsole::new();
    let (out, err) = buf.handles();
    let mut d: Box<dyn Device> = Box::new(Box::new(buf) as Box<dyn Console>);
    let mut hp = h();
    d.write(PORT_STDOUT, Value::from_int(b'A' as i64), false, &mut hp).unwrap();
    d.write(PORT_STDERR, Value::from_int(b'E' as i64), false, &mut hp).unwrap();
    assert_eq!(&*out.borrow(), b"A");
    assert_eq!(&*err.borrow(), b"E");
}

#[test]
fn buffer_console_low_byte_masking() {
    let buf = BufferConsole::new();
    let (out, _) = buf.handles();
    let mut d: Box<dyn Device> = Box::new(Box::new(buf) as Box<dyn Console>);
    let mut hp = h();
    d.write(PORT_STDOUT, Value::from_int(0x141), false, &mut hp).unwrap();
    assert_eq!(&*out.borrow(), &[0x41u8]);
}

#[test]
fn buffer_console_stdin_consumes_in_order() {
    let buf = BufferConsole::new();
    let stdin = buf.stdin_handle();
    stdin.borrow_mut().extend_from_slice(b"ab");
    let mut d: Box<dyn Device> = Box::new(Box::new(buf) as Box<dyn Console>);
    assert_eq!(d.read(PORT_STDIN).unwrap().0.as_int(), b'a' as i64);
    assert_eq!(d.read(PORT_STDIN).unwrap().0.as_int(), b'b' as i64);
    assert_eq!(d.read(PORT_STDIN).unwrap().0.as_int(), -1);
}

#[test]
fn buffer_console_unknown_read_port_returns_zero() {
    let buf = BufferConsole::new();
    let mut d: Box<dyn Device> = Box::new(Box::new(buf) as Box<dyn Console>);
    assert_eq!(d.read(0x7F).unwrap().0.as_int(), 0);
}

#[test]
fn buffer_console_flush_is_noop_and_succeeds() {
    let buf = BufferConsole::new();
    let (out, _) = buf.handles();
    let mut d: Box<dyn Device> = Box::new(Box::new(buf) as Box<dyn Console>);
    let mut hp = h();
    d.write(PORT_FLUSH, Value::from_int(0), false, &mut hp).unwrap();
    assert!(out.borrow().is_empty());
}

#[test]
fn buffer_console_write_bytes_bulk_stdout() {
    let buf = BufferConsole::new();
    let (out, _) = buf.handles();
    let mut d: Box<dyn Device> = Box::new(Box::new(buf) as Box<dyn Console>);
    let mut hp = h();
    d.write_bytes(PORT_STDOUT, b"hello", &mut hp).unwrap();
    assert_eq!(&*out.borrow(), b"hello");
}

#[test]
fn buffer_console_write_bytes_bulk_stderr() {
    let buf = BufferConsole::new();
    let (_, err) = buf.handles();
    let mut d: Box<dyn Device> = Box::new(Box::new(buf) as Box<dyn Console>);
    let mut hp = h();
    d.write_bytes(PORT_STDERR, b"err", &mut hp).unwrap();
    assert_eq!(&*err.borrow(), b"err");
}

#[test]
fn buffer_console_write_bytes_unknown_port_falls_through_to_write() {
    let buf = BufferConsole::new();
    let (out, _) = buf.handles();
    let mut d: Box<dyn Device> = Box::new(Box::new(buf) as Box<dyn Console>);
    let mut hp = h();
    d.write_bytes(0x77, b"X", &mut hp).unwrap();
    assert!(out.borrow().is_empty());
}

#[test]
fn console_id_constant() {
    assert_eq!(CONSOLE_ID, 0x10);
}
