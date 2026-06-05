use abrase::compiler::Compiler;
use abrase::lexer::Lexer;
use abrase::parser::Parser;
use myriad::devices::BufferConsole;
use myriad::devices::console::SharedBuf;
use myriad::{Host, Value, VirtualMachine, read_string};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen(getter_with_clone)]
pub struct RunResult {
    pub ok: bool,
    pub value: String,
    pub stdout: String,
    pub stderr: String,
    pub error: String,
    pub warnings: String,
}

impl RunResult {
    fn err(error: String, warnings: String) -> Self {
        Self {
            ok: false,
            value: String::new(),
            stdout: String::new(),
            stderr: String::new(),
            error,
            warnings,
        }
    }
}

fn compile(
    source: &str,
    int32: bool,
    no_built_in: bool,
) -> Result<(polka::Module, Compiler), (String, String)> {
    let mut parser = Parser::new(Lexer::new(source)).with_source(source.to_string());
    let ast = parser.parse_program();
    if !parser.errors.is_empty() {
        return Err((parser.pretty_print_errors(), String::new()));
    }
    let mut compiler = Compiler::new()
        .with_source(source.to_string())
        .with_int32_mode(int32)
        .with_no_built_in(no_built_in);
    match compiler.compile_module(&ast) {
        Ok(module) => Ok((module, compiler)),
        Err(_) => Err((compiler.pretty_print_errors(), render_warnings(&compiler))),
    }
}

fn drain(buf: &SharedBuf) -> String {
    let mut bytes = buf.borrow_mut();
    let s = String::from_utf8_lossy(&bytes).into_owned();
    bytes.clear();
    s
}

#[wasm_bindgen]
pub fn run(source: &str) -> RunResult {
    run_with(source, false, false)
}

#[wasm_bindgen]
pub fn run_with(source: &str, int32: bool, no_built_in: bool) -> RunResult {
    let (module, compiler) = match compile(source, int32, no_built_in) {
        Ok(x) => x,
        Err((error, warnings)) => return RunResult::err(error, warnings),
    };
    let warnings = render_warnings(&compiler);

    let console = BufferConsole::new();
    let (out_buf, err_buf) = console.handles();

    let mut vm = VirtualMachine::new().with_fn_names(compiler.fn_names());
    Host::headless()
        .with_console(Box::new(console))
        .install_into(&mut vm);

    match vm.run_module(&module) {
        Ok(v) => {
            let value = match read_string(vm.heap_ref(), v) {
                Some(s) => s,
                None => v.as_int().to_string(),
            };
            RunResult {
                ok: true,
                value,
                stdout: drain(&out_buf),
                stderr: drain(&err_buf),
                error: String::new(),
                warnings,
            }
        }
        Err(e) => {
            let mut res = RunResult::err(format!("runtime error: {}", e), warnings);
            res.stdout = drain(&out_buf);
            res.stderr = drain(&err_buf);
            res
        }
    }
}

#[wasm_bindgen]
pub fn check(source: &str) -> RunResult {
    check_with(source, false, false)
}

#[wasm_bindgen]
pub fn check_with(source: &str, int32: bool, no_built_in: bool) -> RunResult {
    let mut parser = Parser::new(Lexer::new(source)).with_source(source.to_string());
    let ast = parser.parse_program();
    if !parser.errors.is_empty() {
        return RunResult::err(parser.pretty_print_errors(), String::new());
    }
    let mut compiler = Compiler::new()
        .with_source(source.to_string())
        .with_int32_mode(int32)
        .with_no_built_in(no_built_in);
    compiler.run_typeck_only(&ast);
    let warnings = render_warnings(&compiler);
    if !compiler.errors.is_empty() {
        return RunResult::err(compiler.pretty_print_errors(), warnings);
    }
    RunResult {
        ok: true,
        value: String::new(),
        stdout: String::new(),
        stderr: String::new(),
        error: String::new(),
        warnings,
    }
}

#[wasm_bindgen]
pub struct Session {
    vm: VirtualMachine,
    module: polka::Module,
    out_buf: SharedBuf,
    err_buf: SharedBuf,
}

#[wasm_bindgen]
impl Session {
    #[wasm_bindgen(constructor)]
    pub fn new(source: &str) -> Result<Session, String> {
        let (module, compiler) = compile(source, false, false).map_err(|(e, _)| e)?;

        let console = BufferConsole::new();
        let (out_buf, err_buf) = console.handles();

        let mut vm = VirtualMachine::new().with_fn_names(compiler.fn_names());
        Host::headless()
            .with_console(Box::new(console))
            .install_into(&mut vm);

        vm.run_to_yield(&module)?;
        Ok(Session { vm, module, out_buf, err_buf })
    }

    pub fn resume(&mut self, input: i64) -> Result<bool, String> {
        self.vm.resume(&self.module, Value::from_int(input))
    }

    pub fn take_stdout(&mut self) -> String {
        drain(&self.out_buf)
    }

    pub fn take_stderr(&mut self) -> String {
        drain(&self.err_buf)
    }

    pub fn heap_live_count(&self) -> usize {
        self.vm.heap_live_count()
    }
}

pub const EXAMPLES: &[(&str, &str)] = &[
    ("merge_sort.abe", include_str!("../../../examples/merge_sort.abe")),
    ("mandelbrot.abe", include_str!("../../../examples/mandelbrot.abe")),
    ("nqueens.abe", include_str!("../../../examples/nqueens.abe")),
    ("tree_sum.abe", include_str!("../../../examples/tree_sum.abe")),
    ("dual_handler.abe", include_str!("../../../examples/dual_handler.abe")),
];

#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[wasm_bindgen]
pub fn example_names() -> Vec<String> {
    EXAMPLES.iter().map(|(n, _)| n.to_string()).collect()
}

#[wasm_bindgen]
pub fn example_source(name: &str) -> String {
    EXAMPLES
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, s)| s.to_string())
        .unwrap_or_default()
}

fn render_warnings(compiler: &Compiler) -> String {
    compiler
        .warnings
        .iter()
        .map(|w| format!("warning: {}", w.message))
        .collect::<Vec<_>>()
        .join("\n")
}
