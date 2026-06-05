use polka::{OpCode, Value};

pub enum DebugEvent<'a> {
    Trace {
        func: usize,
        pc: usize,
        op: &'a OpCode,
        base_reg: usize,
        // The current fn's register window and which of those regs hold handles.
        window: &'a [u64],
        handle_mask: u128,
        // Source line of this op (0 = no debug info).
        line: u32,
        // Source file of this fn ("" = unknown).
        file: &'a str,
    },
    HandlePush {
        effect_id: u16,
        cell_slot: u32,
        cell_gen: u32,
        suspend_pc: usize,
        suspend_base: usize,
        dest: usize,
        depth: usize,
    },
    Resume {
        saved_pc: usize,
        saved_base: usize,
        cell_dest: usize,
        val: Value,
        handler_dest: usize,
        alive: Value,
        depth: usize,
    },
}

pub type DebugSink = Box<dyn FnMut(&DebugEvent, &[String])>;

/// Symbolic label for a function id: "name#id" if a non-empty name is known,
/// "#id" otherwise. Used by trace, halt errors, anywhere a fn is rendered.
pub fn render_fn_label(idx: usize, names: &[String]) -> String {
    match names.get(idx) {
        Some(n) if !n.is_empty() => format!("{}#{}", n, idx),
        _ => format!("#{}", idx),
    }
}

pub fn stderr_sink() -> DebugSink {
    Box::new(|event, names| {
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
