use std::io::{Read, Write};
use abrase::compiler::Compiler;
use abrase::lexer::Lexer;
use abrase::parser::Parser;
use myriad::{Value, VirtualMachine};
use myriad::devices::{
    Console, BufferConsole, CONSOLE_ID,
    SystemDevice, SYSTEM_ID,
};

/// Real stdio console. Lives in the CLI (std) host, not in no_std myriad.
pub struct StdoutConsole;

impl Console for StdoutConsole {
    fn read_byte(&mut self) -> Result<Option<u8>, String> {
        let mut byte = [0u8];
        match std::io::stdin().read(&mut byte) {
            Ok(0) => Ok(None),
            Ok(_) => Ok(Some(byte[0])),
            Err(e) => Err(format!("console.stdin: {}", e)),
        }
    }
    fn write_stdout(&mut self, byte: u8) -> Result<(), String> {
        std::io::stdout().write_all(&[byte]).map_err(|e| format!("console.stdout: {}", e))
    }
    fn write_stderr(&mut self, byte: u8) -> Result<(), String> {
        std::io::stderr().write_all(&[byte]).map_err(|e| format!("console.stderr: {}", e))
    }
    fn flush(&mut self) -> Result<(), String> {
        std::io::stdout().flush().map_err(|e| format!("flush: {}", e))?;
        std::io::stderr().flush().map_err(|e| format!("flush: {}", e))
    }
    fn write_stdout_bulk(&mut self, bytes: &[u8]) -> Result<(), String> {
        std::io::stdout().write_all(bytes).map_err(|e| format!("console.stdout: {}", e))
    }
    fn write_stderr_bulk(&mut self, bytes: &[u8]) -> Result<(), String> {
        std::io::stderr().write_all(bytes).map_err(|e| format!("console.stderr: {}", e))
    }
}

/// Text sink for VM diagnostics (TRACE_SLOT/TRACE_STATIC/trace_frames). Writes
/// a line to stderr; passed to `VirtualMachine::with_heap_trace`/`with_trace_out`.
pub fn eprintln_sink(s: &str) {
    eprintln!("{}", s);
}

/// myriad op-trace sink (was `myriad::debug::stderr_sink`). Lives here because
/// it needs std stderr; myriad only defines the abstract `DebugSink` type.
pub fn myriad_stderr_sink() -> myriad::DebugSink {
    use myriad::debug::{DebugEvent, render_fn_label};
    Box::new(|event: &DebugEvent, names: &[String]| {
        match event {
            DebugEvent::Trace { func, pc, op, line, .. } => {
                if *line > 0 {
                    eprintln!("[{}:{} @{}] {:?}", render_fn_label(*func, names), pc, line, op);
                } else {
                    eprintln!("[{}:{}] {:?}", render_fn_label(*func, names), pc, op);
                }
            }
            DebugEvent::HandlePush {
                effect_id, cell_slot, cell_gen, suspend_pc, suspend_base, dest, depth,
            } => {
                eprintln!(
                    "  [handle] push effect_id={} cell=(slot {},gen {}) suspend_pc={} suspend_base={} dest=r{} depth={}",
                    effect_id, cell_slot, cell_gen, suspend_pc, suspend_base, dest, depth
                );
            }
            DebugEvent::Resume {
                saved_pc, saved_base, cell_dest, val, handler_dest, alive, depth,
            } => {
                eprintln!(
                    "  [resume] -> saved_pc={} saved_base={} cell_dest=r{} val={:?} handler_dest=r{} alive={:?} depth={}",
                    saved_pc, saved_base, cell_dest, val, handler_dest, alive, depth
                );
            }
        }
    })
}

/// Install the standard stdio host (system + real console) onto a VM. Replaces
/// the old `myriad::Host::default()`, which now defaults to a headless buffer.
pub fn install_std_devices(vm: &mut VirtualMachine) {
    vm.install_device(SYSTEM_ID, Box::new(SystemDevice::new()));
    vm.install_device(CONSOLE_ID, Box::new(Box::new(StdoutConsole) as Box<dyn Console>));
}

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
