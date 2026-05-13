# 12. Rust Integration

## Design Principles

Ect language has no concept of standard lib. Rust fully controls namespace and can:
1. Register global functions/types - Ect uses without import
2. Register modules - Ect imports normally

Module names are freely chosen by Rust (`io`, `ui`, `fs`, `tauri`, `gpu`, etc.). Ect reserves no prefixes.

## Namespace Layers

```
┌─────────────────────────────────────┐
│ Ect language built-in (always available) │
│   panic, assert, type conversion,    │
│   List/Map/Set built-in methods      │
├─────────────────────────────────────┤
│ Rust global (no import needed)       │
│   Rust-selectively exposed functions/types │
├─────────────────────────────────────┤
│ Modules (require import)             │
│   Rust-provided modules (io, ui)     │
│   User-defined modules (my.utils)    │
└─────────────────────────────────────┘
```

## Rust-side Binding (Rust sketch)

```rust
let mut runtime = ect::Runtime::new();

// Register as global function (no import needed in Ect)
runtime.register_global_fn("read_file", read_file_impl);

// Register as module (import needed in Ect)
runtime.register_mod("io", io_mod());

// Inject capability
runtime.inject("window", main_window);
runtime.eval(script)?;
```

Rust registration macro (sketch):

```rust
#[ect_mod("io")]
mod io {
    #[ect_fn(effects = "async, exn<IoError>")]
    async fn read_file(path: String) -> EctResult<String, IoError> {
        tokio::fs::read_to_string(&path).await
            .map_err(IoError::from)
            .into()
    }
}
```

## Ect-side Usage

```
// Rust registered as global: no import needed
let content = read_file("config.toml").await?;

// Rust registered as module: import needed
import io.{File, open_file};

fn load() -> <async, exn<io.IoError>> File {
  open_file("data.txt").await
}
```

## Naming Conflicts

User-defined cannot shadow Rust globals:

```
fn read_file(path: String) -> String { ... }
// ERROR: 'read_file' is already a global function provided by host
```

Same for module name conflicts.

## Opaque Types

Rust can expose opaque types: Ect holds reference, cannot inspect internals.

```rust
#[ect_type(opaque)]
struct GpuDevice { /* ... */ }
```

Ect side:
```
import gpu.{GpuDevice, create_device};

fn use_gpu(d: &mut GpuDevice) -> <async> Unit {
  // Can only operate on GpuDevice via gpu module functions
}
```

## Capability Injection

Rust constructs capability instances at VM startup and injects them. Scripts cannot create capabilities from nothing; must receive from entry point.

## @export Annotation

Rust calling script functions requires `@export`:

```
@export
fn handle_request(req: Request) -> <async, exn> Response {
}
```

Rust calls by name; parameters and return values pass through serialization layer.

## Type Schema Export

Rust toolchain should support exporting registered globals, modules, and types as `.ect.d` declaration files for LSP and LLM tools.
