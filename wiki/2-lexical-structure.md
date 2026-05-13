# Lexical Structure

## Encoding

Source files use UTF-8 encoding. Identifiers allow Unicode characters; keywords are ASCII only.

## Keywords

```
fn        let        const      if         else
match     for        while      loop       break
continue  return     type       trait      impl
import    mod        pub        scope      region
handle    throw      true       false      where
async     await      in         as         self
Self      mut        thread     _
```

## Identifiers

```
identifier   ::= XID_Start XID_Continue*
type_name    ::= UpperLetter (Letter | Digit)*
value_name   ::= LowerLetter (Letter | Digit | "_")*
effect_name  ::= LowerLetter (Letter | Digit | "_")*
```

## Comments

```
// line comment
```

## Literals

| Type | Example |
|------|---------|
| Integer | `42` |
| Float | `3.14` |
| Boolean | `true`, `false` |
| Character | `'a'`, `'\n'`, `'\u{1F600}'` |
| String | `"hello"`, `"line1\nline2"` |
| String interpolation | `"hello {name}, you are {age}"` (literals only) |
| Unit | `()` |

String interpolation expressions must be simple identifiers or field access.

## Separation and Blocks

- Statements end with semicolon `;`
- Code blocks use `{ }`, no `;` after `{ }`
- No indentation-sensitive syntax
