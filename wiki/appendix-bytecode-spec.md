# Appendix B: Bytecode Specification

Bytecode never changes over the language iteration. We implement a host so it can run the dumped bytecode on different platforms, and we use a compiler to produce this product.

Since the design is only 40 instructions, thus the VM does not know about types or data structures, only primitive integers and basic operations. A VM does not check, it only executes. We keep it simple so it can be migrated to different platform implementations.

But the compiler thus needs 2 passes — one yields High Level Representation (HIR), which contains tags, types, data structure annotations, etc. Another lowers them to basic integers that the VM can directly execute.

* The VM knows: registers, control flow, memory, two ports.
* The compiler knows: types, layouts, devices, lowering.
* The host knows: actual I/O.

Below is the definition.

## 1. Design Principles

1. **Stable artifact.** Compilers evolve, host VMs evolve; the `.ecm` (ECT Module) binary format does not.
2. **Strict 4-byte encoding.** Every instruction is exactly 4 bytes: `[opcode:8][a:8][b:8][c:8]`. No trailers, no prefixes, no multi-word instructions. `pc += 4` advances one instruction, always.
3. **Type erasure at the boundary.** Registers hold 64-bit scalars and pointers. Composites live on the heap. The VM never inspects type at runtime.
4. **Compiler does the heavy lifting.** Generics → monomorphized. Effects → desugared. Ownership → explicit `copy`/`move`/`drop`. Closures → lambda-lifted. Large literals → const pool. The VM is a flat switch over ~40 opcodes.
5. **Host I/O is device I/O.** All host interaction flows through `dei` (device input) and `deo` (device output) on numbered ports. The standard device catalog (`appendix-device-catalog.md`) fixes port semantics. Hosts implement the devices they support; modules declare the devices they require; load-time validation does the matching.

## 2. Instruction Encoding

Every instruction is 4 bytes, laid out in one of three forms:

```h
3-register form:    [op:8] [r_a:8] [r_b:8] [r_c:8]
reg + imm16:        [op:8] [r_a:8] [imm:16  little-endian]
imm16 only:         [op:8] [pad:8] [imm:16  little-endian]
```

The opcode selects the form. There is no instruction-prefix byte, no length escape, no multi-word encoding. Register indices are unsigned 8-bit. Immediates are unsigned 16-bit unless the semantics column says otherwise (jump offsets are signed 16-bit).

Values that don't fit in 16 bits — full 64-bit integers, f64 literals, interned strings — go in the module's **constant pool** and are loaded with `pushconst r_a, pool_idx`. The pool holds up to 65 536 entries; each entry is 64 bits.

## 3. Register Model

* **Per-frame**: 256 registers (`r0`–`r255`), each 64 bits.
* **Frame stack**: each function call opens a new register window. The VM maintains a window stack and a return-address stack.
* **Value representation**:
  - Integers (i64) and floats (f64) live directly in registers.
  - Pointers (heap handles) live directly in registers as 64-bit values.
  - Composites (records, variants, arrays, strings) live on the heap; the register holds the pointer.
* **No runtime type information.** The VM does not tag, check, or dispatch on type. The compiler emits the correct ops; the VM trusts them.

## 4. Instruction Set

### 4.1 Arithmetic — 10 opcodes (`0x00`–`0x09`)

| Op | Mnemonic | Form | Semantics |
|---|---|---|---|
| `0x00` | `add r_a, r_b, r_c` | 3-reg | `r_a := r_b + r_c` (i64, wraps) |
| `0x01` | `sub r_a, r_b, r_c` | 3-reg | `r_a := r_b − r_c` (i64, wraps) |
| `0x02` | `mul r_a, r_b, r_c` | 3-reg | `r_a := r_b × r_c` (i64, wraps) |
| `0x03` | `div r_a, r_b, r_c` | 3-reg | `r_a := r_b / r_c` (i64; trap on `r_c == 0`) |
| `0x04` | `mod r_a, r_b, r_c` | 3-reg | `r_a := r_b % r_c` (i64; trap on `r_c == 0`) |
| `0x05` | `neg r_a, r_b` | 3-reg (`r_c` unused) | `r_a := −r_b` (i64) |
| `0x06` | `fadd r_a, r_b, r_c` | 3-reg | `r_a := r_b + r_c` (f64, IEEE 754) |
| `0x07` | `fsub r_a, r_b, r_c` | 3-reg | `r_a := r_b − r_c` (f64) |
| `0x08` | `fmul r_a, r_b, r_c` | 3-reg | `r_a := r_b × r_c` (f64) |
| `0x09` | `fdiv r_a, r_b, r_c` | 3-reg | `r_a := r_b / r_c` (f64; produces ±∞/NaN per IEEE 754) |

