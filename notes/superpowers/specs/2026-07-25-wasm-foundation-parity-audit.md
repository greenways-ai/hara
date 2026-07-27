# WASM Foundation Parity Audit

**Scope:** `specs/hara/data/foundation.edn` vs the Rust/WASM runtimes (`rust/src/core.rs`, `rust/src/lib.rs`, `rust/raw/src/lib.rs`).
**Date:** 2026-07-25
**Status:** snapshot of `main` after the studio-kernels merge.

## Executive summary

The WASM runtimes implement the core special forms and a large subset of the
builtin/collection/iteration surface, but they lag the spec in four broad
areas:

1. **Spec-named builtins are missing or renamed.** Many predicates, state
   operations, string functions, and file operations exist under non-spec
   names or not at all.
2. **Two spec runtime libraries are absent:** `std.foundation.coroutine` and
   `std.foundation.os` have no guest-visible symbols.
3. **`std.foundation.file` is incomplete** and only works in the
   wasm-bindgen `Runtime`; the raw HTA runtime cannot install file providers.
4. **The namespace contract is partially implemented.** Only `:intrinsics`
   (top-level) and `:require` work; `:config`, `:builtins`, and the
   six-library intrinsic set are not spec-conformant.

The raw HTA runtime additionally does **not** bootstrap
`lib/src/std/lib/foundation.hal`, so all foundation symbols (e.g. `first`,
`rest`, `partition-by`, `sort`, `distinct`) are unavailable there. This is the
most urgent practical gap because the studio kernels run on the raw HTA
surface.

## Methodology

- Read `specs/hara/data/foundation.edn` as the authority.
- Cross-referenced each boundary item against:
  - special-form dispatch in `rust/src/core.rs`
  - builtin/object/array method implementations in `rust/src/core.rs`
  - intrinsic library tables in `rust/src/kernel/generated.rs`
  - `lib/src/std/lib/foundation.hal` for foundation macros/symbols
  - `rust/src/lib.rs` (wasm-bindgen Runtime) and `rust/raw/src/lib.rs`
    (raw HTA runtime) for bootstrap and capability wiring

## Detailed findings

### 1. Special forms (spec lines 56-66)

All ten special forms are implemented in `core.rs`:

| Form | Location | Status |
|---|---|---|
| `quote` | `core.rs:4931` | ✅ |
| `if` | `core.rs:6807` | ✅ |
| `do` | `core.rs:5306` | ✅ |
| `let` | `core.rs:6819` | ✅ |
| `loop` | `core.rs:6757` | ✅ |
| `recur` | `core.rs:6699` | ✅ |
| `fn` | `core.rs:4978` | ✅ |
| `def` | `core.rs:5158` | ✅ |
| `defn` | `core.rs:5259` | ✅ |
| `ns` | `core.rs:5330` / `core.rs:4625` | ✅ |

### 2. Macros (spec lines 68-77)

| Macro | Spec | Foundation | Status |
|---|---|---|---|---|
| `->` | required | missing | ❌ |
| `->>` | required | missing | ❌ |
| `if-let` | required | `foundation.hal:824` | ✅ |
| `when-let` | required | `foundation.hal:833` | ✅ |
| `if-not` | required | `foundation.hal:819` | ✅ |
| `cond->` | required | `foundation.hal:841` | ✅ |
| `cond->>` | required | `foundation.hal:847` | ✅ |
| `code-line` | required | `foundation.hal:853` | ✅ |
| `code-column` | required | `foundation.hal:857` | ✅ |

Also missing from `foundation.hal`: `when`, `and`, `or`.

### 3. Reader literals (spec lines 79-82)

`#"..."` is parsed at `rust/src/kernel/parser.rs:176`, but it is turned into
a plain string value (`Value::Regex(String)` at `core.rs:4920`). It is not a
structured `:hara-regex` value carrying source, flags, and dialect as the spec
requires.

### 4. Builtin symbols

#### Numeric

| Symbol | Status | Notes |
|---|---|---|
| `+ - * /` | ✅ | `core.rs:2164` |
| `mod` | ✅ | aliased to `%` |
| `=` | ✅ | `core.rs:5316` |
| `not=` | ❌ | — |
| `< <= > >=` | ✅ | `core.rs:2252` |
| `compare` | ✅ | `core.rs:4749` |

#### Invocation

| Symbol | Status |
|---|---|
| `apply` | ✅ `core.rs:6426` |
| `gensym` | ✅ `core.rs:4790` |

#### State

| Symbol | Status | Notes |
|---|---|---|
| `atom` | ✅ | `core.rs:4813` |
| `atom?` | ❌ | — |
| `deref` | ✅ | `core.rs:5033` |
| `reset!` | ✅ | `core.rs:4813` |
| `swap!` | ✅ | `core.rs:4882` |
| `compare-and-set!` | ⚠️ | implemented as `compare:set!` |
| `add-watch` | ⚠️ | implemented as `watch:add` |
| `remove-watch` | ⚠️ | implemented as `watch:remove` |
| `get-watches` | ⚠️ | implemented as `watch:list` |

