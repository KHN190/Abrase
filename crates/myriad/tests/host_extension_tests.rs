use myriad::devices::{BufferConsole, Console};
use myriad::{Device, Host, Value, VirtualMachine};

struct AccDevice {
    sum: u64,
}
impl AccDevice {
    fn new() -> Self { Self { sum: 0 } }
}
impl Device for AccDevice {
    fn read(&mut self, _port: u8) -> Result<(Value, bool), String> {
        Ok((Value::from_int(self.sum as i64), false))
    }
    fn write(&mut self, _port: u8, val: Value, is_handle: bool, heap: &mut myriad::Heap) -> Result<(), String> {
        if is_handle { heap.rc_dec_handle(val.raw())?; return Ok(()); }
        self.sum = self.sum.wrapping_add((val.as_int() as u64) & 0xFF);
        Ok(())
    }
}

const ACC_ID: u8 = 0x42;

#[test]
fn host_headless_swaps_in_buffer_console() {
    let mut vm = VirtualMachine::new();
    Host::headless().install_into(&mut vm);
    let console: Box<dyn Console> = Box::new(BufferConsole::new());
    vm.install_device(0x10, Box::new(console));
}

#[test]
fn host_with_console_overrides_default() {
    let buf = BufferConsole::new();
    let out = buf.out_buf.clone();
    let console: Box<dyn Console> = Box::new(buf);

    let mut vm = VirtualMachine::new();
    Host::default()
        .with_console(console)
        .install_into(&mut vm);

    let dev = vm.take_device(0x10).expect("console installed");
    let mut dev = dev;
    let mut heap = myriad::Heap::new();
    dev.write(0x01, Value::from_int(b'A' as i64), false, &mut heap).unwrap();
    vm.install_device(0x10, dev);

    assert_eq!(&*out.borrow(), b"A");
}

#[test]
fn host_with_device_installs_extension_at_arbitrary_id() {
    let mut vm = VirtualMachine::new();
    Host::default()
        .with_device(ACC_ID, Box::new(AccDevice::new()))
        .install_into(&mut vm);

    let dev = vm.take_device(ACC_ID).expect("accumulator installed");
    let mut dev = dev;
    let mut heap = myriad::Heap::new();
    dev.write(0, Value::from_int(0x10), false, &mut heap).unwrap();
    dev.write(0, Value::from_int(0x20), false, &mut heap).unwrap();
    let (v, _) = dev.read(0).unwrap();
    assert_eq!(v.as_int(), 0x30);
}

#[test]
fn take_device_removes_from_table() {
    let mut vm = VirtualMachine::new();
    Host::default()
        .with_device(ACC_ID, Box::new(AccDevice::new()))
        .install_into(&mut vm);

    let first = vm.take_device(ACC_ID);
    assert!(first.is_some());

    let second = vm.take_device(ACC_ID);
    assert!(second.is_none());

    assert!(vm.take_device(0xAA).is_none());
}

#[test]
fn default_write_bytes_falls_back_to_per_byte_write() {
    let mut dev: Box<dyn Device> = Box::new(AccDevice::new());
    let mut heap = myriad::Heap::new();
    dev.write_bytes(0, &[1, 2, 3, 4], &mut heap).unwrap();
    assert_eq!(dev.read(0).unwrap().0.as_int(), 1 + 2 + 3 + 4);
}
