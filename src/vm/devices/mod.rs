pub mod system;
pub mod console;

pub use system::{SystemDevice, SYSTEM_ID};
pub use console::{Console, BufferConsole, StdoutConsole, CONSOLE_ID};
