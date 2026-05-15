# 12. Rust Integration

Ect has no stdlib; a Rust host provides all globals, modules, capabilities.

Rust registers globals (no import) or modules (import required). Ect calls Rust via effects. Opaque types, capability injection, `@export` for callbacks.

```rust
// Rust-side
runtime.register_global("read_file", read_file_impl);
runtime.register_module("io", io_mod());

// Ect-side
let content = read_file("config.toml")?;

import io.{File, open_file};
fn load() -> <io, exn<io.IoError>> File {
  open_file("data.txt")
}

import gpu.{GpuDevice, create_device};
fn use_gpu(d: &mut GpuDevice) -> <gpu> Unit { ... }

@export
fn handle_request(req: Request) -> <io, exn> Response { ... }
```

User cannot shadow Rust globals. Capabilities must be provided by Rust host from startup only.

**Notes**:

In VM, `import`/`export` compile to `DEI`/`DEO` (device I/O) opcodes that read/write through device ports (device ID + port address). The Rust host must implement the [device catalog](./appendix-device-catalog.md) defined in appendix (console, filesystem, network, clock, etc.).
