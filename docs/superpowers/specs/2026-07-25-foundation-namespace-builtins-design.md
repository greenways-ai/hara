# Foundation namespaces and host builtins — design

Date: 2026-07-25
Status: proposed

## Goal

Make the JVM Hara foundation boundary explicit and editable while keeping ordinary Hara code
independent of JVM implementation details.

`std.foundation` is the small, automatically referred language foundation. Most evaluation,
reader, regular-expression, collection, and sequence behavior is implemented in Hara. Runtime
facilities that cannot be expressed portably are declared explicitly with `:builtins`.

The separately named foundation libraries are:

| Namespace | Default alias |
| --- | --- |
| `std.foundation.string` | `str` |
| `std.foundation.bytes` | `bytes` |
| `std.foundation.promise` | `promise` |
| `std.foundation.coroutine` | `coroutine` |
| `std.foundation.file` | `file` |
| `std.foundation.os` | `os` |

Socket, block, zip, and other libraries remain ordinary `std.lib.*` namespaces. They are not
automatically aliased as part of the foundation boundary.

## Current JVM status

The current implementation predates this design:

- The core namespace is `std.lib.foundation`.
- Every namespace unconditionally refers all Vars from `hara.lang.intrinsic` and
  `std.lib.foundation`.
- Every namespace receives `str`, `promise`, `bytes`, `socket`, `file`, `block`, and `zip`
  aliases.
- `ns` accepts `:intrinsics`, `:require`, `:flavor`, and `:import`. It rejects `:config` and
  `:builtins`.
- `(:intrinsics :all)` is equivalent to omitting the clause. Its map form supports
  `:exclude` and plural `:aliases`.
- A `HaraLibraryProvider` installs an entire library. `HaraStaticLibrary` scans every
  `@HaraExport` method and immediately defines every exported Var.
- `@HaraExport` already provides the export name, function/value/macro kind, docstring,
  arglists, and intrinsic-macro marker.
- `std.lib.coroutine` exists but must be required explicitly.
- `std.lib.file` exposes `resolve`, `read`, and `write`. `read` and `write` return promises
  and all three operations require file authority.

The target contract below replaces these behaviors. The foundational `std.lib.*` names are
renamed rather than retained as compatibility aliases.

## The `ns` contract

Foundation configuration lives in one optional `:config` clause:

```clojure
(ns app
  (:config {:blank true
            :builtins [host-clock]
            :intrinsics {:exclude [string os]
                         :alias {file std-file}}})
  (:require [app.data :as data]))
```

`:require`, `:flavor`, and `:import` remain separate structural clauses. Standalone
`:builtins` and `:intrinsics` clauses are not valid.

### Configuration schema

The only supported keys are `:blank`, `:builtins`, and `:intrinsics`:

```clojure
{:blank boolean
 :builtins [unqualified-symbol ...]
 :intrinsics
 :all
 ;; or
 {:exclude [library-symbol ...]
  :alias {library-symbol alias-symbol ...}}}
```

When `:config` is absent, its effective value is:

```clojure
{:blank false
 :builtins []
 :intrinsics :all}
```

Unknown keys, malformed values, duplicate symbols, duplicate clauses, and alias collisions are
errors. The complete `ns` form is validated before namespace state is changed.

### Blank namespaces

`:blank true` suppresses the automatic referral of public `std.foundation` Vars. It does not:

- remove Hara special forms;
- suppress the aliases selected by `:intrinsics`;
- erase definitions already owned by a namespace when the namespace is revisited.

This is primarily the bootstrap mechanism for `std.foundation` itself and for deliberately
minimal namespaces. Hara does not need Clojure's `:refer-clojure` clause.

### Intrinsic aliases

`:intrinsics` controls only automatic aliases for the six separate foundation libraries. It
does not control host exports and it does not grant file, process, or other capabilities.

```clojure
(ns app
  (:config
   {:intrinsics {:exclude [bytes os]
                 :alias {string text
                         coroutine co
                         file std-file}}}))
```

The logical library keys are `string`, `bytes`, `promise`, `coroutine`, `file`, and `os`.
An exclusion removes only the automatic alias. The namespace can still be loaded explicitly
with `:require`.

The option is singular `:alias`, not `:aliases`.

### Host builtins

`:builtins` activates an exact set of host exports for the namespace being declared:

```clojure
(ns std.foundation.string
  (:config
   {:builtins [length upper lower trim split]}))
```

The rules are:

1. Every name is an unqualified symbol.
2. Every name must be registered by the host for the declaring namespace.
3. Only listed names are activated; `:all` is intentionally unsupported.
4. The resulting Var receives the export kind, docstring, and arglists from `@HaraExport`.
5. Missing exports, namespace mismatches, duplicate names, and conflicting bindings fail.
6. Repeating the identical activation is idempotent.
7. A builtin declaration cannot expose an arbitrary Java class or method.

Any namespace may use `:builtins` when a runtime provider has registered exports for that exact
namespace. Foundation namespaces use the same mechanism as application- or extension-owned
namespaces; there is no privileged hard-coded string-library path.

## Runtime loading model

Host export discovery and Var activation become separate operations.

A `HaraLibraryProvider` may register an annotated builtin implementation class for its namespace.
The loader catalogs its `@HaraExport` entries without defining them. Evaluating the namespace's
`:builtins` declaration activates only the selected entries.

