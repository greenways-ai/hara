# Hara runtime libraries

Hara keeps its automatically referred core small. The runtime generates the library namespaces;
they are not backed by `.hal` source files and do not expose their JVM implementation.

Every namespace receives these aliases by default:

| Alias | Generated namespace | Xtalk family |
| --- | --- | --- |
| `str/` | `std.lib.string` | `x:str-*` |
| `promise/` | `std.lib.promise` | `x:promise-*`, `x:with-delay` |
| `bytes/` | `std.lib.bytes` | byte operations |
| `socket/` | `std.lib.socket` | `x:socket-*` |
| `file/` | `std.lib.file` | `x:file-*` |
| `block/` | `std.lib.block` | source-preserving Hara blocks |
| `zip/` | `std.lib.zip` | persistent tree zipper navigation |

`(ns app)` and `(ns app (:intrinsics :all))` are equivalent. Aliases can be excluded or renamed:

```clojure
(ns app
  (:intrinsics
    {:exclude [bytes]
     :aliases {string text
               promise async}}))
```

Exclusion only removes the qualified alias. It does not remove core constructors such as `bytes`.
Generated namespaces also work with ordinary namespace clauses:

```clojure
(ns app
  (:intrinsics {:exclude [string]})
  (:require [std.lib.string :as text :refer [trim]]))
```

The core constructors are `str`, `promise`, `bytes`, `array`, and `object`. Core also provides
signed 32-bit `bit-and`, `bit-or`, `bit-xor`, `bit-not`, `bit-shift-left`, and
`bit-shift-right`.

`array` and `object` create mutable marker values distinct from persistent `[]` and `{}`. Dot
calls are accepted only on those marker values:

```clojure
(. (array 1 2 3) (filter (fn [x] (> x 1))))
(. (object "name" "Hara") (get "name"))
```

Array methods are `get`, `set`, `push-first`, `push-last`, `pop-first`, `pop-last`, `insert`,
`remove`, `slice`, `clone`, `map`, `filter`, `fold-left`, and `fold-right`. Object methods are
`has?`, `get`, `set`, `delete`, `clone`, `assign`, `keys`, `vals`, and `pairs`. Object keys are
strings.

Promises are backed by the runtime's native promise mechanism. On the JVM that is
`CompletableFuture`. `promise/run`, `promise/new`, `promise/all`, `promise/then`,
`promise/catch`, `promise/finally`, `promise/native?`, and `promise/delay` follow the
`xt.lang.spec-promise` settlement and adoption model.

Bytes expose unsigned values from `0` through `255` through `bytes/get`; `bytes/u8` and
`bytes/s8` explicitly convert representations. The ordinary `bytes/get` library operation is
unsigned, while protocol `INth/nth` preserves the signed byte value. String encoding and decoding
are UTF-8.

File and socket namespaces are always present and aliased. Availability does not grant authority.
Unsupported or denied operations fail when invoked. JVM embeddings grant authority with Graal
`IOAccess`; the CLI grants it explicitly with `--allow-file` and `--allow-net`. `file/resolve`
resolves a child path beneath a supplied root and returns a normalized path string. File reads and
writes use bytes and return promises. Socket v1 follows the callback contract: `connect` accepts
host, port, a reserved options value, and an `(fn [error connection] ...)` callback; the current
implementation does not interpret options. `send` accepts bytes and returns the byte count;
`close` is direct. Receive/framing and HTTP are outside this slice.

Additional provider-backed namespaces are available through ordinary `require` and are not
automatically aliased:

| Namespace | Purpose |
| --- | --- |
| `std.lib.handle` | releases opaque HTA extension handles |
| `std.lib.context` | context registries, runtime spaces, lifecycles, and pointers |
| `std.lib.task` | task definitions, invocation, selection, and bulk processing |
| `std.lib.coroutine` | Lua-style coroutines: create, resume, yield, status, close, await |
| `code.test` | facts, fixtures, matchers, and structured test execution |

These providers are installed lazily under their canonical public namespace. Their implementation
classes and helper namespaces are not guest-visible API. Requiring a provider does not grant file,
network, process, reflection, classpath, or compilation authority.

`std.lib.coroutine` provides Lua-style coroutines on the Truffle runtime. `create` wraps a
function without starting it; `resume` starts or continues a coroutine and returns the yielded
or final value; `yield` suspends the current coroutine from any call depth and returns the next
resume's arguments; `status` reports `:suspended`, `:running`, or `:dead`. With multiple values,
`yield` and resume arguments pack into vectors (single values pass through as-is). Errors inside
a body rethrow at the resume site and leave the coroutine `:dead`. `close` unwinds a suspended
coroutine, running its `finally` clauses. `await` blocks the coroutine until a promise settles.
A single coroutine is not safe for concurrent `resume`/`close` calls from multiple threads;
resume is synchronous and single-resumer by design.
Var bindings established around a `resume` do not propagate into the coroutine body. Coroutines
are currently Truffle-only; the Rust runtime does not implement them yet.

`std.resp.client` is a separate, explicitly required blocking RESP2 client. It exposes `connect`,
`call`, `write`, `read`, `pipeline`, `open?`, and `close`; connections require network capability.
Bulk strings decode as UTF-8 strings by default and can be preserved as bytes with
`{:decode-bulk :bytes}`.
