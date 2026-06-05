<p align="center">
  <a href="https://khn190.github.io/abrase/"><img src="./assets/banner.svg" alt="Abrase" width="720"></a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/dependencies-zero-2b2620?labelColor=f4efe2" alt="zero dependencies">
  <img src="https://img.shields.io/badge/memory-no_GC%2C_leak_free-2b2620?labelColor=f4efe2" alt="no GC, leak free">
  <img src="https://img.shields.io/badge/rustc-1.85+-2b2620?labelColor=f4efe2" alt="rustc 1.85+">
  <a href="https://khn190.github.io/abrase/"><img src="https://img.shields.io/badge/playground-wasm-d97757?labelColor=f4efe2" alt="playground"></a>
</p>

<p align="center">
  <a href="https://github.com/KHN190/Abrase/actions/workflows/ci.yml"><img src="https://github.com/KHN190/Abrase/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://codecov.io/gh/KHN190/Abrase"><img src="https://codecov.io/gh/KHN190/Abrase/branch/dev/graph/badge.svg" alt="codecov"></a>
  <a href="https://crates.io/crates/abrase-cli"><img src="https://img.shields.io/crates/v/abrase-cli.svg?label=abrase-cli" alt="crates.io"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="MIT"></a>
</p>

Abrase (`.abe`, abbreviated **Abe**) is a Rust-inspired language. Abrase source compiles to **Polka** bytecode, which runs on the **Myriad** runtime. The interpreter can be added to any Rust application and compile to Rust (the latter is WIP).

It features:

* Strong typed
* Algebraic effects
* Region memory lifecycle design - no GC, leak free
* Myriad runtime — computation core
* Linter & Debugger

It can be added to **any Rust application**. See [wiki](https://github.com/KHN190/Abrase/wiki).

Try it now in your [browser](https://khn190.github.io/abrase/).

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
