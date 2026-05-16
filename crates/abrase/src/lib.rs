pub mod ast;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod ty;
pub mod typeck;
pub mod myriad;
pub mod compiler;

// Re-export polka as `bytecode` so existing call sites (`abrase::bytecode::*`)
// and intra-crate `crate::bytecode::*` paths continue to work. Polka itself
// is now a sibling crate with no abrase or myriad dependency.
pub use polka as bytecode;

// Back-compat re-exports during the vm → myriad rename. Remove once
// downstream embedders have migrated to `abrase::myriad::*`.
pub use myriad as vm;
pub use myriad::host;
