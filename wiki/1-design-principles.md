# 1. Design Principles

Ect is a Rust dialect with static types, effect system, region-based lifetime management.

Ect makes code explicit and checkable at compile time, so LLMs can reason about code with minimal local context, and the runtime stays simple after compiler. All side effects, ownership, and type constraints visible in signatures.

## Language Design

* Signature as contract — Signatures declare type, effect, and ownership.
* Explicity over implicity.
* Redundancy over compactness.
* Error localization.

## Virtual Machine Design

The bytecode is **Polka**; the host runtime that executes Polka is called **Myriad**. Compiling Ect produces a Polka module (`.pk`); any conforming Myriad host can load and run it.

* Minimal — Polka is a small instruction set; Myriad fits in any Rust app.
* Safety — enforced by the compiler; Myriad executes only.
* Debuggable — Polka's simplicity enables profilers, debuggers, and traces on top of Myriad.

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

