# 8. Module System

## Module Declaration

```
// file: my/utils.ect
mod my.utils

pub fn helper(x: Int) -> Int { x * 2 }

fn internal(x: Int) -> Int { x + 1 }  // private by default
```

Module names should correspond to file paths (`my/utils.ect` → `my.utils`).

## Imports

```
import io.{read_file, write_file}
import ui.window
import my.utils.{helper as h}

import my.utils
// usage: my.utils.helper(42)
```

## Visibility

Two levels only:
- `pub` - public, visible across modules
- Otherwise - module-private

## Namespace

Ect reserves no special namespaces. All module names are freely chosen by user or Rust.

**Naming conflicts:** User-defined and Rust-injected global functions/modules with same name cause compile error. Explicit is better than implicit shadowing.

## Circular Dependencies

Module dependency graph must be a DAG. Checked at compile time.
