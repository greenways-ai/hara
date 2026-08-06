# `tahto.*` to `lang.*` namespace migration

This document defines the second stage after the portable source-root split.

## Scope

Rename the language-authoring framework mechanically and atomically:

```text
tahto.base.*     -> lang.base.*
tahto.common.*   -> lang.common.*
tahto.core       -> lang.core
tahto.core.*     -> lang.core.*
tahto.model.*    -> lang.model.*
tahto.protocol.* -> lang.protocol.*
tahto.runtime.*  -> lang.runtime.*
tahto.typed.*    -> lang.typed.*
```

Move the corresponding trees:

```text
core/lib/src/tahto  -> core/lib/src/lang
core/lib/test/tahto -> core/lib/test/lang
```

The generated Rust snapshot follows the same namespace-shaped move:

```text
core/rust/hal-src/tahto -> core/rust/hal-src/lang
```

## Non-goals

- Do not rename `xt.*` or `postgres.*` namespaces.
- Do not add forwarding `tahto.*` namespaces to core.
- Do not rename serialized data vocabulary such as `:tahto/error-code`,
  `:tahto/phase`, or `:tahto/provenance` in the structural change.
- Do not rewrite historical Foundation source paths such as
  `foundation-base/src/tahto/...`.

## Required rewrites

The namespace cut must update more than `(ns ...)` declarations:

- requires, uses, imports, and aliases;
- qualified Vars and macros;
- quoted namespace and Var symbols;
- dynamic `requiring-resolve` and registry symbols;
- generated-source expectations;
- Java and Rust test paths;
- parity manifests and Hara-owned workflow names;
- embedded resource namespace indexes;
- documentation examples and links.

## Migration order

1. Record a before-manifest of every `tahto.*` namespace and public Var.
2. Move the production and test trees with Git rename detection preserved.
3. Rewrite code namespace references repository-wide.
4. Regenerate `core/rust/hal-src` from the canonical roots.
5. Rename Hara-owned operational artifacts such as `tahto-runtime.yml` and
   `tahto-*-parity.edn`.
6. Assert that the public manifest differs only by the `tahto` to `lang`
   namespace prefix.
7. Reject all remaining live `tahto.*` code references, allowing only pinned
   historical source paths and migration documentation.

## Acceptance gates

- `core/lib/src/tahto` and `core/lib/test/tahto` do not exist.
- `core/lib/src/lang` and `core/lib/test/lang` contain the complete moved trees.
- No production or test file declares a `tahto.*` namespace.
- No registry, quoted symbol, or dynamic resolver points at `tahto.*`.
- No compatibility namespace duplicates the `lang.*` API.
- JVM, Rust, raw WASM, browser WASM, HALC, and portable language tests pass.
- The `xt.*` and `postgres.*` namespace inventories are unchanged.
