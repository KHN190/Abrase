# Abrase

<img src="./abrase.png" alt="Abrase" width="100" style="float: right; margin-left: 20px;">

Abrase (`.abe`, abbreviated **Abe**) is a Rust-inspired language designed for code review, not just human authorship. Abrase source compiles to **Polka** bytecode, which runs on the **Myriad** runtime.

It features:

* Static type check
* Effect system
* Simplified lifecycle management

We also include Myriad as a safe sandbox environment, which can later compile to native or transplanted to any platform in a few days due to simplicity design.

It can be added to **any Rust application**. See [wiki](https://github.com/KHN190/Abrase/wiki).

> Why design is hard? The simple answer is, you had too much freedom. Then why design becomes easy? Because you prisoned yourself with taste, and with freedom gone, you are left with only choices.

> By designing a programming language you become clearer with your taste, your limitations, and you find the same joy writing a very short poem.

## Language Overview

```rust
effect Metric {
  op record(msg: String) -> Unit
}

fn random_walk(steps: Int) -> <Metric> Int {
  let t0 = now();
  Metric.record("start: {t0}");
  srand(0.42);

  let mut pos = 0;
  let mut i = 0;
  while i < steps {
    let r = rand();
    pos = pos + if r < 0.5 { 2 } else { -1 };
    i = i + 1
  };

  let t1 = now();
  Metric.record("time: {t1}");
  let d = pos.abs();
  Metric.record("I am at: {d}");
  pos
}

fn main() -> Int {
  handle random_walk(1000) {
    return pos => pos,
    Metric.record msg => {
      println(msg);
      resume(())
    }
  }
}
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
  let s = build(10000);
  return s
}
```

| Runtime | Time | vs. |
|---|---|---|
| Abrase | _19.3 ms_ | 1× |
| Python 3 (CPython) | _22.7 ms_ | _1.2x_ |
| Node.js (V8) | _34.1 ms_ | _1.7x_ |

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
