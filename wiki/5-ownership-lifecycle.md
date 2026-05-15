# 5. Ownership & Lifecycle

## Ownership Categories

| Category | Annotation | Semantics |
|----------|------------|-----------|
| Copy | `@copy` | Free copy |
| Move | `@move` | Only ownership (default) |
| Share | `@share` | Shared ref count |

## Inference Rules

- Primitive types (Int, Float, Bool, Char) → `@copy`
- String, List<T>, Map<K,V> → `@move`
- Tuples, records: all fields @copy → @copy, otherwise @move
- Arrays: elements @copy → @copy, otherwise @move
- Variants: all fields of all variants @copy → @copy, otherwise @move
- Shared<T> → @share
- Function types: default @copy (but closures may be @move when capturing @move values)

## Explicit Override

```
@copy type Color = { r: UInt8, g: UInt8, b: UInt8 }
```

## Move Semantics

```rust
let s = "hello";         // s: String, @move
let t = s;               // ownership transferred to t
// println(s);           // ERROR: s was moved
```

Move occurs on:
- Assignment to new variable
- Passing as function argument
- Returning as value
- Storing in data structure

## Copy Semantics

```rust
let x = 42;              // x: Int, @copy
let y = x;               // copy, x still usable
```

@copy uses bitwise copy.

## Shared

`Shared<T>` for read-only sharing across tasks or scopes:

```rust
let cfg = Shared.new(load_config());   // Shared<Config>
let cfg2 = cfg.clone();                // ref count +1
```

- Reference-counted sharing (@share semantics)
- T is always immutable (language-level guarantee)
- clone() increments count, original not moved
- Cannot get &mut T

## References & Regions

References must be used within `region` blocks. Regions are lexical scopes.

```rust
region r {
  let view = &cfg;                   // &Config in r
  process(view);
}
```

**Borrowing rules:**
- At any time: many &T OR unique &mut T OR no references and can be moved
- References cannot escape region blocks
- References cannot be stored in data structures
- References cannot be held across await

## Drop

Values are dropped immediately after last use in ownership chain.

```rust
trait Drop {
  fn drop(self) -> <io> Unit
}
```

- No exceptions allowed in drop
- Drop order: local variables in reverse declaration order; fields in declaration order


## Examples

```rust
// Wrong
fn main() -> String {
  let s = "hello";
  let t = s; // s is moved
  let u = s; // Error: use after move
  u
}

// Option 1: Use references (borrow) - preferred
fn main() -> String {
  let s = "hello";
  let t = &s;     // borrow s (don't move)
  let u = &s;     // borrow s again
  s               // move s at the end when returning
}

// Option 2: Clone the value
fn main() -> String {
  let s = "hello";
  let t = s.clone();  // clone s
  let u = s;          // now s is moved to u
  u
}

// Option 3: Only move at the very end
fn main() -> String {
  let s = "hello";
  // use s multiple times without assigning elsewhere
  // then move it at the end
  s
}
```
