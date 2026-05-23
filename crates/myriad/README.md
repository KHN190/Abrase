# Myriad

> Published as `myriad-rs` (`myriad` name was taken). The Rust import
> path stays `myriad` — add to your `Cargo.toml` as
> `myriad = { package = "myriad-rs", version = "0.1.0" }` and keep
> `use myriad::*;` unchanged.

Myriad is the runtime for [Polka](https://crates.io/crates/polka-rs) bytecode. 

This crate defines its computation core.

## Embedding

```rust
use myriad::{Host, VirtualMachine};

let module = /* polka::Module from your compiler */;
let mut vm = VirtualMachine::new();
Host::default().install_into(&mut vm);
let result = vm.run_module(&module)?;
```

See the [main repo](https://github.com/KHN190/Abrase) for more examples.

## License

MIT
