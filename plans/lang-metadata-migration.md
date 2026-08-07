# `:tahto/*` to `:lang/*` language metadata migration

HARA-1 moved executable compiler and runtime namespaces from `tahto.*` to
`lang.*`. HARA-2 completes the name reservation by moving Hara-owned serialized
metadata and protocol identifiers out of the Tahto keyword space.

## Mapping

The keyword namespace root changes mechanically:

```text
:tahto/name          -> :lang/name
:tahto.hara/name     -> :lang.hara/name
:tahto.standard/name -> :lang.standard/name
:tahto.eval/name     -> :lang.eval/name
```

This includes compiler diagnostics, provenance, work attribution, grammar
coordinates, runtime type identifiers, evaluation plugin coordinates, target
errors, Hara lowering control records, and PostgreSQL literal-evaluator
capabilities.

String markers such as `$tahto`, historical upstream paths, and ordinary prose
are not keyword namespaces and are not changed by this migration.

## Authority boundary

After this PR:

- Hara writes only `:lang…/*` language metadata;
- `tahto.*` and `:tahto…/*` are available for the Greenways Tahto fabric;
- Hara does not define Greenways fabric records or application semantics; and
- arbitrary user forms are never recursively rewritten.

## One-release reader

`lang.common.compat` provides a bounded read adapter:

- canonical keys are checked first;
- legacy keys are constructed dynamically rather than written as literals;
- top-level metadata maps are canonicalized shallowly;
- canonical values win when both spellings exist;
- plugin coordinates canonicalize their keyword head explicitly; and
- runtime type descriptors canonicalize their `:type` protocol identifier.

The compatibility adapter is applied only at named metadata boundaries:

```text
compiler contexts and hydration metadata
provenance frames, stacks, and error data
emitter option maps
runtime type installation
evaluation plugin lookup
PostgreSQL literal-evaluator capabilities
```

Source forms and application values remain untouched even when they contain a
legacy-looking keyword.

## Migration fixtures

`core/spec/fixtures/lang-metadata-v1/` publishes:

- `legacy.edn`, accepted at compatibility reads for one release;
- `canonical.edn`, the only shape written after HARA-2; and
- fixture documentation defining precedence and non-recursive behavior.

The permanent guard permits literal legacy keywords only in `legacy.edn` and
its README.

## Generated source cut

`packaging/scripts/migrate-tahto-metadata-to-lang` performs the deterministic
source update:

1. rewrites Hara-owned keyword literals in canonical language sources, tests,
   integrations, Java sources, and specifications;
2. installs compatibility calls at the named read boundaries;
3. updates focused compatibility assertions;
4. regenerates `core/rust/hal-src` from canonical source roots;
5. runs source-layout, no-live-namespace, metadata-reservation, mirror, and
   whitespace checks; and
6. emits no forwarding `tahto.*` namespace or legacy keyword writer.

## Acceptance

- no active Hara source, test, Java source, integration, or Rust HAL mirror
  contains a literal `:tahto…/*` keyword;
- canonical compiler/runtime outputs use `:lang…/*`;
- the legacy fixture is accepted through the compatibility adapter;
- canonical keys override legacy keys when both are supplied;
- old plugin coordinates and runtime type descriptors resolve to canonical
  identifiers;
- source forms are not recursively rewritten;
- canonical-to-Rust HAL mirrors are synchronized; and
- focused JVM/Rust/HAL tests plus the relevant full suites remain non-vacuous.

The legacy reader is removed in the next compatibility release. The published
fixture remains as historical migration evidence.
