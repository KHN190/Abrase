# 7. Functions & Closures

Functions are first-class. Signatures must declare effects and types explicitly for clarity. Traits enable polymorphism.

Abe provides:

* Functions with mandatory signature annotations. 
* Generics with `where` constraints. 
* Closures with implicit borrow (or explicit `move`). 
* Methods via `self: &Self`, `self: &mut Self`, `self: Self`.
* Traits for impls.

```rust
fn greet(name: String) -> <console> Unit { println("hello {name}"); }
fn max<T>(a: T, b: T) -> T where T: Ord { ... }

let add = |x: Int, y: Int| x + y;
let inc = |x| x + 1;
let moved = move |x| uses_x;

impl User {
  fn name(self: &Self) -> &String { &self.name }
  fn rename(self: &mut Self, n: String) -> Unit { self.name = n; }
  fn consume(self: Self) -> String { self.name }
}

trait Show { fn show(self: &Self) -> String }
impl Show for Int { fn show(self) -> String { ... } }

user.name() or User.name(&user)
```

No default parameters, variadics, or overloading; use generics/traits.