Renaming breaks spec-conformant code and should be reconciled.

#### Predicates

| Symbol | Status |
|---|---|
| `nil? true? false?` | ✅ |
| `boolean?` | ❌ |
| `number? integer? decimal?` | ❌ |
| `string? char? keyword? symbol?` | ❌ |
| `collection? sequential?` | ❌ |
| `list? vector?` | ✅ |
| `tuple? queue?` | ❌ |
| `map? set?` | ✅ |
| `map-entry?` | ❌ |
| `array? object? bytes? promise? coroutine? atom?` | ❌ |
| `callable? iterable? iterator? counted? indexed? associative? derefable? watchable?` | ❌ |

#### Evaluation

| Symbol | Status | Notes |
|---|---|---|
| `eval` | ✅ | `core.rs:4994` |
| `read-string` | ❌ | parser exists but symbol not bound |
| `read-forms` | ❌ | parser exists but symbol not bound |

#### Regex

All regex builtins (`re-pattern`, `regex?`, `re-find`, `re-matches`,
`re-seq`, `re-groups`) are missing. The reader value is just a string.

#### Collections

| Symbol | Status | Notes |
|---|---|---|
| `count` | ✅ | `core.rs:6574` |
| `get` | ✅ | `core.rs:6580` |
| `find` | ❌ | only the `IFind` protocol exists |
| `has?` | ❌ | only the `IFind` protocol exists |
| `assoc` | ✅ | `core.rs:6599` |
| `dissoc` | ✅ | `core.rs:6611` |
| `conj` | ✅ | `core.rs:6668` |
| `cons` | ✅ | `core.rs:6676` |
| `nth` | ✅ | `core.rs:6593` |
| `empty` | ✅ | `core.rs:6541` |
| `vec` | ❌ | — |
| `list` | ✅ | `core.rs:5566` |

#### Iteration

| Symbol | Status |
|---|---|
| `iter map filter take drop` | ✅ |
| `mapcat keep` | ✅ |
| `cycle zip partition-pair partition partition-all` | ✅ |
| `interpose interleave take-while drop-while` | ✅ |
| `range repeat repeatedly iterate concat` | ✅ |
| `reduce` | ❌ |
| `every? any?` | ✅ |

`reduce` is the most significant iteration gap; `sort`, `sort-by`, `into`,
and `distinct` in `foundation.hal` depend on it.

### 5. Runtime values

| Value | Status |
|---|---|
| `array` constructor | ✅ `core.rs:5593` |
| array methods (`get set push-first push-last pop-first pop-last insert remove clone slice map filter fold-left fold-right`) | ✅ `core.rs:3023` |
| `object` constructor | ✅ `core.rs:5600` |
| object methods (`has? get set delete clone assign keys vals pairs`) | ✅ `core.rs:3170` |

### 6. Runtime libraries

The intrinsic alias table at `rust/src/kernel/generated.rs:5` installs
`string promise bytes socket file`. The spec expects `string bytes promise
coroutine file os`.

#### `std.foundation.string` / `str`

| Spec symbol | Status | Notes |
|---|---|---|
| `length` | ❌ | exists as `str/count` |
| `blank? includes?` | ❌ | — |
| `starts-with? ends-with?` | ✅ | `core.rs:2861` |
| `char-at` | ⚠️ | exists as `str/char` |
| `slice` | ⚠️ | exists as `str/substring` |
| `index-of` | ✅ | `core.rs:2923` |
| `last-index-of` | ❌ | — |
| `join split` | ✅ | `core.rs:2909`, `core.rs:2901` |
| `split-lines` | ❌ | — |
| `repeat replace-first` | ❌ | `replace` exists |
| `trim trim-left trim-right` | ✅ | `core.rs:2983` |
| `upper lower` | ✅ | `core.rs:5696` |
| `capitalize decapitalize` | ❌ | — |
| `pad-left pad-right` | ✅ | `core.rs:2869` |
| `reverse` | ❌ | — |
| `encode-utf8 decode-utf8` | ⚠️ | exists as `str/encode`, `str/decode` |

#### `std.foundation.bytes` / `bytes`

All spec symbols (`count get set copy slice u8 s8`) are present at
`core.rs:5724`.

#### `std.foundation.promise` / `promise`

All spec symbols are present at `core.rs:5395` (`then`/`catch` mapped to
`promise/map`/`promise/recover`).

#### `std.foundation.coroutine` / `coroutine`

All spec symbols are missing. Internal fiber support exists in
`rust/src/fiber.rs` but is not exposed to guest code.

#### `std.foundation.file` / `file`

| Symbol | Status | Notes |
|---|---|---|
| `resolve` | ✅ | `core.rs:1449` |
| `read write` | ✅ | `core.rs:1466` |
| `exists? list mkdir delete` | ❌ | — |

File capability wiring:
- **wasm-bindgen `Runtime`**: `install_memory_file_provider` /
  `install_native_file_provider` work (`rust/src/lib.rs:311`).
- **raw HTA runtime**: no provider installed; `file/*` returns
  `file/unsupported`.

#### `std.foundation.os` / `os`

