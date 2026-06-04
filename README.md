# Abrase

[![CI](https://github.com/KHN190/Abrase/actions/workflows/ci.yml/badge.svg)](https://github.com/KHN190/Abrase/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/KHN190/Abrase/branch/dev/graph/badge.svg)](https://codecov.io/gh/KHN190/Abrase)
[![crates.io](https://img.shields.io/crates/v/abrase-cli.svg?label=abrase-cli)](https://crates.io/crates/abrase-cli)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

<img src="./abrase.png" alt="Abrase" width="100" align="right">

Abrase (`.abe`, abbreviated **Abe**) is a Rust-inspired language made for LLM & efficiency. Abrase source compiles to **Polka** bytecode, which runs on the **Myriad** runtime.

It features:

* Strong typed
* Algebraic effects
* Region memory lifecycle design - no GC, leak free
* Myriad runtime — computation core, or OS embedded
* Linter & Debugger

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

Generally 1.3~2x better than CPython. On specific smaller tasks, could be ~10x faster. See [`Wiki / Optimizations`](./wiki/12-Optimizations.md).

Reproduce with [hyperfine](https://github.com/sharkdp/hyperfine).

## Polka — bytecode design

* 46 opcodes, 4 bytes each. Data & opcode are aligned.
* 128 registers per frame.

See [`Wiki / Bytecode Spec`](./wiki/Appendix-Bytecode-Spec.md).

Read about the blog [here](medium.com/p/05cb0e4df3e5).
