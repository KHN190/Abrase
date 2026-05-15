# Appendix B: Bytecode Specification

This specification defines the bytecode structure. The VM only handles registers, control flow, memory, and device ports; it is entirely unaware of types or data structures. High-level semantics are lowered by the compiler.

## 1. Instruction Encoding

Every instruction is exactly 4 bytes, using one of three layouts:

```h
3-register form:    [op:8] [r_a:8] [r_b:8] [r_c:8]
reg + imm16:        [op:8] [r_a:8] [imm:16]
imm16 only:         [op:8] [pad:8] [imm:16]
```

Values exceeding 16 bits (64-bit ints, floats, strings) live in the module's constant pool (up to 65,536 entries, 64-bit each) and are loaded via pushconst.

## 2. Register Model

* Per-frame: 256 registers (r0–r255), 64 bits each.
* Frame stack: Every function call opens a new register window. The VM tracks the window base and return addresses internally.

## 3. Instruction Set

### 3.1 Arithmetic — 10 opcodes (`0x00`–`0x09`)

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

Integer overflow wraps. Float NaN/Inf are not trapped.

### 3.2 Comparison — 7 opcodes (`0x0a`–`0x10`)

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

### 3.3 Bitwise — 5 opcodes (`0x11`–`0x15`)

| Op | Mnemonic | Form | Semantics |
|---|---|---|---|
| `0x11` | `and r_a, r_b, r_c` | 3-reg | bitwise AND |
| `0x12` | `or r_a, r_b, r_c` | 3-reg | bitwise OR |
| `0x13` | `xor r_a, r_b, r_c` | 3-reg | bitwise XOR |
| `0x14` | `shl r_a, r_b, r_c` | 3-reg | logical left shift |
| `0x15` | `shr r_a, r_b, r_c` | 3-reg | arithmetic right shift (sign-extends) |

Shift count is taken modulo 64.

### 3.4 Control Flow — 5 opcodes (`0x16`–`0x1a`)

| Op | Mnemonic | Form | Semantics |
|---|---|---|---|
| `0x16` | `jmp +off` | imm16 only | `pc := pc + sext(off)` |
| `0x17` | `jz r_a, +off` | reg + imm16 | if `r_a == 0`, `pc := pc + sext(off)` |
| `0x18` | `jnz r_a, +off` | reg + imm16 | if `r_a ≠ 0`, `pc := pc + sext(off)` |
| `0x19` | `call r_a, fn_id` | reg + imm16 | open new frame for `fn_id`; on return, write result to caller's `r_a` |
| `0x1a` | `ret r_a` | 3-reg (`r_b`, `r_c` unused) | return value `r_a` to the caller's frame |

Jump offsets have each instruction 4 bytes, signed 16-bit. Functions that need farther jumps must be split. Functions are identified by a 16-bit id from the module's function table.

### 3.5 Data Movement — 3 opcodes (`0x1b`–`0x1d`)

| Op | Mnemonic | Form | Semantics |
|---|---|---|---|
| `0x1b` | `pushconst r_a, pool_idx` | reg + imm16 | `r_a := constants[pool_idx]` |
| `0x1c` | `copy r_a, r_b` | 3-reg | `r_a := r_b`; both registers remain live |
| `0x1d` | `move r_a, r_b` | 3-reg | `r_a := r_b`; `r_b` becomes empty (ownership transfer) |

### 3.6 Memory — 6 opcodes (`0x1e`–`0x23`)

| Op | Mnemonic | Form | Semantics |
|---|---|---|---|
| `0x1e` | `ld r_a, r_b, offset` | reg + imm16 | `r_a := heap[r_b + offset]` |
| `0x1f` | `st r_a, r_b, offset` | reg + imm16 | `heap[r_b + offset] := r_a` |
| `0x20` | `ldidx r_a, r_b, r_c` | 3-reg | `r_a := heap[r_b + r_c]` (dynamic offset) |
| `0x21` | `stidx r_a, r_b, r_c` | 3-reg | `heap[r_b + r_c] := r_a` (dynamic offset) |
| `0x22` | `lea r_a, r_b, offset` | reg + imm16 | `r_a := r_b + offset` (compute address, no load) |
| `0x23` | `ref r_a, r_b` | 3-reg | allocate a 1-slot heap object, store `r_b` in it, return pointer in `r_a` |

