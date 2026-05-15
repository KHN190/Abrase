# 2. Lexical Structure

Minimize implicit conversions and parser ambiguity. Everything explicit, like identifiers follow strict naming conventions.

UTF-8 source files with keywords, identifiers (Unicode), comments, literals, and block delimiters.

```rust
// keywords
fn, let, const, if, match, for, while, loop, async, await, handle, throw

// identifier rules
TypeName, value_name, effect_name, _

// literals
42, 3.14, true, 'a', "hello {name}", ()

// syntax
{ stmt; stmt; expr }
```

See BNF spec in appendix for formal lexical grammar.
