pub mod system;
pub mod console;
pub mod hostfunc;
pub mod clock;
pub mod random;

pub use system::{SystemDevice, SYSTEM_ID};
pub use console::{Console, BufferConsole, StdoutConsole, CONSOLE_ID};
pub use hostfunc::{HostFuncDevice, HostImpl, HOSTFUNC_ID};
pub use clock::{ClockDevice, CLOCK_ID};
pub use random::{RandomDevice, RANDOM_ID};
