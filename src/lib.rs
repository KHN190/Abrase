pub mod ast;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod ty;
pub mod typeck;
pub mod bytecode;
pub mod myriad;
pub mod compiler;

// Back-compat re-exports during the vm → myriad rename. Remove once
// downstream embedders have migrated to `abrase::myriad::*`.
pub use myriad as vm;
pub use myriad::host;