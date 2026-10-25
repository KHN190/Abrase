# Myriad

> Published as `myriad-rs` (`myriad` name was taken). The Rust import
> path stays `myriad` — add to your `Cargo.toml` as
> `myriad = { package = "myriad-rs", version = "0.1.0" }` and keep
> `use myriad::*;` unchanged.

Myriad is the runtime for [Polka](https://crates.io/crates/polka-rs) bytecode. 

This crate defines its computation core.

## Embedding

`Runtime::new(&module)` selects a real `word32` runtime for `INT32_SAFE`
cartridges and `word64` otherwise. Register natives through its typed
`word32_mut()` / `word64_mut()` accessors; their `Value` and `Heap` types
cannot be mixed. The selected profile stays fixed until the runtime is dropped.
The CLI sets this cartridge flag with `--int32`; no width feature is needed.

The root `VirtualMachine` API below retains the typed 64-bit host API and
rejects `INT32_SAFE` modules. On 32-bit bare metal only `word32` is compiled.

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
