use abrase::compiler::Compiler;
use abrase::lexer::Lexer;
use abrase::parser::Parser;
use myriad::{Value, VirtualMachine};
use myriad::devices::{
    Console, BufferConsole, CONSOLE_ID,
    SystemDevice, SYSTEM_ID,
};

#[derive(Default)]
pub struct VmOptions {
    pub trace: bool,
    pub handlers: bool,
    pub leak: bool,
    pub cart: bool,
    pub print_value: bool,
    pub exit_on_halt: bool,
    pub fn_names: Vec<String>,
    pub static_names: Vec<String>,
}

pub fn eprintln_sink(text: &str) { eprintln!("{}", text); }

pub mod word32 {
    use myriad::word32 as vm;
    fn is_handle_mask(mask: &vm::TraceMask, index: usize) -> bool { index < 128 && mask[index / 32] & (1 << (index % 32)) != 0 }
    include!("profile_host.rs");
}
pub mod word64 {
    use myriad::word64 as vm;
    fn is_handle_mask(mask: &vm::TraceMask, index: usize) -> bool { index < 128 && mask & (1u128 << index) != 0 }
    include!("profile_host.rs");
}
pub use word64::{StdoutConsole, install_std_devices, myriad_stderr_sink};

pub struct Runtime {
    vm: VirtualMachine,
}

impl Runtime {
    pub fn new() -> Self {
        let mut vm = VirtualMachine::new();
        vm.install_device(SYSTEM_ID, Box::new(SystemDevice::new()));
        let console: Box<dyn Console> = Box::new(StdoutConsole);
        vm.install_device(CONSOLE_ID, Box::new(console));
        Self { vm }
    }

    pub fn new_for_tests() -> (Self, BufferConsole) {
        let mut vm = VirtualMachine::new();
        vm.install_device(SYSTEM_ID, Box::new(SystemDevice::new()));
        let console = BufferConsole::new();
        let console_clone = BufferConsole {
            out_buf: console.out_buf.clone(),
            err_buf: console.err_buf.clone(),
            stdin_buf: console.stdin_buf.clone(),
        };
        let boxed: Box<dyn Console> = Box::new(console);
        vm.install_device(CONSOLE_ID, Box::new(boxed));
        (Self { vm }, console_clone)
    }

    pub fn eval(&mut self, source: &str) -> Result<Value, String> {
        let mut parser = Parser::new(Lexer::new(source)).with_source(source.into());
        let ast = parser.parse_program();
        if !parser.errors.is_empty() {
            return Err(parser.pretty_print_errors());
        }
        let mut compiler = Compiler::new().with_source(source.into());
        let module = compiler.compile_module(&ast).map_err(|errs| {
            errs.iter().map(|e| e.pretty_print(source)).collect::<Vec<_>>().join("\n")
        })?;
        self.vm.run_module(&module)
    }
}
