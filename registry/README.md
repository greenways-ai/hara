# hara extension registry

Status: **placeholder** — planned, not yet implemented.

## Intent

A registry of hara wasm extensions: versioned, discoverable bundles that
any hara runtime (Java/Truffle, Rust native, browser, Chrome extension)
can resolve and load via `(require [...])`.

Each extension is described by a `hara.extension.edn` manifest (see
`specs/hara/extensions-contract.md` for the contract) plus its wasm
artifact and optional host workers.

## Until the registry exists

Extensions incubate in-tree under
[`rust/extensions/`](../rust/extensions/) — e.g. `ledger-noir` and
`crypto-hash-sha256` — and reference packages live under
[`lib/examples/extensions/`](../lib/examples/extensions/). The registry
will eventually publish these (and third-party extensions) with
namespaced coordinates such as `hara/runtime/wasm`.
