# 1. Design Principles

Four axioms guide the design:

## P1: Local Reasoning

Expression types, effects, and ownership depend only on function signature and current scope bindings. No implicit global state, dynamic scope, runtime reflection, or macro rewriting.

## P2: Signature as Contract

Function signatures include three dimensions: type, effect, ownership. Reading the signature reveals what the function does, what resources it needs, and how it handles inputs.

## P3: Redundancy Over Compactness

LLM context is cheap; explicit is preferred. Repeating type names is better than clever omission rules. No convenient but error-prone syntactic sugar.

## P4: Error Localization

Parse, type, effect, and ownership errors must point to the smallest identifiable syntactic unit. No macro chain or inference chain expansion. Structured output for LLM fixes.

# VM Design

Core idea: Ect has all possible types and checks inferenced during compile time, and runtime VM doesn't need to know them.

## P1: Run in any Rust application

The VM is designed in less than 300 lines so you can add it and execute compiled bytecode to any Rust application. It is simple so it can be transplanted to any platforms easily too.

## P2: Safety

Runtime safety is ensured because of simplicity.

## P3: Debug

Debugger, profiler and trace are made easy.

These are checked and inferenced during compile:

- Types
- Effects
- Ownership and borrows
- Region
- Generics monomorphization
- Trait dispatch
- Async transform
- Effect handler lowering
- Pattern match
- Const fn and expr

These are made to runtime:

- Bytecode
- Memory management (move, copy, drop, ref count)
- Arithmetics
- Control flow
- Method calls
- Host function calls
