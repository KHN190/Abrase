# Appendix C: Device Catalog

This document extends the [bytecode spec](./appendix-bytecode-sped.md) and fixes the meaning of standard port for hosts. A host is a machine that can execute VM bytecode.

A module declares the device IDs it requires in its `.ecm` header. The loader rejects modules referring to devices the host does not provide.

## Conventions

- All values are 64-bit. Smaller logical types (bytes, u16, i32) sit in the low bits with the upper bits zeroed.
- "in" means readable via `dei`; "out" means writable via `deo`. A port may be either or both.
- `-1` is the standard "error / EOF / no value" sentinel where noted.
- Unmapped ports within an implemented device are reserved. Reads return 0; writes are silently dropped.

## 17.1 System (`0x00`)

| Port | Direction | Semantics |
|---|---|---|
| `0x00` | in | Spec version. Packed as `[major:16][minor:16][patch:32]`. |
| `0x01` | out | Halt. Low 32 bits are the process exit code. |
| `0x02` | out | Panic. Value is a pool index of a `String` message; host prints to stderr, halts with code 1. |
| `0x03` | in | Module load flags (bitmap of declared device IDs). |

The System device is mandatory: every conforming host implements it.

## 17.2 Console (`0x10`)

| Port | Direction | Semantics |
|---|---|---|
| `0x00` | in | Read one byte from stdin. `-1` on EOF. |
| `0x01` | out | Write low byte to stdout. |
| `0x02` | out | Write low byte to stderr. |
| `0x03` | out | Flush stdout and stderr. Value ignored. |

## 17.3 Screen (`0x20`)

A simple framebuffer device. Pixels are RGBA8888 packed into a u32 (low bits of the port value).

| Port | Direction | Semantics |
|---|---|---|
| `0x00` | in | Width in pixels. |
| `0x01` | in | Height in pixels. |
| `0x02` | out | Set cursor x. |
| `0x03` | out | Set cursor y. |
| `0x04` | out | Write RGBA pixel at (cursor_x, cursor_y); cursor advances by 1 in x, wrapping to next row. |
| `0x05` | out | Fill the screen with RGBA color. |
| `0x06` | out | Present the back buffer. Value ignored. |

## 17.4 Audio (`0x30`)

A single PCM output stream.

| Port | Direction | Semantics |
|---|---|---|
| `0x00` | in | Sample rate in Hz. |
| `0x01` | in | Channel count (1 = mono, 2 = stereo, …). |
| `0x02` | out | Write one signed 16-bit sample (low bits) per channel, in interleaved order. |
| `0x03` | out | Flush pending samples to the audio device. Value ignored. |

## 17.5 FileSystem (`0x40`)

Stateful per-handle device. A program selects a working handle on `0x03`; subsequent reads/writes operate on it.

| Port | Direction | Semantics |
|---|---|---|
| `0x00` | out | Pool index of path string for the next `open`. |
| `0x01` | out | Mode for the next `open`: `0` read, `1` write (truncate), `2` append. Triggers the open. |
| `0x02` | in | Result of last `open`: file handle (`≥ 0`) or `-1` on error. |
| `0x03` | out | Select an existing file handle for subsequent byte / close operations. |
| `0x04` | in | Read one byte from the selected handle. `-1` on EOF or error. |
| `0x04` | out | Write low byte to the selected handle. |
| `0x05` | out | Close the selected handle. Value ignored. |

## 17.6 Network (`0x50`)

Same shape as FileSystem: stateful handles, byte-stream I/O. Hosts may map this to TCP, Unix sockets, or simulation.

| Port | Direction | Semantics |
|---|---|---|
| `0x00` | out | Pool index of `"host:port"` string for the next `connect`. |
| `0x01` | out | Trigger a connect attempt. Value ignored. |
| `0x02` | in | Result of last connect: connection handle (`≥ 0`) or `-1`. |
| `0x03` | out | Select a connection handle for subsequent byte / close operations. |
| `0x04` | in | Receive one byte from the selected connection. `-1` on disconnect or error. |
| `0x04` | out | Send low byte on the selected connection. |
| `0x05` | out | Close the selected connection. Value ignored. |

## 17.7 Clock (`0x60`)

| Port | Direction | Semantics |
|---|---|---|
| `0x00` | in | Wall-clock milliseconds since Unix epoch. |
| `0x01` | in | Monotonic nanoseconds since some host-defined zero. |
| `0x02` | out | Sleep this many milliseconds before resuming. The host implements as a cooperative yield. |

## 17.8 RandomSource (`0x70`)

| Port | Direction | Semantics |
|---|---|---|
| `0x00` | in | One random byte (low 8 bits). |
| `0x01` | in | One random 64-bit word. |
| `0x02` | out | Seed the generator with the written value. Hosts using a non-deterministic source may ignore. |

## 17.9 Dispatch (`0xE0`)

The bridge between bytecode and the VM's handler stack. Used by compiler-generated dispatch thunks; user code never writes to these ports directly.

| Port | Direction | Semantics |
|---|---|---|
| `0x00` | out | Write `(effect_id << 8) \| op_id`. Asks the VM to resolve the topmost handler matching `effect_id` and look up `op_id` in its dispatch table. |
| `0x00` | in | After a `0x00` write, reading returns the arm function id (16-bit). If no matching handler is on the stack, the read returns `0xFFFF` and the host should trap; the typechecker rules this out in well-formed programs. |

The VM provides this device unconditionally; modules using effects implicitly require it.

Coroutines, schedulers, and other concurrency abstractions are built on top of this device by host-defined effect handlers — the bytecode itself has no concurrency primitives (no `spawn` / `join` / `yield`). A host that wants cooperative tasks declares a `scheduler` effect and implements its handler arms using the continuation-cell protocol of bytecode §3.9.

## 17.10 Reserved

Device IDs not listed above (`0x80`–`0xDF`, `0xE1`–`0xFF`) are not assigned. Hosts may use them for experimental devices, but modules using IDs in this range are non-portable. When a new standard device lands, it takes the lowest unassigned ID.
