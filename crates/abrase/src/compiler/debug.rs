pub type CompileDebugSink = Box<dyn FnMut(&str)>;

pub fn stderr_sink() -> CompileDebugSink {
    Box::new(|msg| eprintln!("{}", msg))
}
