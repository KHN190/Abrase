# 6. Expressions & Statements

## Everything is an Expression

`if`, `match`, `{...}` are all expressions with values.

```
let max = if a > b { a } else { b };

let label = match shape {
  Circle { radius } => "circle",
  Rect { .. } => "rect",
  Point => "point",
};

let computed = {
  let a = expensive_call();
  let b = other_call(a);
  a + b
};
```

Block expression value is the last expression; if last line ends with `;`, value is `()`.

## let Binding

```
let x = 42;                 // inferred type
let y: Int = 42;            // explicit type
let mut z = 0;              // mutable binding
let (a, b) = (1, 2);        // destructuring
let Point { x, y } = point; // record destructuring
```

**`let mut` semantics:**
- Applies to @copy and @move types
- Only allows rebinding (replacement), not mutation
- For mutable references, use explicit `&mut`

## Pattern Matching

```
match value {
  0 => "zero",
  1 | 2 | 3 => "small",
  4..=9 => "medium",
  10.. => "large",
  n => "got {n}",
  n if n < 0 => "negative",
  Some(x) => "value: {x}",
  None => "nothing",
  Point { x: 0, y: 0 } => "origin",
  Point { x, y } => "at ({x}, {y})",
  _ => "other",
}
```

Exhaustiveness checking is enforced by compiler.

## Control Flow

```
if condition { ... } else if cond2 { ... } else { ... }

for x in collection { }

while condition { }

loop {
  if done { break value; }
}
```

`break` can carry values (only in `loop`). `continue` goes to next iteration.

## Operators

| Category | Operators |
|----------|-----------|
| Arithmetic | `+ - * / %` |
| Comparison | `== != < > <= >=` |
| Logical | `&& || !` |
| Assignment | `= += -= *= /= %=` |
| Access | `.` `[]` |
| Reference | `&` `&mut` |
| Error propagation | `?` |
| Range | `..` `..=` |

No operator overloading. No implicit numeric conversions.

## Error Propagation `?`

```
fn parse_two(a: String, b: String) -> <exn<ParseError>> (Int, Int) {
  (parse(a)?, parse(b)?)
}
```

`?` only works on `Result<T, E>` types or calls with `exn<E>` effect. Expands to match on Ok/Err.