All spec symbols are missing. No guest-visible `os/` namespace exists.

### 7. Foundation symbols

`lib/src/std/lib/foundation.hal` defines many spec symbols, but the raw HTA
runtime does **not** load it, so they are absent in the studio/kernel path.

Even in the wasm-bindgen `Runtime` (which does load it), several symbols are
broken because they depend on missing builtins:

| Symbol | Status | Why |
|---|---|---|
| `sort sort-by into` | ❌ broken | need `reduce` |
| `distinct` | ❌ broken | needs `has?` |
| `reduce` | ❌ missing | not in core |
| `vec` | ❌ missing | not in core |
| `when and or -> ->>` | ❌ missing | macros not defined |
| atoms under spec names | ❌ broken | renamed to `watch:add` etc. |
| lazy seqs | ⚠️ risky | can hang the HTA encoder when returned from `map`/`filter`/`concat` |

Known operational issues from the studio work:
- `conj` on vectors hangs the HTA encoder.
- `str/split` hangs.
- `let` accepts only one body form.
- `require` vectors must be unquoted in the raw runtime.

### 8. Excluded `iter-*` symbols (spec lines 530-555)

The spec intends `iter-*` names to be internal. They are implemented as
internal helpers in `core.rs:3344`, but `eval` also exposes them publicly as
`iter-map`, `iter-filter`, etc. (`core.rs:5844`). This is a spec deviation.

### 9. Namespace contract (spec lines 9-53)

| Clause | Status | Notes |
|---|---|---|
| `:config` map | ❌ | not parsed |
| `:builtins` clause | ❌ | not parsed |
| `:intrinsics` clause | ⚠️ | parsed only as top-level `:intrinsics {...}` |
| `:require` | ✅ | `:as`, `:refer` supported |
| `:flavor` / `:import` | ⚠️ | recognized but ignored |
| automatic intrinsic libraries | ⚠️ | installs `string promise bytes socket file`; spec wants `string bytes promise coroutine file os` |

## Priority recommendations

### Critical (blocks spec-conformant code on raw HTA / studio kernels)

1. **Bootstrap `lib/src/std/lib/foundation.hal` in the raw HTA runtime.**
   Without this, studio kernels lack `first`, `rest`, `partition-by`, `sort`,
   `distinct`, etc. This is the single biggest practical gap.
2. **Add `reduce` to `core.rs`.** It unblocks `sort`, `sort-by`, `into`,
   `distinct`, and user reductions.
3. **Add `vec` and spec-named predicates (`string?`, `keyword?`, `symbol?`,
   `number?`, `boolean?`, `array?`, `object?`, `bytes?`, `promise?`,
   `atom?`, `callable?`, `iterable?`, etc.).** Foundation code relies on
   many of these.

### Important

4. Reconcile state operation names: `compare-and-set!`, `add-watch`,
   `remove-watch`, `get-watches`.
5. Implement `read-string` and `read-forms` builtins (parser already exists).
6. Add missing string functions: `length`, `blank?`, `includes?`,
   `last-index-of`, `split-lines`, `repeat`, `replace-first`, `reverse`,
   `capitalize`, `decapitalize`.
7. Complete `std.foundation.file` (`exists?`, `list`, `mkdir`, `delete`) and
   wire it in the raw runtime (via `host/call` or a provider bridge).
8. Fix the HTA encoder so lazy seqs and `conj`-on-vector do not hang.

### Low / follow-up

9. Add `std.foundation.coroutine` and `std.foundation.os` runtime libraries.
10. Implement structured regex values and regex builtins.
11. Remove or hide public `iter-*` symbols.
12. Implement the full `:config`/`:builtins` namespace contract.
13. Add `->`, `->>`, `when`, `and`, `or` macros.

## Update: explicit builtin declarations

`lib/src/std/foundation.hal` now declares the builtins it imports:

```clojure
(ns std.foundation
  (:config
   {:builtins
    [+ - < <= = > >= apply array? assoc compare concat conj cons count
     empty gensym get has? iter list list? map? mod not= nth reduce set?
     vec vector?]}))
```

The Rust namespace parser (`rust/src/kernel/generated.rs`) now accepts the
`:config` clause with `:blank`, `:builtins`, and nested `:intrinsics` options,
matching the Java parser. This lets hosts read a namespace's exact builtin
requirements before loading it.

**Path note:** the tracked source is currently at `lib/src/std/foundation.hal`
(namespace `std.foundation`) while the Java HIR compile step still looks for
`lib/src/std/lib/foundation.hal` (namespace `std.lib.foundation`). This is a
pre-existing namespace/path mismatch; the `:config` change does not resolve it.

## Conclusion

The WASM runtimes are closer to the spec than they first appear — most
special forms, collection primitives, iteration, and array/object support are
in place. The raw HTA path is held back by two things: it does not bootstrap
the foundation HAL, and a handful of missing/broken builtins (`reduce`,
spec-named state fns, predicates) break the HAL that is loaded. Fixing those
foundation-level items first will give the biggest parity improvement for the
studio and any other raw-wasm consumer.
