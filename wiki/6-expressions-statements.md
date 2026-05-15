# 6. Expressions & Statements

Everything is an expression, that everything returns a value. 

* `if`, `match`, `{ ... }` are expressions. 
* `let` binds with pattern matching. 
* Control flow: `for`, `while`, `loop`, `break`, `continue`. 
* Operators, no overloading. 
* `?` propagates `exn<E>`.

```rust
let max = if a > b { a } else { b };

let label = match shape {
  Circle { r } => "circle",
  Rect { .. } => "rect",
  Point => "point",
};

let (a, b) = (1, 2);
let Point { x, y } = point;
let mut z = 0;

match value { 0 => "zero", 1..3 => "small", n => "got {n}" }

for x in list { }
loop { if done { break x; } }

parse(a)?, handle?.method, condition ? a : b  // propagation
```

Block value is last expression (no trailing `;`). If last line ends with `;`, value is `()`. `break` can carry a value (in `loop` only). Exhaustiveness check is enforced.
