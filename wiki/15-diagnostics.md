# 15. Diagnostics (Not Implemented)

Diagnostics are structured messages emitted by the compiler pipeline (lexer, parser, typeck, codegen).

## Severity

| Level   | Meaning                                          | Blocks compilation |
|---------|--------------------------------------------------|--------------------|
| `error` | Invalid program. Must be fixed.                  | Yes                |
| `warn`  | Compiles but likely wrong. Should be fixed.      | No                 |
| `info`  | Informational. Inferred types, effect changes.   | No                 |
| `hint`  | UX suggestion. "did you mean X", "consider `&`"  | No                 |

## Diagnostic structure

```
Diagnostic {
    code:     DiagCode,       // e.g. E101
    severity: Severity,       // Error | Warn | Info | Hint
    span:     Span,           // primary source location
    message:  String,         // human-readable summary
    labels:   Vec<Label>,     // secondary spans with context messages
    notes:    Vec<String>,    // "help: ...", "consider: ..."
}

Label {
    span:    Span,
    message: String,
}
```

## Error codes

### E1xx — Type errors

| Code | Message summary                                   |
|------|---------------------------------------------------|
| E101 | Type mismatch: expected `X`, found `Y`            |
| E102 | Cannot convert `X` to `Y`                         |
| E103 | Unknown type `X`                                  |
| E104 | Generic argument count mismatch                   |
| E105 | Recursive type without indirection                |
| E106 | Type alias cycle detected                         |
| E107 | Variance conflict: covariant position expects `X` |

### E2xx — Ownership & lifetime errors

| Code | Message summary                                          |
|------|----------------------------------------------------------|
| E201 | Use of moved value `x`                                   |
| E202 | Cannot move out of borrowed content                      |
| E203 | Cannot mutably borrow `x`: already borrowed immutably    |
| E204 | Cannot borrow `x` as immutable: already borrowed mutably |
| E205 | Cannot mutably borrow immutable variable `x`             |
| E206 | Reference escapes its region                             |
| E207 | Value does not live long enough                          |

### E3xx — Effect errors

| Code | Message summary                                              |
|------|--------------------------------------------------------------|
| E301 | Effect `X` required but not declared                         |
| E302 | Effect `X` declared but not used                             |
| E303 | Impure expression in `const` context                         |
| E304 | Mutable state in `const` context                             |
| E305 | Exception type mismatch in `exn<X>` effect                   |
| E306 | Unhandled effect `X` at function boundary                    |
| E307 | `handle` arm for effect `X` but `X` is not active            |

### E4xx — Declaration errors

| Code | Message summary                                   |
|------|---------------------------------------------------|
| E401 | Duplicate definition of `X`                       |
| E402 | Missing function body                             |
| E403 | `impl` method `X` not in trait definition         |
| E404 | Trait `X` not implemented for type `Y`            |
| E405 | `const fn` body contains non-const expression     |
| E406 | Conflicting ownership attributes on type `X`      |

### E5xx — Pattern errors

| Code | Message summary                               |
|------|-----------------------------------------------|
| E501 | Non-exhaustive patterns: `X` not covered      |
| E502 | Unreachable pattern                           |
| E503 | Pattern type mismatch: expected `X`, found `Y`|
| E504 | Tuple pattern length mismatch                 |
| E505 | Range pattern requires `Int` or `Char`        |
| E506 | Array pattern on non-array type               |
| E507 | Reference pattern on non-reference type       |

### E6xx — Import & privacy errors

| Code | Message summary                                      |
|------|------------------------------------------------------|
| E601 | Unresolved name `X`                                  |
| E602 | `X` is private                                       |
| E603 | Import collision: `X` already defined in this scope  |
| E604 | Intermediate module `X` in path is private           |
| E605 | Duplicate import of `X`                              |

### E7xx — Record & trait errors

| Code | Message summary                                  |
|------|--------------------------------------------------|
| E701 | Missing field `X` in record literal              |
| E702 | Unknown field `X` on type `Y`                    |
| E703 | Field `X` is private                             |
| E704 | Record completeness: fields `X, Y` not covered   |
| E705 | Unsatisfied trait bound `T: X`                   |
| E706 | Conflicting trait impls for type `X`             |

---

### W1xx — Unused

| Code | Message summary                        |
|------|----------------------------------------|
| W101 | Unused variable `x`                    |
| W102 | Unused import `X`                      |
| W103 | Unused effect annotation `<X>`         |
| W104 | Unused function `x`                    |
| W105 | Unused type `X`                        |

### W2xx — Redundant

| Code | Message summary                                      |
|------|------------------------------------------------------|
| W201 | Redundant type annotation: already inferred as `X`   |
| W202 | Redundant ownership attribute: default is `X`        |
| W203 | Dead code after unconditional `return`               |
| W204 | Effect `<X>` is already covered by alias `Y`         |

---

### I1xx — Inferred types (info)

| Code | Message summary                          |
|------|------------------------------------------|
| I101 | Type inferred as `X`                     |
| I102 | Return type inferred as `X`              |
| I103 | Generic parameter `T` resolved to `X`   |

### I2xx — Effect changes (info)

| Code | Message summary                                  |
|------|--------------------------------------------------|
| I201 | Effect set widened from `<X>` to `<X, Y>`        |
| I202 | Effect set narrowed: `X` handled by `handle`     |
| I203 | Closure effects inferred as `<X>`                |

---

### H1xx — Hints

| Code | Message summary                              |
|------|----------------------------------------------|
| H101 | Did you mean `X`?                            |
| H102 | Consider borrowing with `&` instead of moving |
| H103 | Consider adding effect `<X>` to declaration  |
| H104 | Consider using `_` to ignore this value      |
