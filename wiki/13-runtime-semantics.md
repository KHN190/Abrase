# 13. Runtime Semantics

This page explains execution.

## Evaluation

Eager, left-to-right. Sub-expressions complete before their enclosing operation runs.

```rust
a + b      // a, then b, then +
f(x, y)    // x, then y, then call f
(a, b)     // a, then b
arr[i]     // arr, then i, then index
```

Function arguments are evaluated before the call. Conditionals (`if`, `match`) evaluate the discriminant first, then exactly one arm.

## Drop

A binding is dropped at the end of its enclosing scope, in reverse order of creation. A binding that has been moved is **not** re-dropped at scope exit.

```rust
fn f() {
  let a = Buffer.new(1024);     // alloc a
  let b = Buffer.new(1024);     // alloc b
  consume(b);                   // b moved out; not dropped here
}                               // drop a
```

`drop(x)` runs the destructor immediately. `panic` and uncaught `throw` unwind the current scope and run drops on the way out.

## Regions and References

`region r { ... }` opens a lexical lifetime. References created inside cannot outlive the block. At region exit, the host invalidates any outstanding `&T in r` handles.

```rust
region r {
  let view = &data;
  process(view);
} // view invalidated; data accessible again
```

References cannot cross an `await`. The compiler enforces this; if it did not, the runtime would observe a dangling pointer.

## Scopes and Concurrency

`scope s { ... }` opens a coroutine group. Tasks spawned via `s.spawn(...)` run cooperatively. `scope` exits only after every spawned task has joined.

If any task panics or throws an uncaught exception, the scope cancels the others, runs their drops, and re-raises the error from `scope`.

Only `.await` and `.yield` are suspension points. Between two suspension points, a task runs to completion without preemption.

## Capabilities

Globals and modules registered by the host (see §12) are available from `main` and propagate by ordinary argument passing. There is no ambient lookup at arbitrary call sites — a function can only call host code it explicitly received or imported.

A capability becomes unreachable when its last reference is dropped; the host is notified through the corresponding device's close protocol (see device catalog).

## Effects

`<exn<E>>` returning functions surface failures as a value the caller must match against. An unhandled `throw` propagates outward through `scope` and `region` exits, running drops along the way, until it meets a `try` block or terminates the program.

User-defined effects use `handle` to install a handler frame. Effect operations dispatch to the most recently installed handler for that effect. Handlers may `resume` to continue the suspended computation or return a value to short-circuit.

## Faults

The VM produces an unrecoverable trap on:

- integer division or modulus by zero
- pointer dereference of an invalid handle
- register frame overflow (recursion or local count exceeds the per-frame budget)

Traps abort the current program; drops do not run. Recoverable conditions (file-not-found, network error, etc.) are surfaced as effect values, not traps.

## Determinism

Given the same inputs, devices, and scheduler decisions, execution is deterministic. Non-determinism enters only through host device behavior (clock, random, network).
