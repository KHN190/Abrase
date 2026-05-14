# 16. Bytecode Specification

This specification defines the stable interface. Implementations are free to optimize, add tooling, or extend for performance—as long as the bytecode behavior matches. The format itself is the product; the VMs are the implementations.

## 1. Philosophy

1. Bytecode is the stable artifact. Source code, IR, and VMs are implementations. The .ecm (ECT Module) binary format is the contract between compiler and runtime.

2. Type erasure at boundaries. The compiler performs all type checking. At runtime, values are scalars (64-bit), pointers (GC/stack refs), or composites (layout-aware). No runtime type information exists.

3. Static compilation of dynamic features. Effects (exn, async, custom), generics (monomorphized), and ownership (explicit moves/drops) are compiled away. The VM sees only primitive operations.

## 2. Instruction Encoding

All instructions are 4 bytes: `[opcode:8][reg_a:8][reg_b:8][reg_c:8]`

Register indices are 0–255 per function frame.

### 2.1 Instruction Categories

Arithmetic (10 opcodes: 0x00–0x09)

  ┌──────┬────────────────────┬─────────────────────────────────────────────┐
  │  Op  │      Mnemonic      │                  Semantics                  │
  ├──────┼────────────────────┼─────────────────────────────────────────────┤
  │ 0x00 │ add r_a, r_b, r_c  │ r_a := r_b + r_c (i64)                      │
  ├──────┼────────────────────┼─────────────────────────────────────────────┤
  │ 0x01 │ sub r_a, r_b, r_c  │ r_a := r_b - r_c (i64)                      │
  ├──────┼────────────────────┼─────────────────────────────────────────────┤
  │ 0x02 │ mul r_a, r_b, r_c  │ r_a := r_b * r_c (i64)                      │
  ├──────┼────────────────────┼─────────────────────────────────────────────┤
  │ 0x03 │ div r_a, r_b, r_c  │ r_a := r_b / r_c (i64, trap on div-by-zero) │
  ├──────┼────────────────────┼─────────────────────────────────────────────┤
  │ 0x04 │ mod r_a, r_b, r_c  │ r_a := r_b % r_c (i64)                      │
  ├──────┼────────────────────┼─────────────────────────────────────────────┤
  │ 0x05 │ neg r_a, r_b       │ r_a := -r_b (i64)                           │
  ├──────┼────────────────────┼─────────────────────────────────────────────┤
  │ 0x06 │ fadd r_a, r_b, r_c │ r_a := r_b + r_c (f64, IEEE 754)            │
  ├──────┼────────────────────┼─────────────────────────────────────────────┤
  │ 0x07 │ fsub r_a, r_b, r_c │ r_a := r_b - r_c (f64)                      │
  ├──────┼────────────────────┼─────────────────────────────────────────────┤
  │ 0x08 │ fmul r_a, r_b, r_c │ r_a := r_b * r_c (f64)                      │
  ├──────┼────────────────────┼─────────────────────────────────────────────┤
  │ 0x09 │ fdiv r_a, r_b, r_c │ r_a := r_b / r_c (f64)                      │
  └──────┴────────────────────┴─────────────────────────────────────────────┘

Overflow/underflow in integer arithmetic is not trapped; behavior matches two's complement. `NaN/Inf` in float ops is not trapped.

Comparison (3 opcodes: 0x0a–0x0c)

  ┌──────┬───────────────────┬─────────────────────────────────────────┐
  │  Op  │     Mnemonic      │                Semantics                │
  ├──────┼───────────────────┼─────────────────────────────────────────┤
  │ 0x0a │ eq r_a, r_b, r_c  │ r_a := (r_b == r_c) ? 1 : 0             │
  ├──────┼───────────────────┼─────────────────────────────────────────┤
  │ 0x0b │ lt r_a, r_b, r_c  │ r_a := (r_b < r_c) ? 1 : 0 (i64 signed) │
  ├──────┼───────────────────┼─────────────────────────────────────────┤
  │ 0x0c │ flt r_a, r_b, r_c │ r_a := (r_b < r_c) ? 1 : 0 (f64)        │
  └──────┴───────────────────┴─────────────────────────────────────────┘

Comparison returns 0 (false) or 1 (true) in r_a.

