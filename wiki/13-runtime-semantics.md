# 13. Runtime Semantics

## Evaluation Strategy

**Strict evaluation** (eager). Function arguments fully evaluated before call.

## Evaluation Order

**Left to right:**
- Binary operators: left then right
- Function calls: evaluate callee, then left-to-right arguments
- Tuple/record literals: source code order

## Memory Model

- `@copy` values: stack-allocated, bitwise copy
- `@move` values: usually heap-allocated, move transfers ownership (no memory copy)
- `@share` values: heap-allocated + reference counting

## In-place Optimization

Compiler performs in-place update optimization on affine values:

```
let xs = [1, 2, 3];
let xs = xs.push(4);    // compiler can modify in-place since old xs unused
```

## Error Handling Implementation

`<exn<E>>` effect compiles to sum return type (`Result<T, E>`). `throw` compiles to early return. `handle` compiles to match. No runtime stack unwinding mechanism.

## Async Implementation

`<async>` effect compiles to state machine, interacting with Rust-provided executor. Recommend Rust use Tokio or similar runtime.
