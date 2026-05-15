# 3. Type System

Static typing with local, bidirectional inference. Types are fully explicit in signatures.

Ect provides:

* Primitives (Int, Float, Bool, Char, String, Unit, Never), 
* Compound types (tuple, array, list, record, variant), 
* Functions, 
* References, 
* Generics,
* Effect.

```rust
// Primitives
Int, Float, Bool, Char, String, Unit

// Tuples
(Int, String, Bool), (Int,)

// Array and List
[Int; 16], List<Int>

// Record (named fields)
type User = { name: String, age: Int }

// Variant
type Shape = | Circle { radius: Float } | Point

// Function type
(Int, String) -> <exn<E>> Bool

// Generic function
fn sort<T>(xs: List<T>) -> List<T> where T: Ord { ... }

// Effect
effect Logger { fn log(msg: String) -> Unit }
```

Notes:

* Unit (`()`) is the empty type—return value of functions that don't return data (just side effects). 

* Never is the divergent type for functions that never return (e.g., `panic`, infinite loop). Never is a subtype of any type.

* Type / Effect must be declared before use. No forward references.

References must be annotated with regions: `&T` or `&T in r`. 
