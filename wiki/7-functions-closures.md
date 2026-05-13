# 7. Functions & Closures

## Function Definition

```
fn name(param1: T1, param2: T2) -> <effects> ReturnType {
  body
}

// No return value
fn greet(name: String) -> <console> Unit {
  println("hello {name}");
}

// Generic + constraints
fn max<T>(a: T, b: T) -> T
  where T: Ord
{
  if a.compare(&b) == Ordering.Less { b } else { a }
}
```

## Closures

```
let add = |x: Int, y: Int| x + y;
let inc = |x| x + 1;                   // type inference
let with_effect = |s: String| -> <console> Unit {
  println(s);
};
```

Closure capture follows ownership rules: default borrow, explicit `move |...| ...` when needed.

## self Parameter

Method first parameter named `self`:

```
impl User {
  fn name(self: &Self) -> &String { &self.name }
  fn rename(self: &mut Self, new_name: String) -> Unit {
    self.name = new_name;
  }
  fn consume(self: Self) -> String { self.name }
}
```

Call syntax: `user.name()` or explicit `User.name(&user)`.

## Traits

```
trait Show {
  fn show(self: &Self) -> String
}

impl Show for Int {
  fn show(self) -> String { int_to_string(self) }
}
```

**Orphan rule strict:** `impl Trait for Type` must be defined in Type or Trait's module.

## Default Parameters & Variadic Functions

Not supported. Use builder pattern or multiple function versions instead.

## Function Overloading

Not supported. Functions with same name must have unique signature. Use generics or traits for polymorphism.
