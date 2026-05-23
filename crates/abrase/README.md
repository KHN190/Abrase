# Abrase (compiler)

The Abrase language compiler. Produces [Polka](https://crates.io/crates/polka-rs) bytecode that runs on [Myriad](https://crates.io/crates/myriad-rs).

Abrase (`.abe`) is a Rust-inspired language built around three ideas: static
type checking, an algebraic effect system, and region-based memory
management.

## Embedding

```rust
use abrase::compiler::Compiler;
use abrase::lexer::Lexer;
use abrase::parser::Parser;

let src = std::fs::read_to_string("hello.abe")?;
let ast = Parser::new(Lexer::new(&src)).parse_program();
let module = Compiler::new().compile_module(&ast)?;
// hand `module` to myriad::VirtualMachine::run_module
```

For the CLI, install [`abrase-cli`](https://crates.io/crates/abrase-cli)
instead. See the [main repo](https://github.com/KHN190/Abrase) for language
docs and examples.

## License

MIT
