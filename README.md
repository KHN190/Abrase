# Abrase

<img src="./abrase.png" alt="Abrase" width="100" style="float: right; margin-left: 20px;">

Abrase (`.abe`, abbreviated **Abe**) is a Rust-inspired language explicitly optimized for language models to effectively utilize long context windows. Abrase source compiles to **Polka** bytecode, which runs on the **Myriad** runtime.

Compiler type & behavior checks are rendered explicit to hinder hallucination and assist local inference. It features:

* Static type check
* Effect system
* Simplified lifecycle management

It can be added to **any Rust application**. See [wiki](https://github.com/KHN190/Abrase/wiki).

## Syntax

```rust
// Types, values, control flow

fn main() -> Int {
  let x = 5;
  let mut y = 10;
  y = y + x;

  if y > 12 { y } else { 0 }
}

// Records, variants, pattern matching

type Tree = Leaf | Node(Int, Tree, Tree)

fn insert(t: Tree, x: Int) -> Tree {
  match t {
    Leaf            => Node(x, Leaf, Leaf)
    Node(v, l, r)   => if x < v { Node(v, insert(l, x), r) }
                       else     { Node(v, l, insert(r, x)) }
    _ => t
  }
}

// Closures with capture-by-value and `move`

fn main() -> Int {
  let bump = 5;
  let add_bump = |n: Int| n + bump;   // bump is cloned into the env
  add_bump(3) + bump                  // bump still usable here
}

// Exceptions via `<exn>` effect

fn div(a: Int, b: Int) -> <exn<Int>> Int {
  if b == 0 { throw 99 } else { a / b }
}

fn pipeline(a: Int, b: Int) -> <exn<Int>> Int {
  let v = div(a, b)?;                  // propagate Err, unwrap Ok
  v + 1
}

fn main() -> Int {
  match pipeline(20, 4) { Ok(v) => v, Err(_) => -1, _ => 0 }
}

// Effect handlers

effect Logger { log(s: String) -> Unit }

fn work() -> <Logger> Int {
  Logger.log("starting");
  42
}

fn main() -> Int {
  handle work() {
    return v       => v,
    Logger.log(_)  => resume(())
  }
}

// String interpolation

fn greet(name: String) -> String { "hello {name}" }

// Ownership annotations

@copy  type Pt    = { x: Int, y: Int }       // bitwise copy on assign
@move  type Buf   = { data: [Int; 1024] }    // ownership transfers
@share type Cfg   = { host: String }         // refcounted
```

## Benchmarks

### Naive recursive

```rust
fn fib(n: Int) -> Int {
  if n < 2 { n } else { fib(n - 1) + fib(n - 2) }
}

fn main() -> Int {
  fib(30)
}
```

| Runtime | Time | vs. |
|---|---|---|
| Abrase | _137 ms_ | 3.2× |
| Python 3 (CPython) | _131 ms_ | _3.1x_ |
| Node.js (V8) | _41.9 ms_ | _1.0x_ |

### String Operation

```rust
fn build(n: Int) -> String {
  let mut result = "x";
  let mut i = n;
  while i > 0 {
    result = "{result}y";
    i = i - 1;
  }
  return result
}

fn main() -> String {
  let s = build(300);
  return s
}
```

| Runtime | Time | vs. |
|---|---|---|
| Abrase | _2.2 ms_ | 1× |
| Python 3 (CPython) | _22 ms_ | _10x_ |
| Node.js (V8) | _34.1 ms_ | _15x_ |

* _The experimental version has zero compiler optimization._
* _Reproduce with [hyperfine](https://github.com/sharkdp/hyperfine)_.

## Polka — bytecode design

* 38 opcodes, 4 bytes each.
* 256 registers per frame, 64-bit each.
* Device interaction through ports definition.

```h
[HEADER, 40 bytes]
  magic:4              = 0xECFF00EC
  version:2            = 0x0100  (1.0)
  flags:2              reserved
  device_mask:32       bitmap of required device IDs (256 bits)
  const_offset:4       byte offset to constants section
  fn_table_offset:4    byte offset to function table
  code_offset:4        byte offset to code section
  debug_offset:4       byte offset to debug section (0 if absent)

[CONSTANTS SECTION]
  count:4
  constant_0:8         (one 64-bit word per entry)
  constant_1:8
  ...

[FUNCTION TABLE]
  count:4
  entry_0: fn_id:2  reg_count:1  param_count:1  code_offset:4  code_size:4
  entry_1: ...
  ...

[CODE SECTION]
  fn_0_bytecode  (4 × instruction_count bytes)
  fn_1_bytecode
  ...

[DEBUG SECTION]  (optional, may be stripped)
  source_lines:    pc → (file_id, line, col)
  symbol_names:    function and parameter names
  type_names:      for pretty-printing
```

See [`Wiki / Bytecode Spec`](./wiki/appendix-bytecode-spec.md).
