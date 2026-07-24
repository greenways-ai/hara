# Polis Foundation `.hal` Port

This ledger tracks semantic decisions while porting `foundation-base/src/hara`
to the `polis.*` transpiler library.
An entry marked `pending` must not be implemented with changed behavior until it
has been reviewed.

## Slice status

| Slice | Status | Notes |
|---|---|---|
| Project and namespace loader | complete | Self-contained examples live under `lib/bench/<NUM>-<GROUP>` |
| Common grammar foundation | complete | Grammar specification, macros, xtalk profiles/system, and aggregate grammar have translated facts |
| `polis.typed` | pending | Does not include `hara.model` |
| Common preprocess and emit | in progress | `preprocess-base` is complete; `emit-rewrite` follows the typed slice |
| `polis.common.book*` | in progress | `book-entry` and `book-meta` are dependency-complete record slices |
| Remaining `polis.lang` | pending | Split by library, rewrite, compiler, and runtime layers |

## Source convention

Polis is a self-contained Hara project under `lib/`. Production
namespaces live under `lib/src/polis`; translated tests live under
`lib/test/polis`. Namespace roots are mapped predictably:

| Foundation | Polis |
|---|---|
| `hara.common.*` | `polis.common.*` |
| `hara.typed.*` | `polis.typed.*` |
| `hara.lang.book*` | `polis.common.book*` |
| transpiler-focused `hara.lang.*` | `polis.lang.*` |

Benchmarks consume these packaged namespaces and do not contain substitute
implementations.

## Pending semantic decisions

| Area | Original behavior | Missing or different facility | Proposed direction | Decision |
|---|---|---|---|---|
| Typed records | Five Clojure records represent typed declarations | Native Hara record declaration was missing | Native `defrecord`, `->Record`, and `map->Record` constructors | implemented |
| Namespace analysis | Uses `ns-publics`, `ns-aliases`, `ns-name`, and `requiring-resolve` | Hara exposed only part of this introspection | Narrow deterministic namespace primitives; publics exclude referred Vars | implemented |
| Source analysis | Uses tools.reader with file, line, and column metadata | Hara parser did not expose a multi-form analysis API | Capability-checked `read-forms` preserving file and source spans | implemented |
| Parallel work | Some later language/runtime paths use futures and parallel task processing | Hara promises differ from Clojure futures | No sequential fallback until separately approved | pending |
| Host integrations | Later language paths use filesystem, process, network, and JVM APIs | Hara uses explicit capabilities and runtime flavors | Keep operations unavailable until a capability-safe mapping is reviewed | pending |

## Intentional runtime adaptations

The following adaptations preserve observable Foundation behavior while using
the portable Hara runtime:

