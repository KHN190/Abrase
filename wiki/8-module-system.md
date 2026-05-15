# 8. Module System

Explicit module imports prevent accidental shadowing (where a later-defined name hides an earlier one with the same name). Names come from users or Rust; Ect reserves no namespaces. Naming conflicts between user and Rust-provided cause compile error instead.

Modules correspond to file paths. Two visibility levels: `pub` (public) or private.

```rust
// path: claws/teeth.ect
mod claws.teeth

pub fn bite(x: Int) -> Int { x * 2 }
fn internal(x: Int) -> Int { x + 1 }

import io.{read, write}
import claws.teeth.{bite as b}
import claws.teeth
// then: claws.teeth.bite(42)
```

Naming conflicts between user and Rust-provided cause compile error. Module graph checked for cycles.
