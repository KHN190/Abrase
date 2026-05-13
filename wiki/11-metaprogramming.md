# 11. Metaprogramming

## Built-in Derive

Only compiler-built-in derive; users cannot extend.

```
@derive(Eq, Ord, Show, Clone, Hash)
type User = {
  name: String,
  age: Int,
}
```

Built-in derives:
- `Eq` - structural equality
- `Ord` - total order
- `Show` - to string
- `Hash` - hashing
- `Clone` - deep copy (only when all fields Clone)
- `Default` - default value

Rust may register additional derives (e.g., `Serialize`, `Deserialize`), but these are Rust-provided, not language built-in.

## const fn

```
const fn fib(n: Int) -> Int {
  if n < 2 { n } else { fib(n-1) + fib(n-2) }
}

const FIB_10: Int = fib(10);   // compile-time evaluation
```

const fn limitations:
- Effect set must be subset of `<total>`
- Cannot call non-const fn
- Cannot perform heap allocation (`<alloc>` not allowed)

## Prohibited Features

- User-defined macros (declarative or procedural)
- Runtime reflection
- `eval` / dynamic code loading
- Dynamic dispatch disabled by default

## Restricted Dynamic Dispatch

For dynamic dispatch, use `dyn Trait`:

```
fn print_all(items: List<dyn Show>) -> <console> Unit {
  for item in items {
    println(item.show());
  }
}
```

`dyn Trait` is a fat pointer with runtime overhead. Explicit to draw LLM attention.
