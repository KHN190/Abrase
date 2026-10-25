use std::io::{Read, Write};
use std::process::ExitCode;
use vm::{VirtualMachine, Value, read_string};
use vm::devices::{Console, CONSOLE_ID, SystemDevice, SYSTEM_ID};
use super::{VmOptions, eprintln_sink};

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

/// myriad op-trace sink (was `myriad::debug::stderr_sink`). Lives here because
/// it needs std stderr; myriad only defines the abstract `DebugSink` type.
pub fn myriad_stderr_sink() -> vm::DebugSink {
    use vm::debug::{DebugEvent, render_fn_label};
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

pub fn execute(vm: VirtualMachine, module: &polka::Module, options: VmOptions) -> ExitCode {
    let trace_filter = std::env::var("TRACE_FN").ok().filter(|_| !options.fn_names.is_empty()).map(|spec| {
        let mut bits = vec![false; options.fn_names.len()];
        for wanted in spec.split(',') {
            match options.fn_names.iter().position(|name| name == wanted.trim()) {
                Some(i) => bits[i] = true,
                None => eprintln!("TRACE_FN: unknown fn '{}'", wanted.trim()),
            }
        }
        bits
    });
    let mut vm = apply_vm_diagnostics(vm.with_trace_frames(options.handlers)
        .with_fn_names(options.fn_names).with_static_names(options.static_names), options.trace);
    if let Ok(spec) = std::env::var("BREAK_AT") { vm = vm.with_debug_sink(break_at_sink(spec)); }
    if let Some(bits) = trace_filter { vm = vm.with_trace_filter(bits); }
    install_std_devices(&mut vm);
    let result = if options.cart {
        vm.run_to_yield(module).and_then(|_| {
            while vm.resume(module, Value::UNIT)? {}
            Ok(())
        })
    } else {
        vm.run_module(module).map(|value| { if options.print_value { print_result(&vm, value); } })
    };
    eprint!("{}", vm.profile_report());
    match result {
        Ok(()) => {
            if options.trace && options.exit_on_halt { eprintln!("[heap] live={}", vm.heap_live_count()); }
            if options.leak { eprint!("{}", vm.live_slots_report()); }
            if options.exit_on_halt {
                if let Some(code) = vm.exit_code() { return ExitCode::from(code as u8); }
            }
            ExitCode::SUCCESS
        }
        Err(error) => { eprintln!("runtime error: {}", error); ExitCode::from(2) }
    }
}

fn print_result(vm: &VirtualMachine, value: Value) {
    if vm.last_result_is_handle() && !value.is_handle_none() {
        if let Some(text) = read_string(vm.heap_ref(), value) { println!("{}", text); return; }
    }
    println!("{}", value.as_int());
}

fn apply_vm_diagnostics(mut vm: VirtualMachine, trace: bool) -> VirtualMachine {
    if trace {
        vm = vm.with_debug_sink(myriad_stderr_sink());
    }
    match std::env::var("TRACE_SLOT") {
        Ok(s) if s == "*" => vm = vm.with_heap_trace(None, true, eprintln_sink),
        Ok(s) => if let Ok(n) = s.parse::<u32>() {
            vm = vm.with_heap_trace(Some(n), false, eprintln_sink);
        },
        Err(_) => {}
    }
    if let Ok(s) = std::env::var("TRACE_STATIC") {
        if !s.is_empty() { vm = vm.with_trace_static(Some(s)); }
    }
    if std::env::var("PROFILE").is_ok() { vm = vm.with_profile(true); }
    if std::env::var("ABRASE_HEAP_CHECK").is_ok() { vm = vm.with_heap_check(true); }
    vm.with_trace_out(eprintln_sink)
}

fn break_at_sink(spec: String) -> vm::DebugSink {
    let (fn_part, pos_part) = match spec.rsplit_once(':') {
        Some(p) => p,
        None => {
            eprintln!("BREAK_AT: expected <fn>:<pc> or <file>:<line>, got '{}'", spec);
            return Box::new(|_, _| {});
        }
    };
    let want_pos: usize = pos_part.parse().unwrap_or(usize::MAX);
    let file_mode = fn_part.contains('.');
    let fn_part = fn_part.to_string();
    Box::new(move |event, names| {
        if let vm::DebugEvent::Trace { func, pc, op, base_reg, window, handle_mask, line, file } = event {
            let matched = if file_mode {
                *line as usize == want_pos && *file == fn_part
            } else {
                *pc == want_pos
                    && fn_part.strip_prefix('#')
                        .map(|id| id.parse::<usize>() == Ok(*func))
                        .unwrap_or_else(|| names.get(*func).map_or(false, |n| n == &fn_part))
            };
            if !matched { return; }
            eprintln!("[break {}:{}] {:?} (base r{})",
                vm::render_fn_label(*func, names), pc, op, base_reg);
            for (i, raw) in window.iter().enumerate() {
                let is_h = is_handle_mask(handle_mask, i);
                if *raw == vm::HANDLE_NONE && !is_h { continue; }
                eprintln!("    r{:<3} = {:#0width$x}{}", i, raw, if is_h { "  (handle)" } else { "" }, width = vm::WORD_BYTES * 2 + 2);
            }
        }
    })
}
