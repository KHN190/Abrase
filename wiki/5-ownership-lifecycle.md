# 5. Ownership & Lifecycle

Explicit ownership prevents resource leaks and data races. Automatic inference from type structure and enforece memory safety.

We have 3 modes:

* `@copy` (bitwise copy), 
* `@move` (transfer ownership), 
* `@share` (ref-counted immutable). 

Primitives are `@copy`; collections default to `@move`. Borrows in `region` blocks only — `fn` bodies and handler arm bodies (§4) are implicitly regions.

```rust
let x = 42;         // Int, @copy
let y = x;          // copy, x still usable

let s = "hello";    // String, @move
let t = s;          // ownership moves; s unusable after
let u = &s;         // borrow s (must be in region)
s;                  // move at end when no longer borrowed

let cfg = Shared.new(config);  // @share ref-count
let cfg2 = cfg.clone();        // +1 refcount

region r {
  let view = &data;   // &T in region r
  process(view);
}                      // view dropped, data accessible again
```

Drop happens at last use. Reference rules: many `&T` XOR one `&mut T` XOR move. At an effect-op call site, no live borrow may originate from a region outside the enclosing handler arm — the borrow barrier (§4).
