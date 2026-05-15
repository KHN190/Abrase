# 11. Metaprogramming

No user defined macros. Built-in derives only, it will expand in compile time. Const evaluation at compile time.

* 4 `@derive` types:

  * `Eq` (equality), 
  * `Ord` (ordering), 
  * `Show` (to string)
  * `Clone` (deep copy)

* `const fn` for compile-time evaluation.

Note: If you derive a trait, all fields in the type must also have that trait.

```rust
@derive(Eq, Ord, Show, Clone)
type Dog = { name: String, age: Int }

impl Show for Dog {
  fn show(self: &Self) -> String {
    "I bark {self.name} at age {self.age}."
  }
}

// Animal can derive Eq because Dog also derives Eq
@derive(Eq, Ord, Show, Clone)
type Animal = 
  | Dog { dog: Dog }
  | Cat { name: String }

const fn fib(n: Int) -> Int {
  if n < 2 { n } else { fib(n-1) + fib(n-2) }
}

const FIB_10: Int = fib(10); // compile-time
```

Const fn must be `<total>`.
