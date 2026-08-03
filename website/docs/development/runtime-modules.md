# Runtime modules

Hara separates compilation from production execution at the Rust crate boundary.

| Crate | Responsibility |
| --- | --- |
| `hara-abi` | Dependency-free package and host ABI values and identities |
| `hara-vm` | Verify, decode, prepare and execute HBC artifacts |
| `hara-compiler` | Compile source and HALC to HBC; optionally compile full WASM artifacts |
| `hara-wasm` | Compatibility facade and shared runtime implementation during extraction |

Production hosts that consume precompiled HBC should depend on `hara-vm`. Tooling,
REPLs and release builders depend on `hara-compiler`. The VM crate calls concrete
Rust execution functions directly, so the crate boundary introduces no per-opcode
dynamic dispatch, serialization, or FFI.

Publication-linked native packages implement `hara_abi::NativeModule`. Calls
use opaque task ids and poll/wait/cancel settlements, allowing a host to expose
them through the same promise-returning `std.native.Host/call` boundary used by
browser adapters. `LinkPlan::registration_source` emits deterministic module
installation statements alongside its Cargo dependencies.

The optional `rust/modules/std-db-postgres` package is the reference stateful
module. Its identity is `gh:hara-lang:std-db-postgres`, its service export is
`std.db.postgres`, and its ABI is `hara.db-provider/1`; it is not a dependency
of the base runtime or browser artifact.

The database ABI carries exact NUMERIC and ARRAY values as reserved portable
maps rather than provider objects. Normal queries use long/double and nested
vector conversion; callers can request tagged decoding per query when exact
decimal text, element types, dimensions or lower bounds are required.

Use link-time optimization for final release binaries when cross-crate inlining is
important. HTA remains a host/module boundary and is not used inside the VM loop.