Integer overflow wraps (two's complement). Float NaN/Inf are not trapped.

### 4.2 Comparison — 7 opcodes (`0x0a`–`0x10`)

| Op | Mnemonic | Form | Semantics |
|---|---|---|---|
| `0x0a` | `eq r_a, r_b, r_c` | 3-reg | `r_a := (r_b == r_c) ? 1 : 0` |
| `0x0b` | `neq r_a, r_b, r_c` | 3-reg | `r_a := (r_b ≠ r_c) ? 1 : 0` |
| `0x0c` | `lt r_a, r_b, r_c` | 3-reg | signed i64 less-than |
| `0x0d` | `gt r_a, r_b, r_c` | 3-reg | signed i64 greater-than |
| `0x0e` | `lte r_a, r_b, r_c` | 3-reg | signed i64 less-or-equal |
| `0x0f` | `gte r_a, r_b, r_c` | 3-reg | signed i64 greater-or-equal |
| `0x10` | `flt r_a, r_b, r_c` | 3-reg | f64 less-than (false if either is NaN) |

Comparison always produces 0 or 1 in `r_a`.

### 4.3 Bitwise — 5 opcodes (`0x11`–`0x15`)

| Op | Mnemonic | Form | Semantics |
|---|---|---|---|
| `0x11` | `and r_a, r_b, r_c` | 3-reg | bitwise AND |
| `0x12` | `or r_a, r_b, r_c` | 3-reg | bitwise OR |
| `0x13` | `xor r_a, r_b, r_c` | 3-reg | bitwise XOR |
| `0x14` | `shl r_a, r_b, r_c` | 3-reg | logical left shift |
| `0x15` | `shr r_a, r_b, r_c` | 3-reg | arithmetic right shift (sign-extends) |

Shift count is taken modulo 64.

### 4.4 Control Flow — 5 opcodes (`0x16`–`0x1a`)

| Op | Mnemonic | Form | Semantics |
|---|---|---|---|
| `0x16` | `jmp +off` | imm16 only | `pc := pc + sext(off)` |
| `0x17` | `jz r_a, +off` | reg + imm16 | if `r_a == 0`, `pc := pc + sext(off)` |
| `0x18` | `jnz r_a, +off` | reg + imm16 | if `r_a ≠ 0`, `pc := pc + sext(off)` |
| `0x19` | `call r_a, fn_id` | reg + imm16 | open new frame for `fn_id`; on return, write result to caller's `r_a` |
| `0x1a` | `ret r_a` | 3-reg (`r_b`, `r_c` unused) | return value `r_a` to the caller's frame |

Jump offsets are in **instruction units** (each instruction is 4 bytes), signed 16-bit: ±32 768 instructions ≈ ±128 KB of code per function. Functions that need farther jumps must be split.

Functions are identified by a 16-bit id from the module's function table (up to 65 536 functions per module). Argument count is fixed by the function table; `call` does not encode it. See §5 for the calling convention.

### 4.5 Data Movement — 3 opcodes (`0x1b`–`0x1d`)

| Op | Mnemonic | Form | Semantics |
|---|---|---|---|
| `0x1b` | `pushconst r_a, pool_idx` | reg + imm16 | `r_a := constants[pool_idx]` |
| `0x1c` | `copy r_a, r_b` | 3-reg | `r_a := r_b`; both registers remain live |
| `0x1d` | `move r_a, r_b` | 3-reg | `r_a := r_b`; `r_b` becomes empty (ownership transfer) |

All literal values — integers larger than 16 bits, floats, interned strings — enter the program through `pushconst`. The compiler manages the pool.

### 4.6 Memory — 6 opcodes (`0x1e`–`0x23`)

| Op | Mnemonic | Form | Semantics |
|---|---|---|---|
| `0x1e` | `ld r_a, r_b, offset` | reg + imm16 | `r_a := heap[r_b + offset]` |
| `0x1f` | `st r_a, r_b, offset` | reg + imm16 | `heap[r_b + offset] := r_a` |
| `0x20` | `ldidx r_a, r_b, r_c` | 3-reg | `r_a := heap[r_b + r_c]` (dynamic offset) |
| `0x21` | `stidx r_a, r_b, r_c` | 3-reg | `heap[r_b + r_c] := r_a` (dynamic offset) |
| `0x22` | `lea r_a, r_b, offset` | reg + imm16 | `r_a := r_b + offset` (compute address, no load) |
| `0x23` | `ref r_a, r_b` | 3-reg | allocate a 1-slot heap object, store `r_b` in it, return pointer in `r_a` |

`ld` and `st` carry a 16-bit constant offset (record fields, fixed array slots). `ldidx` and `stidx` use a register offset (dynamic indexing, array access). `lea` computes an address without loading; the compiler uses it for nested-record fields and slice operations.

Offsets are in 64-bit slot units, not bytes.

### 4.7 Heap — 3 opcodes (`0x24`–`0x26`)

| Op | Mnemonic | Form | Semantics |
|---|---|---|---|
| `0x24` | `alloc r_a, size` | reg + imm16 | allocate `size` slots on the heap; pointer in `r_a` |
| `0x25` | `free r_a` | 3-reg (`r_b`, `r_c` unused) | free the heap object at `r_a` |
| `0x26` | `drop r_a` | 3-reg (`r_b`, `r_c` unused) | invoke the destructor function for `r_a`'s type (compiler-generated dispatch) |

`alloc` size is in 64-bit slots. The maximum single allocation is 65 535 slots ≈ 512 KB; larger objects must be chunked.

The VM does not garbage-collect. The compiler emits `drop` and `free` at scope boundaries. Reference counting (for `Shared<T>`) is synthesized by the compiler — allocate one extra slot for the rc cell, inline atomic-ish increments/decrements — and needs no dedicated opcode.

### 4.8 Host I/O — 2 opcodes (`0x27`–`0x28`)

| Op | Mnemonic | Form | Semantics |
|---|---|---|---|
| `0x27` | `dei r_a, r_b` | 3-reg (`r_c` unused) | `r_a := device_read(port = low16(r_b))` |
| `0x28` | `deo r_a, r_b` | 3-reg (`r_c` unused) | `device_write(port = low16(r_b), value = r_a)` |

A **port** is a 16-bit address: the high byte names the device (`0x00`–`0xFF`, 256 devices), the low byte names a port within the device (`0x00`–`0xFF`, 256 ports per device). The standard device catalog fixes the semantics of every standard port; the VM itself has no opinion about what any device does.

| Device ID | Device | Examples |
|---|---|---|
| `0x00` | System | exit, halt, version, panic |
| `0x10` | Console | byte in/out, stderr |
| `0x20` | Screen | framebuffer, dimensions |
| `0x30` | Audio | sample stream |
| `0x40` | FileSystem | open, read, write, close |
| `0x50` | Network | connect, send, recv |
| `0x60` | Clock | now, monotonic, sleep |
| `0x70` | RandomSource | entropy |
| `0x80`–`0xFF` | Reserved / experimental | — |

A module declares the device IDs it requires in its header (§6). The loader rejects modules whose devices the host does not provide; stub implementations are forbidden.

There are no `import` or `export` opcodes. Host-defined functions are device ports: write arguments to argument ports, write a command index to a trigger port, read the result port. The compiler hides this protocol behind language-level call syntax.

### 4.9 Async — 3 opcodes (`0x29`–`0x2b`)

| Op | Mnemonic | Form | Semantics |
|---|---|---|---|
| `0x29` | `spawn r_a, fn_id` | reg + imm16 | spawn a coroutine running `fn_id`; handle in `r_a` |
| `0x2a` | `await r_a` | 3-reg (`r_b`, `r_c` unused) | suspend until the coroutine handle in `r_a` completes |
| `0x2b` | `yield` | imm16 only (unused) | voluntarily yield to the scheduler |

The scheduler is cooperative. The only suspension points are `await` and `yield`. No preemption, no locks, no atomics.

### 4.10 Effect Handlers — 2 opcodes (`0x2c`–`0x2d`)

| Op | Mnemonic | Form | Semantics |
|---|---|---|---|
| `0x2c` | `handle r_a, effect_id` | reg + imm16 | enter an effect handler frame for `effect_id`; dispatch table pointer in `r_a` |
| `0x2d` | `resume r_a` | 3-reg (`r_b`, `r_c` unused) | resume a captured continuation in `r_a` |

These are the lowering targets for the language's effect system. Exceptions are lowered to a single effect (`exn`); async is its own effect; user-defined effects use `handle` with a custom dispatch table built at compile time.

### 4.11 Opcode Summary

Total: **40 opcodes** (`0x00`–`0x2d`). Slots `0x2e`–`0xFF` are reserved for compatible extension.

## 5. Calling Convention

When `call r_a, fn_id` executes:

1. The VM looks up `fn_id` in the module's function table to find `(reg_count, param_count, code_offset)`.
2. A new register window opens at `base += previous_reg_count`.
3. The callee sees its parameters in `r0`–`r(param_count − 1)` of the new window. **The caller is responsible for arranging arguments there before the `call` instruction.** Typically the compiler emits a sequence of `copy` instructions targeting the upcoming window.
4. When the callee executes `ret r_x`, the VM writes the value of `r_x` (in the callee's window) to `r_a` in the caller's window, and pops the frame.

There is no explicit "argument count" operand. Function arity is fixed by the function table.

## 6. Module Format (`.ecm`)

All multi-byte integers are little-endian.

```h
[HEADER, 40 bytes]
  magic:4              = 0xECFF00EC
  version:2            = 0x0100  (1.0)
  flags:2              reserved
  device_mask:32       bitmap of required device IDs (256 bits)
  const_offset:4       byte offset to constants section
  fn_table_offset:4    byte offset to function table
  code_offset:4        byte offset to code section
  debug_offset:4       byte offset to debug section (0 if absent)

[CONSTANTS SECTION]
  count:4
  constant_0:8         (one 64-bit word per entry)
  constant_1:8
  ...

[FUNCTION TABLE]
  count:4
  entry_0: fn_id:2  reg_count:1  param_count:1  code_offset:4  code_size:4
  entry_1: ...
  ...

[CODE SECTION]
  fn_0_bytecode  (4 × instruction_count bytes)
  fn_1_bytecode
  ...

[DEBUG SECTION]  (optional, may be stripped)
  source_lines:    pc → (file_id, line, col)
  symbol_names:    function and parameter names
  type_names:      for pretty-printing
```

The header carries no type metadata. Composite layouts are entirely a compile-time concern; the compiler emits `ld`/`st` with fixed offsets and `alloc` with fixed sizes, and that is sufficient.

The `device_mask` is the contract between module and host. If a module sets bit `i` in the mask and the host does not implement device `i`, the load fails immediately.

## 7. Versioning

This is version **1.0-experimental**.

While `1.x`:

- New opcodes may be assigned in the reserved range (`0x2e`–`0xFF`).
- New devices may be added to the catalog.
- New optional sections may be appended to the module format.
- **No existing instruction semantics change.** No existing device port meaning changes. No existing field layout changes.

A `2.0` would be the first version permitted to break compatibility, and would require a new magic number.

## 8. Example

Source:

```rust
fn add(x: Int, y: Int) -> Int { x + y }
fn main() -> Int { add(2, 3) }
```

Bytecode:

```rust
.constants
  pool[0] = 0x0000000000000002    ; 2
  pool[1] = 0x0000000000000003    ; 3

.functions
  fn_add (id=0, regs=2, params=2):
    add r0, r0, r1                ; 0x00 00 00 01
    ret r0                        ; 0x1a 00 00 00

  fn_main (id=1, regs=3, params=0):
    pushconst r0, 0               ; 0x1b 00 00 00     r0 := 2
    pushconst r1, 1               ; 0x1b 01 01 00     r1 := 3
    call r2, 0                    ; 0x19 02 00 00     r2 := add(r0, r1)
    ret r2                        ; 0x1a 02 00 00

.header
  device_mask: 0x00…              ; no devices required
```
