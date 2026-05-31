pub mod ast;
pub mod error;
pub mod lexer;
pub mod loader;
pub mod parser;
pub mod ty;
pub mod typeck;
pub mod compiler;

// Re-export polka as `bytecode` so existing call sites (`abrase::bytecode::*`)
// and intra-crate `crate::bytecode::*` paths continue to work. Polka itself
// is a sibling crate with no abrase or myriad dependency.
pub use polka as bytecode;
