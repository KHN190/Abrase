# Abrase

[![CI](https://github.com/KHN190/Abrase/actions/workflows/ci.yml/badge.svg)](https://github.com/KHN190/Abrase/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/KHN190/Abrase/branch/dev/graph/badge.svg)](https://codecov.io/gh/KHN190/Abrase)
[![crates.io](https://img.shields.io/crates/v/abrase-cli.svg?label=abrase-cli)](https://crates.io/crates/abrase-cli)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

<img src="./abrase.png" alt="Abrase" width="100" align="right">

Abrase (`.abe`, abbreviated **Abe**) is a Rust-inspired language made and designed for large language models. Abrase source compiles to **Polka** bytecode, which runs on the **Myriad** runtime.

It features:

* Static type check
* Effect system
* Simplified lifecycle management
* Performative & Leak free

It can be added to **any Rust application**. See [wiki](https://github.com/KHN190/Abrase/wiki).

## Installation

```bash
cargo install abrase-cli

# and use
abrase run hello.abe
abrase disasm examples/nqueens.abe
```

Or download pre-compiled [GitHub Releases](https://github.com/KHN190/Abrase/releases).

## Language Overview

```rust
effect Metric {
  op record(msg: String) -> Unit
}

fn fib(n: Int) -> <Metric> Int {
  Metric.record("entering fib({n})");
  if n < 2 { n } else { fib(n - 1) + fib(n - 2) }
}

fn main() -> Int {
  handle fib(10) {
    return v       => v,
    Metric.record msg => {
      println(msg);
      resume(())
    }
  }
}
```

## Benchmarks

Generally 1.3~2x better than CPython. On specific smaller tasks, could be ~10x faster.

* _See `wiki/12-Optimizations.md`._
* _Reproduce with [hyperfine](https://github.com/sharkdp/hyperfine)_.

## Polka — bytecode design

* 46 opcodes, 4 bytes each.
* 64 registers per frame, 64-bit each.
* Untagged `u64` values; type is implied by the opcode and a per-frame handle mask.
* Effect / region machinery via reserved port encodings (`0xE0` / `0xE1`); user-visible I/O via four core devices (System, Console) and imports.

```h
HEADER (8 bytes)
  magic:4              0xECFF00EC
  version:2            0x0100
  flags:2

FUNCTION TABLE
  count:4
  entry                { fn_id:2, reg_count:1, param_count:1, code_offset:4 }

DATA POOL
  count:4
  values:8 x count     scalar literals + string-pool handles

CODE
  4 bytes per instruction
```

See [`Wiki / Bytecode Spec`](./wiki/Appendix-Bytecode-Spec.md).

Read about the blog [here](medium.com/p/05cb0e4df3e5).
