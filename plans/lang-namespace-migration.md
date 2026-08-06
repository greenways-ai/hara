# `tahto.*` to `lang.*` namespace migration

The portable language roots have already moved to `src-lang` and `test-lang`.
This migration performs the remaining compiler/framework namespace cut.

## Mapping

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

Physical source trees move in parallel:

```text
core/lib/src/tahto  -> core/lib/src/lang
core/lib/test/tahto -> core/lib/test/lang
core/rust/hal-src/tahto -> core/rust/hal-src/lang
```

## Hard-cut policy

- `lang.*` is the only compiler and language-authoring API after the change.
- No forwarding `tahto.*` namespaces are added.
- `xt.*` and `postgres.*` namespace names do not change.
- Serialized data vocabulary such as `:tahto/error-code`, `:tahto/phase`, and
  `:tahto/provenance` does not change in this structural migration.
- Historical Foundation paths such as `src/tahto/...` remain valid upstream
  references even though local target paths become `lib/src/lang/...`.

## Generated migration

`packaging/scripts/migrate-tahto-to-lang` performs the change atomically:

1. moves the production and test trees;
2. rewrites namespace declarations, requires, qualified Vars, quoted registry
   symbols, dynamic resolver targets, and generated-source expectations;
3. renames Hara-owned workflow and parity artifacts;
4. renames Hara Java tests whose class names contain `Tahto`;
5. regenerates `core/rust/hal-src` from the canonical source roots;
6. adds a permanent guard against live `tahto.*` code references;
7. runs source-layout, mirror, namespace, and whitespace checks.

The bootstrap workflow creates a dedicated generated branch and pull request,
then removes the one-shot workflow and migration script from that generated
branch. The final repository therefore contains only the resulting structure
and permanent validation guards.

## Acceptance

- `core/lib/src/tahto` and `core/lib/test/tahto` are absent.
- `core/lib/src/lang` and `core/lib/test/lang` contain the complete framework.
- No executable source, test, registry, or workflow contains a live
  `tahto.*` code namespace.
- No compatibility namespace duplicates the `lang.*` API.
- Public Vars and runtime/grammar coordinates remain otherwise unchanged.
- `xt.*` and `postgres.*` namespace inventories remain unchanged.
- JVM, Rust, raw WASM, browser WASM, HALC, and portable library suites are the
  required integration gates for the generated PR.
