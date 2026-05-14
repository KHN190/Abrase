### 1. Effect Subtyping and Variance

While you verify effect compatibility, you haven't tested **Effect Subtyping**.

* **Missing Logic**: In a sound effect system, a function that produces `<pure>` effects should be compatible with a context that expects `<pure, io>`. This is "Effect Subsumption".
* **Test Case**: Verify that a function returning `<pure>` can be used where a `<pure, exn>` is required.

### 2. Qualified Name Resolution for Types

The BNF defines `<qualified-name>` as `identifier ('.' identifier)* ('.' type-name)?`.

* **Missing Logic**: Your current tests use simple strings for names like `"Int"` or `"std.io.Error"`. You need to verify that the type checker can resolve nested names through the module hierarchy you've built in Phase 15.
* **Test Case**: If in module `root`, can you resolve `root.io.File` vs just `io.File`?

### 3. Closure Effect Inference

BNF includes optional effect sets for closures.

* **Missing Logic**: If a closure doesn't declare its effects, they must be inferred from the body. If it **does** declare them, the body must be checked against that declaration.
* **Test Case**: A closure declared as `|x| -> <pure> x + call_io()` should trigger a static type error.

### 4. Record and Variant Body Validation

The BNF defines `<type-body>` as either a `<record-body>` or a `<variant-body>`.

* **Missing Logic**: You have tests for `Record` expressions, but you need to verify that a record is **exhaustively** initialized (all fields present) and that variant constructor arguments match the type definition.
* **Test Case**: Creating `Point { x: 1 }` should fail if the definition requires `{ x: Int, y: Int }`.

### 5. String Interpolation Type Safety

The BNF includes `<interpolation> ::= '{' <identifier> ('.' <identifier>)* '}'`.

* **Missing Logic**: Static checking for string literals should verify that the identifiers inside `{}` are defined and have a type that can be formatted (perhaps requiring a `Show` trait bound).

### 6. Variance in Generics

* **Missing Logic**: For `<generic-instance>` like `List<T>`, you need to decide if `List<String>` is a subtype of `List<Any>`.
* **Test Case**: Verifying covariance, contravariance, or invariance for generic parameters.

### 7. Completeness of "Exhaustiveness"

While you have wildcard checks, a fully compliant BNF checker should perform **Exhaustiveness Analysis** for Variants.

* **Missing Logic**: If a `match` expression checks an `Option<T>` variant, it must prove that both `Some` and `None` are handled.
