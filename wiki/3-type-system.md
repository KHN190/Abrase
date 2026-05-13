# 2. Type System

## Primitive Types

```
Int      // 64 bit
UInt     // 64 bit
Float    // 64 bit
Bool
Char     // Unicode scalar value
String   // UTF-8, immutable
Unit     // type of ()
Never    // divergent, subtype of any type
```

## Compound Types

**Tuple**
```
(Int, String, Bool)
()
(Int,) // single element tuple needs ','
```

**Array** (stack-allocated, @copy when elements are @copy)
```
[Int; 16]
[Float; 3]
```

**List** (dynamic length, heap-allocated, @move)
```
List<Int>
List<List<String>>
```

**Record**
```
type User = {
  name: String,
  age: Int,
}
```

**Variant** (tagged union)
```
type Shape =
  | Circle { radius: Float }
  | Rect { width: Float, height: Float }
  | Point
```

**Function Type**
```
(Int, String) -> Bool                    // pure function
(Int) -> <io> Unit                       // with effect
() -> <async, exn<E>> T                  // multiple effects
```

**Reference Type**
```
&T                  // immutable reference, inferred region
&mut T              // exclusive mutable reference
&T in r             // explicit region annotation
```

## Generics

```
fn map<T, U>(xs: List<T>, f: (T) -> U) -> List<U>

type Pair<A, B> = { first: A, second: B }
```

**Constraints** use `where` clause:
```
fn sort<T>(xs: List<T>) -> List<T>
  where T: Ord
```

## Type Inference

Bidirectional + local Hindley-Milner.

- Function signatures must be fully annotated
- Local variables in function body can omit types
- Type inference does not cross function boundaries

## Subtyping

No nominal subtyping. `Never` is the only exception: it's a subtype of any type.

Structural equality for tuples and function types; nominal equality for records and variants.

## Type Aliases

```
type alias UserId = Int
type alias Callback = (Int) -> <io> Unit
```
