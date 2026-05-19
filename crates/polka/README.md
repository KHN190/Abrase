# Polka

Polka is the bytecode that runs on the [Myriad](https://crates.io/crates/myriad). It compiles from [Abrase](https://github.com/KHN190/Abrase) language.

The crate defines:

- instruction set
- bytecode layout

> Published as `polka-rs` (the bare `polka` name was taken). The Rust import
> path stays `polka` — add to your `Cargo.toml` as
> `polka = { package = "polka-rs", version = "0.1.0-alpha.1" }` and keep
> `use polka::*;` unchanged.

See the [main repo](https://github.com/KHN190/Abrase) for the full language,
runtime, and examples.

## Status

Experimental, schema stable for 0.1.x.

## License

MIT
