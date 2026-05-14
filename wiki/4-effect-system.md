# 4. Effect System

Effect system controls side effects at the type level. Functions declare what effects they perform (e.g., throwing exceptions, async operations) in their signatures. This makes side effects explicit and checkable by the compiler.

Thus we can achieve a few merits:

1. Explicit over implicit, code signature shows what it does
2. Control flow like exception in a handler and keep other functions pure.
3. Side effects are constrained in a function's lifecycle.
4. Compiler checks if all side effects are all handled.

## Internal Effects

| Effect | Meaning |
|--------|---------|
| `<total>` | Pure function, always terminates (default, can be omitted) |
| `<div>` | May not terminate |
| `<exn<E>>` | May throw exception of type E |
| `<state<S>>` | Can read/write state of type S |
| `<async>` | Async evaluation |
| `<alloc>` | Heap allocation |
| `<nondet>` | Non-deterministic (random numbers, etc.) |

Note: No built-in `<io>`, `<console>`, `<fs>`, `<net>` - these are Rust-provided effects.

## Effect Syntax

```
fn add(x: Int, y: Int) -> Int { x + y }                    // <total> omitted
fn divide(x: Int, y: Int) -> <exn<DivByZero>> Int {
  if y == 0 { throw DivByZero } else { x / y }
}
```

Effects are sets; order is irrelevant: `<a, b>` and `<b, a>` are equivalent.

## Effect Polymorphism

Functions can be polymorphic over effects:

```
fn map<T, U, e>(xs: List<T>, f: (T) -> <e> U) -> <e> List<U>
```

## Effect Aliases

```
effect alias app = <async, fs, net, console>
effect alias graphics = <gpu.device, gpu.queue, gpu.memory>
```

## Handlers

Handlers eliminate effects via `handle` expressions:

```
let result = handle compute() {
  return v => Ok(v),
  exn e => Err(e),
};
```

**Handler clauses:**
- `return v => expr` - executed on normal return
- `exn e => expr` - catch exception
- `<effect_name> args => expr` - custom effect handling

**Custom effects:**

```
effect logger {
  fn log(msg: String) -> Unit
}

fn main() -> <console> Unit {
  handle do_work() {
    return v => v,
    logger.log msg => {
      println("[log] {msg}");
      resume(());
    },
  };
}
```

`resume(value)` resumes interrupted computation in handler.

## Capability vs Effect

| Side-effect form | Use |
|------------------|-----|
| Expressible by "existence of an object" | Capability (passed as value) |
| Intangible environmental influence | Effect (as type annotation) |

Example:
```
// device is capability, async and exn are effects
fn render(scene: &Scene, device: &mut GpuDevice) -> <async, exn<GpuError>> Frame
```
