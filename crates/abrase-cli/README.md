# abrase (CLI)

Command-line tool for the [Abrase](https://github.com/KHN190/Abrase) language.

## Install

```sh
cargo install abrase-cli --version 0.1.0-alpha.1
```

`0.1.x` is pre-release; cargo skips it unless you ask for it by version.
This puts an `abrase` binary on your PATH.

## Use

```sh
abrase run    [--debug] file.abe   # parse, compile, execute main()
abrase check  file.abe              # type-check only
abrase parse  file.abe              # dump AST
abrase disasm file.abe              # dump Polka bytecode
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
