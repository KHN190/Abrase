#![no_std]

#[macro_use]
extern crate alloc;

pub(crate) mod math32;
pub(crate) mod formatter32;
pub mod runtime;
pub use runtime::{Runtime, RuntimeValue, RuntimeProfile};

pub mod word32;
#[cfg(not(all(target_os = "none", target_pointer_width = "32")))]
pub mod word64;

#[cfg(all(target_os = "none", target_pointer_width = "32"))]
pub use word32::*;
#[cfg(not(all(target_os = "none", target_pointer_width = "32")))]
pub use word64::*;