Existing providers that are not part of this boundary may continue to install a complete
provider-backed library.

The bootstrap order is:

1. Install evaluator special forms and the irreducible runtime machinery needed to read and
   execute namespace forms.
2. Discover host export catalogs without publishing their entries.
3. Load `std.foundation` from Hara source or its HIR artifact.
4. Let its `:config` declaration activate the exact primitive boundary.
5. Refer the completed `std.foundation` into ordinary non-blank namespaces.
6. Load separate foundation libraries on demand and install their configured aliases.

Interpreted source, analyzed AST execution, required modules, and `FoundationHirLoader` must all
use the same `ns` configuration path.

## Foundation placement

Evaluation, reader, and portable regexp operations belong in `std.foundation`. This includes:

- `eval`;
- `read-string`, which reads exactly one form and rejects trailing forms;
- string-based `read-forms`, which performs no file I/O;
- portable regexp construction, predicates, matching, sequencing, groups, and replacement.

File loading is not a reader primitive. Module loading remains namespace infrastructure, while
general filesystem access belongs in `std.foundation.file`.

The detailed builtin and Hara-defined symbol inventory remains data in
`spec/hara/foundation.edn`. Public `iter-*` mechanics and `mapv` are not part of the language
surface. Direct `(map f source)` is eager and preserves the origin family; `(map f)` returns a
lazy iterator transform.

## File API

`std.foundation.file` provides the small portable filesystem boundary:

| Function | Result |
| --- | --- |
| `(resolve root path)` | Normalized path string |
| `(read path)` | Promise of bytes |
| `(write path bytes)` | Promise of `nil` |
| `(exists? path)` | Promise of boolean |
| `(list path)` | Promise of a vector of child paths |
| `(mkdir path)` | Promise of `nil` |
| `(delete path)` | Promise of `nil` |

`resolve` is a synchronous path calculation. Every operation that touches the filesystem is
asynchronous and file-capability gated.

- `write` overwrites or creates the target but does not create missing parent directories.
- `list` returns normalized child paths in deterministic sorted order.
- `mkdir` creates missing parents and succeeds when the directory already exists.
- `delete` removes a file or an empty directory. It is never recursive.

## OS and process API

`std.foundation.os` provides portable host identity:

| Function | Result |
| --- | --- |
| `(platform)` | `:linux`, `:macos`, `:windows`, or `:unknown` |
| `(arch)` | Normalized architecture keyword |
| `(cwd)` | Normalized current-working-directory string |
| `(env)` | String-to-string environment map |
| `(getenv name)` | String or `nil` |

Process names are deliberately prefixed. An unqualified-looking operation such as `os/write`
does not make it clear that the destination is a child process's standard input.

| Function | Semantics |
| --- | --- |
| `(spawn argv)` | Start a process and return an opaque process handle |
| `(spawn argv options)` | Start with `:cwd` and/or string-valued `:env` overrides |
| `(process? value)` | Test for the opaque process handle |
| `(process-alive? process)` | Report whether the child is running |
| `(process-write process bytes)` | Return a promise that writes bytes to standard input |
| `(process-close-input process)` | Return a promise that closes standard input |
| `(process-stdout process)` | Return a promise of all standard-output bytes at EOF |
| `(process-stderr process)` | Return a promise of all standard-error bytes at EOF |
| `(process-wait process)` | Return a promise of the integer exit code |
| `(process-kill process)` | Forcibly terminate a live process |

`argv` is always a non-empty vector of strings. It is passed directly to the host process API
and is never interpreted as a shell command.

Output and error are drained concurrently from process start so a child cannot deadlock on a full
pipe while the caller waits. Repeated calls to `process-wait`, `process-stdout`, and
`process-stderr` observe the same settled results. `process-kill` is idempotent for an exited
process.

Process creation and control require process authority (`--allow-process` on the JVM CLI).
Shell strings, inherited I/O, OS signals, incremental stream iterators, clipboard access,
notifications, URL helpers, and the legacy tmux conveniences are outside this boundary.

## Implementation status in the editable spec

`spec/hara/foundation.edn` should distinguish the normative boundary from implementation
status. Its status data should record:

- the current namespace and alias names;
- which `ns` configuration features exist;
- which exports are installed directly or through providers;
- current public symbols that are outside the target boundary;
- missing target symbols and semantic mismatches;
- JVM/Polis parity issues, including regexp and Unicode behavior.

An implementation is not conformant merely because `HaraContext` happens to define a symbol.
The EDN inventory and conformance cases are the authority.

## Acceptance cases

The namespace suite must cover:

- all `:config` defaults and validation errors;
- `:blank true` suppressing core referrals while preserving configured aliases;
- intrinsic exclusions, renames, explicit `:require`, and collision failures;
- exact builtin activation, metadata propagation, idempotency, and namespace isolation;
- transactional failure without partially modified namespace state;
- identical behavior through source evaluation, required modules, and Foundation HIR.

The runtime-library suite must cover:

- renamed `std.foundation.*` resolution and removal of old foundational names;
- file capability denial and the complete small filesystem contract;
- process capability denial, argv preservation, byte-oriented standard I/O, exit codes,
  repeated observations, and process termination;
- JVM/Polis conformance for every portable operation.