| Foundation construct | Polis implementation | Reason |
|---|---|---|
| Nested argument destructuring in helper definitions | Explicit `first`, `second`, `drop`, and indexed access | Hara does not yet support every nested Clojure binding pattern |
| `list` and syntax-quoted generated forms | Persistent lists assembled with `cons` | Hara does not expose Clojure's `list`, and syntax quote is not a runtime function |
| `with-meta` | `IObjType/with-meta` through `protocol-call` | Metadata mutation is intentionally protocol-based |
| Repeated traversal of lazy collection operations | Materialize once with `vec` | Hara lazy iterators are one-shot |
| Dynamic values consumed by lazy transforms | Materialize inside the dynamic binding | Deferred iteration must not escape the binding extent |
| Arbitrary `eval` in grammar mixins | Qualified symbol resolution plus a narrow form evaluator | Portable HAL code must not depend on host evaluation |
| Clojure `seq?` as a list/form predicate | Dispatch after the supported symbol and map mixin cases | Hara's `seq?` specifically identifies lazy `HaraSeq` values |
| Runtime `macroexpand-1` for `->` and `->>` data | Explicit thread-first/thread-last form transformation | Hara expands these forms during analysis; quoted transpiler data is transformed portably in HAL |
| Macros imported by an `ns` declaration | `:refer-macros` in the generated `:require` specification | Hara runtime `require` supported macro references, but source namespace declarations did not |
| Nested macro argument destructuring | Bind the vector as one macro argument and emit a runtime `first` | Hara macro parameters are symbols, while the public call shape remains unchanged |
| Clojure `integer?` in index-offset folding | Hara `number?` | Target grammar offsets are numeric indices and Hara currently exposes the broader numeric predicate |
| `std.lib.walk/prewalk` plus volatile accumulators | Explicit pending-node loop with persistent maps and sets | Keeps xtalk scanning portable and prevents mutable host values from escaping |
| Clojure set union, subset, and membership helpers | Small persistent reductions local to the xtalk system | Hara deliberately does not expose the whole `clojure.set` surface |
| Symbol-valued `code.test` `:id` metadata | Fact names and `:refer` metadata only | Unquoted metadata symbols are resolved by Hara; the test registry already has stable names |
| Foundation `Grammer` record | Persistent map tagged with `:polis/type :polis/grammar` | Hara records are structs rather than map values; downstream grammar consumers require keyword lookup and map semantics |
| Metadata-based operator category ordering | Explicit persistent `+op-categories+` and `+op-order+` vectors | Hara Vars do not provide the Foundation compiler's source-line ordering contract |
| Recursive lookup over lazy path tails | Materialize the remaining path in `get-in` and `assoc-in` | Hara lazy iterators are one-shot; probing them with `empty?` must not consume the next lookup key |
| Keyword lookup against sets | Hara keywords now query persistent and Java sets | Foundation argument parsing uses keyword lookup on grammar allow-lists and expects missing members to return `nil` |
| Metadata-bearing symbol equality | Symbol equality and hashing ignore metadata | Matches Clojure/Foundation value semantics and allows copied metadata assertions to compare unchanged |
| Macro parameter destructuring | Bind the vector parameter and select its first value inside `with:macro-opts` | Hara macro parameters currently support symbols and `&`, so the public call shape remains unchanged while the implementation avoids nested parameter destructuring |
| `std.lib.walk` | Lazy Java-backed static library over persistent Hara collections | Supplies the Foundation traversal contract without mutable host collections escaping; map/set implementations and metadata are preserved |
| `volatile!` used as a local traversal flag | Hara `atom`, `reset!`, and `deref` | `polis.common.preprocess-input` needs local mutable state only while checking whether a template expression can be evaluated |
| Host form `eval` during preprocessing | Core Hara `eval` evaluates the readable Hara form in the active namespace | Preserves persisted template behavior while keeping evaluation inside the Hara runtime rather than invoking Clojure/JVM evaluation |
| Foundation `std.lib.impl/defimpl` for `BookEntry` | Native Hara `defrecord` with the same fields and constructors | Preserves lookup, construction, and type checks without introducing an implementation-only compatibility namespace; custom Foundation record printing remains unavailable |
| `std.lib.template/$` in copied record-construction facts | Explicit persistent target forms assembled with `list` | Hara does not ship the Foundation template namespace; the test functions retain the same generated forms without adding a compatibility-only public namespace |

Translated `code.test` files are copied from Foundation by default. Namespace
ownership and unsupported dependency syntax may be translated, but fact
grouping, inputs, expected values, and metadata remain unchanged. A failing
copied fact is treated as a source or runtime gap before any test adaptation is
considered.

## Project convention

The nearest `project.hal` is discovered by walking upward from the working
directory. Runnable project fixtures use
`lib/bench/<NUM>-<GROUP>/{project.hal,src,test}`. A descriptor has this form:

```clojure
(defproject project.name
  {:source-paths ["src"]
   :test-paths ["test"]})
```

Namespace paths replace dots with directories and hyphens with underscores.
Project source wins over packaged classpath source. Test paths are considered
only for namespaces ending in `-test`.
