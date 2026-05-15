# 10. Error Handling

Recoverable errors (Result/exn) for expected failures. Panics for invariant violations. Effects make error flow explicit.

A few options:

* `Result<T, E>` (value) and `<exn<E>>` effect are interconvertible.
* `throw` raises, handlers catch by `handle`. 
* `?` operator on exn calls: if error, propagate up; if ok, unwrap and continue. 
* `panic` for unrecoverable bugs.

```rust
// Throws exn<E>
fn parse(s: String) -> <exn<ParseError>> Int {
  if !is_valid(s) { throw ParseError.Invalid }  // raises error
  to_int(s)
}

// Convert exn to Result using handle
fn try_parse(s: String) -> Result<Int, ParseError> {
  handle parse(s) {
    return v => Ok(v),      // on success, wrap in Ok
    exn e => Err(e),        // on exn, wrap in Err
  }
}

// Catch exn and return default value
fn safe_divide(x: Int, y: Int) -> Int {
  handle divide(x, y) {
    return v => v,          // success: return value
    exn _ => 0,             // error: return 0
  }
}

// Propagate exn up the call stack
let val = parse(s)?;  // if parse throws, exit early; else extract value

// Panic on invariant violation (Never type)
panic("invariant: list non-empty")
```

Exn checked at compile time. Panic uncatchable by default.
