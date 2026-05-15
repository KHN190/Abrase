# 1. Design Principles

Ect is a Rust dialect with static types, effect system, region-based lifetime management.

Ect makes code explicit and checkable at compile time, so LLMs can reason about code with minimal local context, and the runtime stays simple thanks to compiler. All side effects, ownership, and type constraints visible in signatures.

## Language Design

* Signature as contract — Signatures declare type, effect, and ownership.
* Explicity over implicity.
* Redundancy over compactness.
* Error localization.

## Virtual Machine Design

Bytecode is the product.

* Minimal - 40 opcodes; runs in any Rust app.
* Safety - enforced by compiler; runtime executes only.
* Debuggable - simplicity enables profilers, debuggers, traces.

## Compile vs. Runtime

Compile time resolves: 

* types, 
* effects, 
* ownership, 
* regions, 
* generics monomorphization, 
* trait dispatch, 
* handlers, 
* pattern match, 
* const evaluation.

Runtime cares: 

* bytecode execution, 
* memory (move/copy/drop/refcount), 
* arithmetic, 
* control flow, 
* device I/O, 
* effect handlers (handle, resume, continuation cells).