Offsets are in 64-bit slot units, not bytes.

### 3.7 Heap — 3 opcodes (`0x24`–`0x26`)

| Op | Mnemonic | Form | Semantics |
|---|---|---|---|
| `0x24` | `alloc r_a, size` | reg + imm16 | allocate `size` slots on the heap; pointer in `r_a` |
| `0x25` | `free r_a` | 3-reg (`r_b`, `r_c` unused) | free the heap object at `r_a` |
| `0x26` | `drop r_a` | 3-reg (`r_b`, `r_c` unused) | invoke the destructor function for `r_a`'s type (compiler-generated dispatch) |

`alloc` size is in 64-bit slots. The maximum single allocation is 65 535 slots ≈ 512 KB; larger objects must be chunked.

Reference counting is synthesized by the compiler and needs no dedicated opcode.

### 3.8 Host I/O — 2 opcodes (`0x27`–`0x28`)

| Op | Mnemonic | Form | Semantics |
|---|---|---|---|
| `0x27` | `dei r_a, r_b` | 3-reg (`r_c` unused) | `r_a := device_read(port = low16(r_b))` |
| `0x28` | `deo r_a, r_b` | 3-reg (`r_c` unused) | `device_write(port = low16(r_b), value = r_a)` |

A **port** is a 16-bit address defined in _Appendix C: Device Catalog_.

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
| `0xE0` | Dispatch | handler-stack lookup for generic effect-op call sites |
| `0x80`–`0xDF`, `0xE1`–`0xFF` | Reserved / experimental | — |

A module declares the device IDs it requires in its header (§6). The loader rejects modules whose devices the host does not provide.

Host-defined functions are device ports: write arguments to argument ports, write a command index to a trigger port, read the result port. The compiler hides this protocol behind language-level call syntax.

### 3.9 Effect Handlers — 2 opcodes (`0x29`–`0x2a`)

| Op | Mnemonic | Form | Semantics |
|---|---|---|---|
| `0x29` | `handle r_a, effect_id` | reg + imm16 | push a handler frame for `effect_id` with dispatch table pointer in `r_a` |
| `0x2a` | `resume r_a` | 3-reg (`r_b`, `r_c` unused) | resume the implicit continuation cell with value in `r_a` |

* Handler Frame (VM): `{ effect_id, dispatch_table_ptr, saved_pc, saved_base }`
* Dispatch Table (Pool): Array of function IDs (return arm at index 0, followed by op arms).
* Continuation Cell (Heap): 4-slot object `[suspend_pc, suspend_base, dest_reg, alive]`. Created by compiler thunks using device `0xE0`, passed to the arm function, and kept alive for multi-shot resumes.

### 3.10 Opcode Summary

Total: **37 opcodes** (`0x00`–`0x2a`). Slots `0x2b`–`0xFF` are reserved for compatible extension.

## 4. Calling Convention

When `call r_a, fn_id` executes:

1. The VM looks up `fn_id` in the module's function table to find `(reg_count, param_count, code_offset)`.
2. A new register window opens at `base += previous_reg_count`.
3. Caller must arrange arguments in the new window's `r0–r(param_count - 1)`before calling.
4. On ret `r_x`, the VM writes `r_x` to the caller's target register and pops the frame.

## 5. Module Format (`.ecm`)

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

The `device_mask` is the contract between module and host. If a module sets bit `i` in the mask and the host does not implement device `i`, the load fails immediately.

## 7. Versioning

This is version **1.0-experimental**.

- New opcodes may be assigned in the reserved range (`0x2e`–`0xFF`).
- New devices may be added to the catalog.
- No existing instruction semantics change. No existing device port meaning changes. No existing field layout changes.

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
