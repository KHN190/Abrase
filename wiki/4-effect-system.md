# 4. Effect System

An extension of the classic type system. Side effects are made explicit in signatures; the compiler tracks and checks them.

Built-in effects:

* `<total>` — pure, default
* `<div>` — diverges
* `<exn<E>>` — throws
* `<state<S>>` — mutable state
* `<alloc>` — heap allocation
* `<nondet>` — non-determinism

Users can declare custom effects.

```rust
fn add(x: Int, y: Int) -> Int { x + y } // <total>
fn divide(x: Int, y: Int) -> <exn<E>> Int { ... }

fn map<T, U, e>(xs: List<T>, f: (T) -> <e> U) -> <e> List<U>  // effect polymorphic

let result = handle compute() {
  return v => Ok(v),
  exn e   => Err(e),
};

effect logger { fn log(msg: String) -> Unit }
handle work() {
  return v          => v,
  logger.log msg    => { ...; resume(()) },
}
```

Effects are sets (order irrelevant).

**Notices**:

These are enforced to prevent dangling references:

* `resume` can be called multiple times.
* Each handler arm creates a local region.
* A function cannot pause if it is borrowing data from outside the handler block.
* Data only enters the paused function through `resume`, and exits when the function pauses again.
* The handler can keep its own private variables to track things across multiple resumes.
* A function can finish without calling `resume`, which calls drop of its references at its region exit.

