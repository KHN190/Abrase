# Myriad

> Published as `myriad-rs` (the bare `myriad` was taken). The Rust import
> path stays `myriad` — add to your `Cargo.toml` as
> `myriad = { package = "myriad-rs", version = "0.1.0-alpha.1" }` and keep
> `use myriad::*;` unchanged.

Myriad is the runtime for [Polka](https://crates.io/crates/polka-rs) bytecode. 

This crate defines its computation core. The register machine interpreter features:

- Effect handlers
- Regioned memory
- Pluggable devices API

## Embedding

```rust
use myriad::{Host, VirtualMachine};

let module = /* polka::Module from your compiler */;
let mut vm = VirtualMachine::new();
Host::default().install_into(&mut vm);
let result = vm.run_module(&module)?;
```

See the [main repo](https://github.com/KHN190/Abrase) for the device catalog
and examples.

## Status

Schema stable; device catalog is still in flux (Screen / Audio / Controller
to land before 1.0). Expect breaking changes to `Device` impls pre-1.0.

## License

MIT
