# abrase (CLI)

Command-line tool for the [Abrase](https://github.com/KHN190/Abrase) language.

## Install

```sh
cargo install abrase-cli --version 0.1.0
```

Puts an `abrase` binary on your PATH.

## Use

```sh
abrase run    [--debug] file.abe   # parse, compile, execute main()
abrase check  file.abe              # type-check only
abrase parse  file.abe              # dump AST
abrase disasm file.abe              # dump Polka bytecode
abrase export file.abe out.pk       # compile to a .pk cartridge
abrase load   [--debug] file.pk     # load and run a prebuilt .pk
```

Example:

```sh
$ echo 'fn main() -> Int { 6 * 7 }' > answer.abe
$ abrase run answer.abe
42
```

See the [main repo](https://github.com/KHN190/Abrase) for the language guide
and `examples/`.

## License

MIT