Bitwise (5 opcodes: `0x0d–0x11`)

  ┌──────┬───────────────────┬──────────────────────────────────────┐
  │  Op  │     Mnemonic      │              Semantics               │
  ├──────┼───────────────────┼──────────────────────────────────────┤
  │ 0x0d │ and r_a, r_b, r_c │ r_a := r_b & r_c                     │
  ├──────┼───────────────────┼──────────────────────────────────────┤
  │ 0x0e │ or r_a, r_b, r_c  │ r_a := r_b | r_c                     │
  ├──────┼───────────────────┼──────────────────────────────────────┤
  │ 0x0f │ xor r_a, r_b, r_c │ r_a := r_b ^ r_c                     │
  ├──────┼───────────────────┼──────────────────────────────────────┤
  │ 0x10 │ shl r_a, r_b, r_c │ r_a := r_b << r_c                    │
  ├──────┼───────────────────┼──────────────────────────────────────┤
  │ 0x11 │ shr r_a, r_b, r_c │ r_a := r_b >> r_c (arithmetic shift) │
  └──────┴───────────────────┴──────────────────────────────────────┘

Control Flow (5 opcodes: `0x12–0x16`)

  ┌──────┬─────────────────────────────────────┬────────────────────────────────────────────────┐
  │  Op  │              Mnemonic               │                   Semantics                    │
  ├──────┼─────────────────────────────────────┼────────────────────────────────────────────────┤
  │ 0x12 │ jmp offset (reg_a holds offset as   │ PC += offset                                   │
  │      │ i32)                                │                                                │
  ├──────┼─────────────────────────────────────┼────────────────────────────────────────────────┤
  │ 0x13 │ jz r_a, offset                      │ if r_a == 0, PC += offset                      │
  ├──────┼─────────────────────────────────────┼────────────────────────────────────────────────┤
  │ 0x14 │ jnz r_a, offset                     │ if r_a != 0, PC += offset                      │
  ├──────┼─────────────────────────────────────┼────────────────────────────────────────────────┤
  │ 0x15 │ call fn_id, r_a..r_c                │ Call function by ID; args in r_a–r_c; result   │
  │      │                                     │ in r_a                                         │
  ├──────┼─────────────────────────────────────┼────────────────────────────────────────────────┤
  │ 0x16 │ ret r_a                             │ Return value in r_a to caller                  │
  └──────┴─────────────────────────────────────┴────────────────────────────────────────────────┘

Offsets are signed 32-bit. fn_id is stored in reg_a (conceptually; encoding is call `[fn_id:16][nargs:8][reserved:8]` in the 4-byte instruction by repurposing register fields).

Memory (8 opcodes: `0x17–0x1e`)

  ┌──────┬───────────────────────┬──────────────────────────────────────────────────────────────┐
  │  Op  │       Mnemonic        │                          Semantics                           │
  ├──────┼───────────────────────┼──────────────────────────────────────────────────────────────┤
  │ 0x17 │ ld r_a, [r_b +        │ Load from address r_b + offset (8 bytes) into r_a            │
  │      │ offset]               │                                                              │
  ├──────┼───────────────────────┼──────────────────────────────────────────────────────────────┤
  │ 0x18 │ st r_a, [r_b +        │ Store r_a to address r_b + offset                            │
  │      │ offset]               │                                                              │
  ├──────┼───────────────────────┼──────────────────────────────────────────────────────────────┤
  │ 0x19 │ ldi r_a, imm          │ Load 64-bit immediate into r_a (inline in next 8 bytes)      │
  ├──────┼───────────────────────┼──────────────────────────────────────────────────────────────┤
  │ 0x1a │ lea r_a, [r_b +       │ Load effective address: r_a := r_b + offset                  │
  │      │ offset]               │                                                              │
  ├──────┼───────────────────────┼──────────────────────────────────────────────────────────────┤
  │ 0x1b │ mov r_a, r_b          │ r_a := r_b                                                   │
  ├──────┼───────────────────────┼──────────────────────────────────────────────────────────────┤
  │ 0x1c │ movz r_a              │ r_a := 0                                                     │
  ├──────┼───────────────────────┼──────────────────────────────────────────────────────────────┤
  │ 0x1d │ move r_a, r_b         │ Ownership move: r_a := r_b; drop r_b (calls destructor if    │
  │      │                       │ needed)                                                      │
  ├──────┼───────────────────────┼──────────────────────────────────────────────────────────────┤
  │ 0x1e │ copy r_a, r_b         │ Ownership copy: r_a := r_b (no destructor call)              │
  └──────┴───────────────────────┴──────────────────────────────────────────────────────────────┘

ldi is followed by 8 immediate bytes. Offset is signed 16-bit or inlined in the 4-byte instruction.

