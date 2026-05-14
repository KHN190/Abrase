# Appendix C: Open Questions

The following design points are not yet finalized:

1. **Default type for numeric literals**: Current `Int = Int64`, `Float = Float64`. Too wide for embedded scenarios?
2. **String indexing semantics**: Current forbids `s[i]`, requires `s.chars()` or `s.bytes()`. Too strict?
3. **Source code visibility of region labels**: Should explicit `in r` annotation be forced in certain scenarios?
4. **Default capture mode for closures**: Current default borrow, explicit `move`. Would reverse be more intuitive?
5. **Panic catchability**: Is `<panic>` effect needed? Current design: completely uncatchable.
6. **Effect hierarchy or subtyping**: When calling `<a, b>` function with `<a>` requirement, should auto-allow? Current: auto-extend.
7. **Trait coherence**: Is orphan rule too strict? Some cross-module scenarios may be limited.
8. **Async runtime specification**: Language doesn't specify runtime, but Rust must provide. Need standard runtime interface?
9. **Should `Shared<T>` be retained**: Previous discussion favored retention (immutable only). Final confirmation needed?
