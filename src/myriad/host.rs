use std::rc::Rc;
use crate::compiler::{Compiler, HostFnDecl};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::ty::Type;
use crate::myriad::{BoxPool, BoxedValue, Value, VirtualMachine};
use crate::myriad::devices::{
    HostFuncDevice, HostImpl, HOSTFUNC_ID,
    Console, BufferConsole, StdoutConsole, CONSOLE_ID,
    SystemDevice, SYSTEM_ID,
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
        // typeck should guarantee args[0]: String.
        self.register_host("println", vec![Type::String], Type::Unit, |pool, args| {
            let s = host_arg_string(pool, args, "println")?;
            println!("{}", s);
            Ok(Value::UNIT)
        });
        // 不带换行;用户可定义自己的 fn print(...)(优先级高于内置),
        // 或直接用 dei/deo 访问 CONSOLE_ID 自行实现输出。
        self.register_host("print", vec![Type::String], Type::Unit, |pool, args| {
            let s = host_arg_string(pool, args, "print")?;
            print!("{}", s);
            Ok(Value::UNIT)
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

fn host_arg_string<'a>(pool: &'a BoxPool, args: &[Value], who: &str) -> Result<&'a str, String> {
    let idx = args[0].as_box()
        .ok_or_else(|| format!("{}: internal: args[0] not a Box ({:?})", who, args[0]))?;
    match pool.get(idx) {
        Some(BoxedValue::String(s)) => Ok(s.as_str()),
        other => Err(format!("{}: internal: box holds {:?}", who, other)),
    }
}
