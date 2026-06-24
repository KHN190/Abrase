<p align="center">
  <a href="https://khn190.github.io/abrase/"><img src="./banner.svg" alt="Abrase" width="720"></a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/dependencies-zero-black" alt="zero dependencies">
  <img src="https://img.shields.io/badge/memory-no_GC%2C_leak_free-black" alt="leak free">
  <img src="https://img.shields.io/badge/rustc-1.85+-black" alt="rustc 1.85+">
  <a href="https://khn190.github.io/abrase/"><img src="https://img.shields.io/badge/playground-wasm-black" alt="playground"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-black" alt="MIT"></a>
</p>

<p align="center">
  <a href="https://github.com/KHN190/Abrase/actions/workflows/ci.yml"><img src="https://github.com/KHN190/Abrase/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://codecov.io/gh/KHN190/Abrase"><img src="https://codecov.io/gh/KHN190/Abrase/branch/dev/graph/badge.svg" alt="codecov"></a>
  <a href="https://crates.io/crates/abrase-cli"><img src="https://img.shields.io/crates/v/abrase-cli.svg?label=abrase-cli" alt="crates.io"></a>
</p>

Abrase (`.abe`, abbreviated **Abe**) is a small, strongly-typed language with algebraic effects and region-based memory. Source compiles to **Polka** bytecode on the **Myriad** runtime.

One source, three ways to run:

* Add to any Rust application — zero-dependency runtime.
* Transpile to Rust (no std), run on bare metal.
* In the browser — try it [here](https://khn190.github.io/abrase/).

It is a self-hosting language. Its own memory management — allocation, reference counting, regions — is written in Abrase, equivalent to native-speed Rust.

Plus algebraic effects & handlers, a linter, and a debugger. See [wiki](https://github.com/KHN190/Abrase/wiki).

## Installation

```bash
cargo install abrase-cli

# and use
abrase run hello.abe
abrase disasm examples/nqueens.abe
```

Or download pre-compiled [GitHub Releases](https://github.com/KHN190/Abrase/releases).

## Code Editor

- [Sublime Text](https://github.com/KHN190/Abrase-sublime)
- [VS Code](https://github.com/KHN190/Abrase-vscode)

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

Generally 2.5x better than CPython. On pure function tasks, could be ~20x faster. In most cases, it's equivalent to JIT like JavaScript in V8 engine. See [`Wiki / Optimizations`](./wiki/12-Optimizations.md).

Reproduce with [hyperfine](https://github.com/sharkdp/hyperfine).

## Polka — bytecode design

* 46 opcodes, 4 bytes each. Data & opcode are aligned.
* 128 registers per frame.

See [`Wiki / Bytecode Spec`](./wiki/Appendix-Bytecode-Spec.md).

Read about the blog [here](medium.com/p/05cb0e4df3e5).
