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
- Serialized data vocabulary was deliberately deferred from this structural
  migration and is moved by HARA-2; see `plans/lang-metadata-migration.md`.
- Historical Foundation paths such as `src/tahto/...` remain valid upstream
  references even though local target paths become `lib/src/lang/...`.

## Generated migration

`packaging/scripts/migrate-tahto-to-lang` performed the change atomically:

1. moved the production and test trees;
2. rewrote namespace declarations, requires, qualified Vars, quoted registry
   symbols, dynamic resolver targets, and generated-source expectations;
3. renamed Hara-owned workflow and parity artifacts;
4. renamed Hara Java tests whose class names contain `Tahto`;
5. regenerated `core/rust/hal-src` from the canonical source roots;
6. added a permanent guard against live `tahto.*` code references;
7. ran source-layout, mirror, namespace, and whitespace checks.

The one-shot generator and its bootstrap/validation workflows are removed from
the generated branch. `lang-runtime.yml` replaces `tahto-runtime.yml` through a
connector-authored follow-up commit because the repository-scoped Actions token
cannot create or rename workflow files.

## Generated result

The migration branch contains three dependency-complete commits:

1. the generated compiler/runtime namespace cut;
2. the workflow rename and one-shot workflow removal;
3. this completion record, which also triggers the full pull-request checks.

Static generation evidence:

```text
88 packaged Rust HAL snapshots moved
380 text files rewritten
314 production namespaces validated
169 test namespaces validated
canonical-to-Rust mirror synchronized
no live tahto.* code namespace detected
git diff --check passed
```

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