Heap (4 opcodes: `0x1f–0x22`)

  ┌──────┬────────────────┬─────────────────────────────────────────────────────────────────────┐
  │  Op  │    Mnemonic    │                              Semantics                              │
  ├──────┼────────────────┼─────────────────────────────────────────────────────────────────────┤
  │ 0x1f │ alloc r_a,     │ Allocate size bytes on heap; address in r_a                         │
  │      │ size           │                                                                     │
  ├──────┼────────────────┼─────────────────────────────────────────────────────────────────────┤
  │ 0x20 │ free r_a       │ Free heap memory at address r_a (no-op if RC; RC decrement if       │
  │      │                │ Shared)                                                             │
  ├──────┼────────────────┼─────────────────────────────────────────────────────────────────────┤
  │ 0x21 │ drop r_a       │ Call destructor for value in r_a (lookup from type table)           │
  ├──────┼────────────────┼─────────────────────────────────────────────────────────────────────┤
  │ 0x22 │ ref r_a, r_b   │ Borrow r_b immutably; ref handle in r_a                             │
  └──────┴────────────────┴─────────────────────────────────────────────────────────────────────┘

Host (2 opcodes: `0x23–0x24`)

  ┌──────┬──────────────┬───────────────────────────────────┐
  │  Op  │   Mnemonic   │             Semantics             │
  ├──────┼──────────────┼───────────────────────────────────┤
  │ 0x23 │ import fn_id │ Call imported host function by ID │
  ├──────┼──────────────┼───────────────────────────────────┤
  │ 0x24 │ export fn_id │ Expose function as host export    │
  └──────┴──────────────┴───────────────────────────────────┘

Async (3 opcodes: `0x25–0x27`)

  ┌──────┬──────────────────────┬───────────────────────────────────────────────────────────────┐
  │  Op  │       Mnemonic       │                           Semantics                           │
  ├──────┼──────────────────────┼───────────────────────────────────────────────────────────────┤
  │ 0x25 │ spawn scope_id,      │ Spawn coroutine in scope; handle in r_a                       │
  │      │ fn_id                │                                                               │
  ├──────┼──────────────────────┼───────────────────────────────────────────────────────────────┤
  │ 0x26 │ await r_a            │ Suspend until r_a (handle) completes; resume at next          │
  │      │                      │ instruction                                                   │
  ├──────┼──────────────────────┼───────────────────────────────────────────────────────────────┤
  │ 0x27 │ yield                │ Yield to scheduler (cooperative point)                        │
  └──────┴──────────────────────┴───────────────────────────────────────────────────────────────┘

Effect Handlers (2 opcodes: `0x28–0x29`)

  ┌──────┬──────────────────┬─────────────────────────────────────────────────────────────┐
  │  Op  │     Mnemonic     │                          Semantics                          │
  ├──────┼──────────────────┼─────────────────────────────────────────────────────────────┤
  │ 0x28 │ handle effect_id │ Enter effect handler for effect_id; dispatch table in reg_a │
  ├──────┼──────────────────┼─────────────────────────────────────────────────────────────┤
  │ 0x29 │ resume r_a       │ Resume handler; continuation in r_a                         │
  └──────┴──────────────────┴─────────────────────────────────────────────────────────────┘

## 3. Module Format (.ecm Binary)

```
  [HEADER (32 bytes)]
    [magic:4]              // 0xECFF00EC
    [version:2]            // 0x0100 (1.0 experimental)
    [flags:2]              // reserved
    [section_count:4]      // number of sections
    [code_offset:4]        // byte offset to code section
    [const_offset:4]       // byte offset to constants section
    [types_offset:4]       // byte offset to types section
    [functions_offset:4]   // byte offset to functions section
    [imports_offset:4]     // byte offset to imports section
    [exports_offset:4]     // byte offset to exports section
    [debug_offset:4]       // byte offset to debug section (0 if absent)

  [CONSTANTS SECTION]
    [count:4]
    [constant_0: 8 bytes each]
    [constant_1: 8 bytes each]
    ...

  [TYPES SECTION]
    [count:4]
    [type_0_id:4] [layout_size:4] [field_count:2] [destructor_fn:2] [fields...]
    [type_1_id:4] [layout_size:4] [field_count:2] [destructor_fn:2] [fields...]
    ...

  [FUNCTIONS SECTION]
    [count:4]
    [fn_0_id:4] [code_offset:4] [code_size:4] [stack_size:2] [param_count:2] [reg_count:2]
  [reserved:2]
    [fn_1_id:4] [code_offset:4] [code_size:4] [stack_size:2] [param_count:2] [reg_count:2]
  [reserved:2]
    ...

  [CODE SECTION]
    [fn_0_bytecode (variable length)]
    [fn_1_bytecode (variable length)]
    ...

  [IMPORTS SECTION]
    [count:4]
    [import_0_id:4] [name_offset:4] [name_length:2] [param_count:2]
    [import_1_id:4] [name_offset:4] [name_length:2] [param_count:2]
    ...
    [string_pool: variable length]

  [EXPORTS SECTION]
    [count:4]
    [export_0_fn_id:4] [name_offset:4] [name_length:2] [reserved:2]
    [export_1_fn_id:4] [name_offset:4] [name_length:2] [reserved:2]
    ...
    [string_pool: variable length]

  [DEBUG SECTION] (optional, omitted for production builds)
    [source_lines: variable length]
    [variable_names: variable length]
    [type_names: variable length]
```

