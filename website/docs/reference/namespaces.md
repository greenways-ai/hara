# Namespace catalog

This catalog describes the public namespace families shipped by the current Truffle runtime. Use
Var metadata and the REPL for exact documentation and arglists.

## Core and intrinsic libraries

| Namespace | Default access | Purpose | Representative Vars |
| --- | --- | --- | --- |
| `std.lib.foundation` | Core names are referred eagerly | Portable L0 functions, collections, sequences, protocols, and macros | `map`, `reduce`, `take`, `comp2` |
| `std.lib.string` | `str/` | String comparison, slicing, joining, case conversion, trimming, and UTF-8 conversion | `trim`, `join`, `to-lower`, `encode` |
| `std.lib.bytes` | `bytes/` | Mutable byte buffers with explicit signed and unsigned conversion | `count`, `get`, `set`, `slice`, `u8` |
| `std.lib.promise` | `promise/` | Native asynchronous settlement and composition | `run`, `all`, `then`, `catch`, `finally` |
| `std.lib.file` | `file/` | Capability-gated path resolution and asynchronous byte I/O | `resolve`, `read`, `write` |
| `std.lib.socket` | `socket/` | Capability-gated callback socket operations | `connect`, `send`, `close` |
| `std.lib.block` | `block/` | Source-preserving Hara block parsing | `parse`, `parse-root`, `parse-first` |
| `std.lib.zip` | `zip/` | Persistent tree zipper navigation | `zipper`, `step-left`, `step-right`, `step-inside`, `step-outside` |
| `std.lib.walk` | explicit `require` | Persistent recursive traversal and key transformation | `walk`, `prewalk`, `postwalk`, `keywordize-keys` |

`(ns app)` and `(ns app (:intrinsics :all))` install the same default aliases. Intrinsic aliases
can be excluded or renamed without removing their underlying provider namespaces.

## Opt-in provider libraries

| Namespace | Load with | Purpose | Representative Vars |
| --- | --- | --- | --- |
| `std.lib.handle` | `(require 'std.lib.handle)` | Release opaque HTA extension handles | `release` |
| `std.lib.context` | `(require 'std.lib.context)` | Context registries, spaces, runtime lifecycles, and pointers | `space`, `pointer`, `pointer-deref`, `registry-list` |
| `std.lib.task` | `(require 'std.lib.task)` | Task definitions, invocation, selection, bulk processing, and summaries | `task`, `invoke`, `bulk`, `bulk-summary` |
| `std.lib.coroutine` | `(require 'std.lib.coroutine)` | Lua-style coroutines with bidirectional yield and promise await | `create`, `resume`, `yield`, `status`, `close`, `await` |
| `code.test` | `(require 'code.test)` | Facts, fixtures, matchers, registry inspection, and structured test execution | `fact`, `run`, `contains`, `throws`, `use-fixtures` |
| `std.resp.client` | explicit `require` | Capability-gated blocking RESP2 connections and pipelines | `connect`, `call`, `pipeline`, `close` |

These libraries are discovered through `HaraLibraryProvider` and installed lazily. Their Java or
HAL implementation details do not create additional public namespaces.

## Native JVM flavor

Portable namespaces do not expose ambient JVM interop. A namespace explicitly selects the JVM
flavor:

```clojure
(ns example.jvm
  (:flavor :jvm)
  (:import [java.lang String]))
```

The provider family is:

| Namespace | Purpose |
| --- | --- |
| `hara.native.jvm` | Native values, construction, member access, and invocation |
| `hara.native.jvm.reflect` | Capability-gated reflection operations |
| `hara.native.jvm.classpath` | Capability-gated classpath operations |
| `hara.native.jvm.compiler` | Capability-gated compilation and class definition |

Selecting a flavor is local to the declaring namespace and does not grant a capability or affect
required namespaces.

## Project namespaces

Project namespaces come from `.hal` files beneath source or test roots declared by the nearest
`project.hal`. Hara resolves `services.api-test` to `services/api_test.hal`. The file must declare
the requested namespace; mismatches fail instead of installing definitions under a surprising name.

## Extension namespaces

Extension manifests declare a namespace and export map. Requiring that symbol discovers the nearest
project extension root or a packaged extension, validates its descriptor, and generates public Vars
under the declared namespace:

```clojure
(ns digest.app
  (:require [crypto.hash.sha256 :as sha]))
```

The same Hara call site can bind to a supported WASM or process-backed provider. Manifest discovery,
provider availability, and capability authority are separate checks.

## Load behavior summary

| Family | Creation | File authority needed | Automatically visible |
| --- | --- | --- | --- |
| Foundation | Eager provider plus packaged HAL fallback | No | Unqualified core Vars |
| Intrinsic libraries | Generated providers | No for loading; operations may require authority | Qualified default aliases |
| Opt-in providers | Lazy provider registry | No for loading | Only after `require` |
| Project source | `.hal` under project roots | Yes | Only after `require` or direct evaluation |
| Extensions | Validated manifest and provider artifact | Depends on provider and project discovery | Only after `require` |
| Native flavor | Explicit `:flavor` clause | Depends on operation | Only in the declaring namespace |
