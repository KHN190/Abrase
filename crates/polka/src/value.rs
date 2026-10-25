#[cfg(all(target_os = "none", target_pointer_width = "32"))]
pub use crate::word32::{Value, HANDLE_NONE, HANDLE_SLOT_MAX, decode_float, encode_float};
#[cfg(not(all(target_os = "none", target_pointer_width = "32")))]
pub use crate::word64::{Value, HANDLE_NONE, HANDLE_SLOT_MAX, decode_float, encode_float};