All multi-byte integers are little-endian.

## 4. Register Model

  - Per-frame registers: 256 64-bit registers (r0–r255)
  - Calling convention: Parameters in r0–rN; return value in r0
  - Scalar representation: Integers and floats as native 64-bit values
  - Pointer representation: Memory addresses as 64-bit unsigned integers
  - Composite representation: Pointer to heap allocation (struct/array)
  - No type tag: The VM does not store or check type at runtime; the layout (from the types section) is consulted at alloc/free/field access

## 5. Type Erasure

The ECT compiler guarantees the following:

1. All type checking is static. The bytecode contains no type information beyond layout metadata.
2. Generics are monomorphized. Generic functions are instantiated for each concrete type at
compile time. The bytecode contains only concrete instances.
3. Ownership is explicit. The compiler emits move, copy, and drop instructions; the runtime trusts
 these instructions.
4. No runtime type casts. Pointer casts are forbidden unless explicitly unsafe (not in this
version). Type confusion is impossible.
5. Layout is invariant. A type's layout (field offsets, size, destructor) does not change between modules. The types section encodes the layout; VMs must respect it.

## 6. Ownership Operations

The compiler generates these instructions to enforce Rust-like ownership at runtime:

  ┌────────────────────────────────┬────────────────┬───────────────────────────────────────────┐
  │            Scenario            │  Instruction   │                 Semantics                 │
  ├────────────────────────────────┼────────────────┼───────────────────────────────────────────┤
  │ Copy value (no ownership       │ copy r_a, r_b  │ r_a := r_b; both values live              │
  │ transfer)                      │                │                                           │
  ├────────────────────────────────┼────────────────┼───────────────────────────────────────────┤
  │ Move value (transfer           │ move r_a, r_b  │ r_a := r_b; drop r_b (calls destructor)   │
  │ ownership)                     │                │                                           │
  ├────────────────────────────────┼────────────────┼───────────────────────────────────────────┤
  │ Create reference               │ ref r_a, r_b   │ r_a := &r_b (immutable borrow)            │
  ├────────────────────────────────┼────────────────┼───────────────────────────────────────────┤
  │ Destroy owned value            │ drop r_a       │ Call destructor for r_a (if needed)       │
  ├────────────────────────────────┼────────────────┼───────────────────────────────────────────┤
  │ Allocate and initialize        │ alloc r_a,     │ Allocate; initialize fields with type     │
  │                                │ size           │ defaults                                  │
  ├────────────────────────────────┼────────────────┼───────────────────────────────────────────┤
  │ Deallocate                     │ free r_a       │ Free heap at address r_a                  │
  └────────────────────────────────┴────────────────┴───────────────────────────────────────────┘

Destructor lookup is via the type table: for type T, consult T.destructor_fn to find the destructor code.

## 7. Effect Lowering

The compiler transforms exception types into Result-like values:

```rust
fn might_fail() -> <exn<Error>> Int
```

Compiles to:

```
// Bytecode: stores Result<Int, Error> in r_a
// On error path: set high bit of r_a, store error code/tag
// On success path: store Int in r_a
```

throw statements become conditional branches setting the error state. try-catch becomes pattern matching on the result.

### 7.2 Async (async)

The compiler transforms async functions into stackful coroutines:

```rust
  async fn fetch() -> <async> Response { ... }
```

Compiles to:

```
  // Bytecode: function body is a coroutine; suspend/resume via stack
  // spawn: allocate coroutine frame, add to scheduler queue
  // await: check if handle is ready; if not, suspend and resume caller
  // No callback or state machine; just control flow
```

The spawn instruction allocates a coroutine frame. The await instruction blocks until the coroutine completes. The VM's scheduler is cooperative: only await and yield are suspension points.

### 7.1 Exception Handling (exn<T>)

For debugging and manual inspection, bytecode can be represented as text:

