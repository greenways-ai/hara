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

Use link-time optimization for final release binaries when cross-crate inlining is
important. HTA remains a host/module boundary and is not used inside the VM loop.
