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