```h
; module: example
; version: 1.0-experimental

.constants
  const_0: 0x0000000000000042  ; 66
  const_1: 0x3ff0000000000000  ; 1.0 (f64)

.types
  type_0: id=MyStruct size=24 fields=3
    field_0: offset=0 size=8
    field_1: offset=8 size=8
    field_2: offset=16 size=8

.functions
  fn_main:
    mov r0, r1              ; 0x1b 00 01 00
    add r0, r1, r2          ; 0x00 00 01 02
    jz r0, +0x10            ; 0x13 00 10 00
    call fn_foo, r0, r1     ; 0x15 [fn_id] [nargs] ...
    ret r0                  ; 0x16 00 00 00

.imports
  fn_print: id=1 params=1

.exports
  fn_main: id=0
```

The .ect.s format is not a requirement for conformance. It is an optional tool for manual inspection and testing. All VMs must consume .ecm binary.

## 9. Conformance Requirements

A compliant ECT VM must:

1. Load and parse .ecm modules correctly. Magic number, section offsets, all data types.
2. Execute all 40 instructions as specified. Behavior must match the semantics table exactly.
3. Maintain register and frame invariants. Each function frame has 256 registers; stack discipline
 is per-function.
4. Implement ownership semantics. move must call destructors; copy must not.
5. Respect type layouts. Field offsets from the types section must be honored.
6. Support cooperative scheduling. spawn queues coroutines; await and yield suspend at defined
points.
7. Trap on division by zero. Dividing by zero is a fatal error (trap).
8. Preserve memory safety. No out-of-bounds access, no use-after-free. (Assumes correct compiler
output.)
9. Export host functions correctly. Functions marked .exports are callable from the host language.
10. Import host functions correctly. .imports are resolved at load time; missing imports cause load failure.

## 10. Implementation Notes

### 10.1 Stack vs. Heap

- Stack: Registers, function frames, local variables. Managed by the VM's call stack.
- Heap: Large values, long-lived data, shared data. Managed by the VM's GC or reference counting.

The compiler emits alloc for heap allocation and free for deallocation. Reference counting (for `Shared<T>`) is handled by alloc/free pair with an RC header.

### 10.2 Garbage Collection

The bytecode does not mandate a GC algorithm. Implementations may use:
- Reference counting (thread-local or atomic)
- Tracing GC (stop-the-world or incremental)
- Arena allocators with bulk deallocation
- Manual allocation (unsafe, not recommended)

The only requirement: memory must be safe. Use-after-free and double-free are fatal errors.

### 10.3 Debugging Support

  The optional debug section encodes:
  - Source line mappings (bytecode offset → source location)
  - Variable names and types
  - Type definitions (for pretty-printing at breakpoints)

A debugger reads this section to provide human-friendly inspection.

## 11. Version and Stability

  This specification is version 1.0-experimental.

  Before 1.0-stable:
  - New instructions may be added (0x2a+).
  - Existing instruction semantics will not change.
  - The module format may gain optional sections.
  - Reserved fields allow for backward-compatible extension.

  After 1.0-stable:
  - The bytecode format is frozen. No breaking changes.
  - New features use version 1.1, 1.2, etc. in the header.
  - Older VMs must reject newer versions gracefully (bail-out, not crash).

## 12. Rationale

### 12.1 Why Register-Based?

Registers are simpler to reason about than a stack. Allocation and reuse are explicit. JIT and optimization are easier.

### 12.2 Why Type Erasure?

All type checking happens at compile time. The runtime is minimal and fast. Type information at runtime would add overhead with no safety benefit (the compiler already proved safety).

### 12.3 Why Cooperative Scheduling?

No preemption means no locks, mutexes, or atomic operations in the scheduler. Safety is by construction, not by discipline. Coroutines are lighter weight than OS threads.

### 12.4 Why Explicit Ownership?

Move vs. copy vs. drop makes memory lifetime explicit. The VM can safely deallocate without GC pauses or conservative scanning.

### 12.5 Why a Stable Bytecode Format?

The compiler is allowed to evolve. VMs are allowed to optimize. But the bytecode must remain a stable contract. This enables:

- Multiple compiler implementations
- Multiple VM implementations
- Distribution of bytecode as a product (like .pyc or .class)
- Long-term compatibility

## 13. Example Bytecode

### 13.1 Source

```rust
fn add(x: Int, y: Int) -> Int {
  x + y
}

fn main() {
  add(2, 3)
}
```

### 13.2 Bytecode

```h
.constants
  const_0: 0x0000000000000002
  const_1: 0x0000000000000003

.functions
  fn_add:
    add r0, r0, r1          ; r0 := r0 + r1
    ret r0

  fn_main:
    ldi r0, 2               ; r0 := 2
    ldi r1, 3               ; r1 := 3
    call fn_add, r0, r1     ; call add(2, 3); result in r0
    ret r0

.exports
  fn_main: id=0
```