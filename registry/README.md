# Hara package registry

Status: **bootstrap** — the normative contract is drafted and the public
[`hara-lang/hara-packages`](https://github.com/hara-lang/hara-packages)
repository now hosts the reviewed registry. Remote resolution and publication
remain under implementation.

## Intent

The reviewed Git registry records package ownership and immutable releases for
Hara runtimes. Archives are GitHub Release assets on their source repositories;
clients require publisher intent and registry-CI Ed25519 attestations before
mounting their read-only package roots.

The package system also carries extension packages described by
`hara.extension.edn`, including WASM artifacts and optional host workers.

## Current implementation

The initial Rust implementation exposes deterministic local commands:

```text
hara package check
hara package build
hara package inspect
```

Remote resolution and publication require the registry and identity clients.
Extensions continue to incubate in-tree under
[`rust/extensions/`](../rust/extensions/) — e.g. `ledger-noir` and
`crypto-hash-sha256` — and reference packages live under
[`lib/examples/extensions/`](../lib/examples/extensions/). The registry
will eventually publish these (and third-party extensions) with
namespaced coordinates such as `hara/runtime/wasm`.
