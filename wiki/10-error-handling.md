# 10. Error Handling

## Two-Layer Model

| Layer | Mechanism | Use case |
|-------|-----------|----------|
| Recoverable | `Result<T, E>` + `<exn<E>>` effect | Business errors, IO failures |
| Unrecoverable | `panic` | Invariant violations, bugs |

## Result + exn

`<exn<E>>` is an effect; `Result<T, E>` is a value. They can be converted.

```rust
// Exception version
fn parse(s: String) -> <exn<ParseError>> Int {
  if !is_valid(s) { throw ParseError.Invalid }
}

// Result version
fn try_parse(s: String) -> Result<Int, ParseError> {
  handle parse(s) {
    return v => Ok(v),
    exn e => Err(e),
  }
}
```

## throw and catch

```rust
fn divide(x: Int, y: Int) -> <exn<MathError>> Int {
  if y == 0 { throw MathError.DivByZero }
  x / y
}

fn safe_divide(x: Int, y: Int) -> Int {
  handle divide(x, y) {
    return v => v,
    exn _ => 0,
  }
}
```

## ? Operator

Only for propagating `exn<E>` effect:

```
fn pipeline(s: String) -> <exn<ParseError>, exn<ValidateError>> Output {
  let parsed = parse(s)?;
  let valid = validate(parsed)?;
  process(valid)
}
```

## panic

```
panic("invariant violated: list should not be empty")
```

`panic` has type `Never`, usable anywhere. Panic cannot be caught by handler (unless `<panic>` effect explicitly enabled).
