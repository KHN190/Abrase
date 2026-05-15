# 9. Concurrency Model

Concurrency is provided by effect system.

## Example

```rust
effect pause { fn wait(ms: Int) -> Unit }

fn step() -> <pause> Int {
  pause.wait(10);                    // suspension point — handler decides
  41 + 1
}

let v = handle step() {
  return v       => v,               // finish
  pause.wait ms  => {                // `resume` is bound by `handle` & region
    sleep_ms(ms);
    resume(())                       //  continuation of `step` (see §4)
  },
};
```

Any function whose effect set is handled by the scheduler is a coroutine.

## Region

`region` opens a lexical lifetime. Bindings created inside live in that region; drops run in reverse order at region exit. A reference `&T in r` is invalid the instant `r` ends.

```rust
region r {
  let data = load();
  let view = &data;                  // &T in r
  process(view);
}                                    // view invalidated, data dropped
```

## Sharing

* `Shared<T>` — immutable refcounted shared data.
* `Channel<T>` — typed message passing.
