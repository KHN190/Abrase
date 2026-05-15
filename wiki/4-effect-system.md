# 4. Effect System

As an extension of type classic type system. Make side effects explicit in signatures. Compiler tracks and checks effects.

Built-in effects:
* `<total>` (pure, default), 
* `<div>` (diverges), 
* `<exn<E>>` (throws),
* `<state<S>>`, 
* `<async>`, 
* `<alloc>`, 
* `<nondet>`. 

Users can define custom effects. Handlers eliminate effects, like exceptions.

```rust
fn add(x: Int, y: Int) -> Int { x + y }  // <total> (pure)
fn divide(x: Int, y: Int) -> <exn<E>> Int { ... }

fn map<T, U, e>(xs: List<T>, f: (T) -> <e> U) -> <e> List<U>  // effect polymorphic

let result = handle compute() {
  return v => Ok(v),
  exn e => Err(e),
};

effect logger { fn log(msg: String) -> Unit }
handle work() { return v => v, logger.log msg => { ... resume(()); } }
```

Effects are sets (order irrelevant). 
