use std::rc::Rc;
use abrase::compiler::{Compiler, HostFnDecl};
use abrase::lexer::Lexer;
use abrase::parser::Parser;
use abrase::ty::Type;
use myriad::{BoxPool, Value, VirtualMachine};
use myriad::devices::{
    HostFuncDevice, HostImpl, HOSTFUNC_ID,
    Console, BufferConsole, StdoutConsole, CONSOLE_ID,
    SystemDevice, SYSTEM_ID,
    Clock, SystemClock, CLOCK_ID,
    Random, SystemRandom, RANDOM_ID,
};

pub struct Runtime {
    vm: VirtualMachine,
    pending_host: Vec<HostFnDecl>,
    pending_impls: Vec<HostImpl>,
}

impl Runtime {
    pub fn new() -> Self {
        let mut vm = VirtualMachine::new();
        vm.install_device(SYSTEM_ID, Box::new(SystemDevice::new()));
        let console: Box<dyn Console> = Box::new(StdoutConsole);
        vm.install_device(CONSOLE_ID, Box::new(console));
        let clock: Box<dyn Clock> = Box::new(SystemClock::new());
        vm.install_device(CLOCK_ID, Box::new(clock));
        let rng: Box<dyn Random> = Box::new(SystemRandom::new());
        vm.install_device(RANDOM_ID, Box::new(rng));
        let mut rt = Self { vm, pending_host: Vec::new(), pending_impls: Vec::new() };
        rt.register_default_hosts();
        rt
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
        let clock: Box<dyn Clock> = Box::new(SystemClock::new());
        vm.install_device(CLOCK_ID, Box::new(clock));
        let rng: Box<dyn Random> = Box::new(SystemRandom::new());
        vm.install_device(RANDOM_ID, Box::new(rng));
        let mut rt = Self { vm, pending_host: Vec::new(), pending_impls: Vec::new() };
        rt.register_default_hosts();
        (rt, console_clone)
    }

    pub fn register_host<F>(&mut self, name: &str, params: Vec<Type>, ret: Type, func: F)
    where F: Fn(&mut BoxPool, &[Value]) -> Result<Value, String> + 'static
    {
        let fn_id = self.pending_host.len() as u16;
        self.pending_host.push(HostFnDecl {
            name: name.into(),
            params,
            ret,
            fn_id,
        });
        self.pending_impls.push(Rc::new(func));
    }

    fn register_default_hosts(&mut self) {
        // `device_in` and `device_out` are the language contract — Myriad
        // MUST register them at startup. 
        //
        //   device_in(port: Int, data: Int) -> Unit   // write to device
        //   device_out(port: Int) -> Int              // read from device
        //
        // Any other host fn (print, circle, dot, ...) is optional. User fns
        // are NOT allowed to redefine names that are registered as host fns.
        self.register_host("device_in", vec![Type::Int, Type::Int], Type::Unit, |_pool, _args| {
            Err("device_in: must be lowered to Deo by codegen; closure should never run".into())
        });
        self.register_host("device_out", vec![Type::Int], Type::Int, |_pool, _args| {
            Err("device_out: must be lowered to Dei by codegen; closure should never run".into())
        });
    }

    pub fn eval(&mut self, source: &str) -> Result<Value, String> {
        let mut parser = Parser::new(Lexer::new(source)).with_source(source.into());
        let ast = parser.parse_program();
        if !parser.errors.is_empty() {
            return Err(parser.pretty_print_errors());
        }
        let mut compiler = Compiler::new().with_source(source.into());
        for decl in &self.pending_host {
            compiler.register_host_fn(decl.clone());
        }
        let module = compiler.compile_module(&ast).map_err(|errs| {
            errs.iter().map(|e| format!("{:?}", e)).collect::<Vec<_>>().join("\n")
        })?;

        if module.requires_device(HOSTFUNC_ID) {
            let mut dev = HostFuncDevice::new();
            for f in &self.pending_impls {
                dev.register(f.clone());
            }
            self.vm.install_device(HOSTFUNC_ID, Box::new(dev));
        }

        self.vm.run_module(&module)
    }
}

