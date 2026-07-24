# Rust and WASM runtime mapping

## Native Rust CLI

The shared Rust runtime can also be built as a native command-line executable. It uses the same
evaluator, protocols, persistent data structures, and provider traits as the browser/WASM build;
only the host adapters differ.

```text
$ cargo run --manifest-path rust/Cargo.toml --bin hara -- eval '(+ 19 23)'
42

$ cargo run --manifest-path rust/Cargo.toml --bin hara -- --file program.hal
```

With no `eval` or `--file` argument, the executable reads forms from standard input as a small
REPL. Native filesystem and socket capabilities are opt-in:

```text
$ hara --root ./sandbox --native-sockets
```

The native binary is intended for CLI and server use. The WASM build remains the
browser/runtime-embedding target, while both targets share the Rust core so protocol and evaluator
behavior can be tested for parity.

This document maps the tested Truffle Hara L0 contract to a Rust implementation. It is a
portability design, not a second language specification: observable behavior comes from
`l0-language.md`, `runtime-libraries.md`, and `l0-conformance.edn`.

## Runtime layers

```text
Hara reader/forms
        |
        v
Rust evaluator + lexical environments
        |
        +--> persistent values (Hara-owned structural sharing)
        +--> explicit mutable markers (runtime handles)
        +--> protocol registry
        +--> iterator/promise machinery
        |
        v
provider boundary
   |         |          |
 WASM     browser     native/WASI
 host     adapters     capabilities
```

The evaluator must not call browser, WASI, or operating-system APIs directly. Host-dependent
operations use providers and return the same stable Hara errors as the Truffle runtime.

## Value and ownership model

Persistent values are Hara-owned implementations, split by data structure under
`rust/src/lang/data/`. The vector uses a 32-way persistent trie, maps use a native HAMT, sorted
maps use a persistent balanced tree, and the list, queue, ordered collections, sets, trie, tuples,
symbols, and keywords retain their Java protocol shapes. Structural sharing uses `Rc`; no tracing
garbage collector or third-party immutable-collection crate is required.

```text
Value
├── immutable collections  -> Hara vector/HAMT/tree/list modules with Rc sharing
├── functions/environments -> Rc-owned frames
├── array/object markers   -> Rc<RefCell<...>> or opaque handles
├── promises               -> provider-backed settlement state
└── foreign values         -> provider-owned opaque handle
```

Use `Weak` references or a handle table for closure/provider cycles. Do not use interior
mutability for ordinary persistent values.

## Mapping matrix

| Hara surface | Rust target | Acceptance source |
| --- | --- | --- |
| Reader, metadata, literals | `Reader`, `Form`, source spans | reader cases in `l0-conformance.edn` |
| `if`, `do`, `let`, `loop`, `recur`, `fn` | evaluator nodes and lexical frames | compiler/runtime cases |
| Persistent collections | Hara-owned list, vector trie, HAMT map, and set modules | collection and navigation cases |
| Protocols | context-local `ProtocolRegistry` | protocol cases |
| Iterators | closeable `HaraIterator` trait | iterator cases |
| `promise/*` | `PromiseProvider` plus adoption/recovery rules | generated-library tests |
| Bytes | owned byte buffer with signed protocol view and unsigned library view | byte cases |
| `array`/`object` | explicit mutable marker handles | mutable-boundary tests |
| `file/*` | `FileProvider` capability | capability tests |
| `socket/*` | callback-based `SocketProvider` | loopback capability tests |
| JVM/native interop | provider-specific extension only | native-flavor tests |
| `x:*` names | excluded from Hara L0; future `hara.polyglot` adapter | xtalk equivalence metadata |

## Provider interfaces

Providers own host handles and capability checks. `file/resolve` normalizes a child path under a
granted root; it never grants read/write authority. Browser providers may use virtual storage and
WebSocket-style connections, while WASI/native providers may use preopened directories and host
networking. Unsupported hosts return `unsupported`; denied capabilities return `denied`.

```rust
trait FileProvider {
    fn resolve(&self, root: &str, path: &str) -> Result<String, FileError>;
    fn read(&self, path: &str) -> Promise<Result<Bytes, FileError>>;
    fn write(&self, path: &str, bytes: Bytes) -> Promise<Result<(), FileError>>;
}

trait SocketProvider {
    fn connect(&self, host: &str, port: u16, options: Value,
               callback: Callback) -> Result<(), SocketError>;
    fn send(&self, socket: SocketHandle, bytes: Bytes) -> Result<usize, SocketError>;
    fn close(&self, socket: SocketHandle) -> Result<(), SocketError>;
}
```

Sockets remain callback-based. The Rust runtime must not add socket-promise method families.

## Conformance sequence

1. Port the reader and scalar evaluator against the existing conformance IDs.
2. Add persistent values and protocol dispatch, then run collection/protocol IDs.
3. Add iterators, markers, bytes, and promises using the focused Java tests as cross-runtime cases.
4. Add provider adapters only after portable cases pass; capability-specific cases must remain
   explicit and must not become portable by accident.
## Target profiles

The exported runtime reports one of three profiles:

```text
wasm   -> browser-safe memory files and callback loopback sockets
native -> scoped std::fs files and callback TCP sockets
wasi   -> provider registry reserved for preopened files and host networking
```

The browser playground installs the memory-file and loopback providers explicitly. Native hosts
install `NativeFileProvider` and `NativeSocketProvider` through the same registry. WASI remains a
separate host-selection point: the portable core does not assume ambient filesystem or network
authority.

## Reproducible release evidence

The main workflow verifies all three runtime profiles:

```text
native  cargo test --manifest-path rust/Cargo.toml
browser cargo build --target wasm32-unknown-unknown --release --lib
        npm run test:hta && npm run test:browser
wasi    cargo build --target wasm32-wasip1 --release --lib
parity  1,024 shared cases through Rust WASM and Truffle
```

CI uploads `rust-wasm-artifact-sizes.txt`, containing byte counts for the browser runtime, WASI
runtime, and raw HTA extension. It also uploads the Rust-WASM/Truffle timing matrix. These reports
are generated from the same commit as the conformance run rather than copied into documentation.


## Iterator boundary

The Rust core exposes a host-neutral closeable iterator value:

```text
(iter collection) -> iterator
(iter-has? iterator) -> boolean
(iter-next iterator) -> value
(iter-close iterator) -> nil
```

`iter-next` consumes one value and reports `iter-next reached the end of the iterator` at the boundary, matching the JVM runtime. `iter-map` and `iter-filter` preserve callback-based function invocation while remaining independent of filesystem, sockets, JVM classes, or browser APIs. The WASM build uses the same evaluator and iterator representation.

## Resource loading boundary

The runtime accepts host-supplied source resources through a registry:

```text
register-resource(name, source text)
          |
          v
load-resource(name) -> evaluate in the current runtime environment
```

Resource loading is deliberately source-oriented: the host supplies a named Hara resource, while the Rust evaluator owns parsing, evaluation, and namespace state. A missing resource returns the stable `module/not-found` error. Browser hosts can populate the registry from fetched assets; native and WASI hosts can connect it to their own module providers without changing the evaluator.

## Bytes and mutable storage

The Rust core now separates transport bytes from language byte buffers. File and socket providers continue to use owned `Vec<u8>` transport data; `(bytes ...)` creates a mutable byte buffer with signed storage and checked `-128..255` input.

```text
(bytes 1 2 -3) -> (bytes 1 2 -3)
bytes/get       -> unsigned 0..255
bytes/u8 -1     -> 255
bytes/s8 255    -> -1
bytes/set       -> same mutable buffer
```

This keeps byte mutation explicit while preserving byte-for-byte provider transport.

## Marker values and dot calls

Mutable host-like values are explicit Rust-owned markers:

```text
(array ...)  -> Rc<RefCell<Vec<Value>>>
(object ...) -> Rc<RefCell<Vec<(String, Value)>>>
       |
       +--> (. target (get ...))
       +--> (. target (set ...))
       +--> (. target (clone ...))
```

The evaluator rejects dot calls on ordinary persistent vectors/maps and does not inspect host members. Array mutation returns the same marker identity; object keys are limited to strings or keywords.

## Promise values

Promises are first-class Rust values backed by a single-settlement state machine:

```text
(promise) -> pending promise
      |\
      +--> promise/resolve -> fulfilled
      +--> promise/reject  -> rejected
      +--> promise/adopt   -> copies a settled source
```

`promise/state` returns `:pending`, `:fulfilled`, or `:rejected`; `promise/value` returns the fulfilled value or a stable error. Provider operations can return the same value without introducing separate socket-promise APIs.

## Portable protocol calls

The evaluator exposes protocol dispatch without requiring a host registry for the core collection protocols:

```text
(protocol-call ICount count value)
(protocol-call INth nth value index)
(protocol-call ILookup lookup value key [default])
```

Byte `INth` returns signed stored values while `bytes/get` remains unsigned. Unknown protocol/method pairs return a stable missing-method error; host extensions continue to use the separate `ProtocolRegistry`.

## Module resource loading

Registered resources may contain multiple top-level forms. The evaluator processes them in order, allowing a module to define several functions or values before returning its final form:

```text
register_resource("math", "(defn inc ...) (defn dec ...) (inc 4)")
require_resource("math") -> evaluates once
require_resource("math") -> :loaded
```

Resource loading remains explicit at the host boundary; source is parsed and evaluated by Rust, and no filesystem lookup is implied.

## Namespace isolation

Runtime namespaces own independent lexical environments:

```text
user  -> {answer: 42}
math  -> {answer: 7}

use_namespace("math") -> math bindings
use_namespace("user") -> user bindings
```

`create_namespace` allocates an empty namespace, `use_namespace` switches while preserving the previous environment, and `eval_in_namespace` evaluates source in the selected namespace. This is the foundation for `:require`, aliases, and generated runtime libraries without implicit source-file lookup.

## Core collection helpers

The Rust evaluator includes the host-neutral bootstrap operations used by `std/lib/foundation`: `not`, numeric comparisons, `mod`, `first`, `rest`, `last`, and `empty?`. They operate over persistent collections, byte buffers, marker arrays, and iterators without invoking host APIs.

## Iterator aliases and combinators

The Rust core exposes the bootstrap iterator aliases and bounded combinators:

```text
map/filter       -> iter-map/iter-filter
take/drop        -> iter-take/iter-drop
zip              -> pair vectors until either source ends
cycle            -> replayable closeable iterator
concat           -> sequential iterator over sources
```

All returned values remain closeable iterator handles. `iter-next` preserves the stable exhaustion error, and callbacks are evaluated through the same captured-function mechanism as ordinary calls.

## Namespace aliases

Aliases are explicit runtime metadata, separate from source lookup:

```text
alias_namespace("math", "hara.math")
resolve_namespace("math") -> "hara.math"
require_resource_in_namespace("helpers", "math")
```

This is the Rust/WASM foundation for Hara `:require` aliases and generated library namespaces.

## String and byte library boundary

Portable string/byte operations are implemented in the Rust core:

```text
str, str/count, str/trim, str/upper, str/lower
str/encode <-> str/decode (UTF-8)
bytes/copy, bytes/slice
```

Copies and slices allocate independent byte-buffer storage. Invalid UTF-8 and out-of-range byte indexes return stable evaluator errors.

## Complete marker method boundary

Array markers now support `push-first`, `push-last`, `pop-first`, `pop-last`, `insert`, `remove`, `clone`, and `slice` in addition to indexed access. Object markers support `keys`, `vals`, `pairs`, and `assign` alongside lookup, mutation, deletion, and cloning. All methods remain available only through restricted dot calls.

## Callback socket API

The public Runtime exposes socket handles without changing the callback contract:

```text
socket_connect(host, port) -> handle
socket_send(handle, bytes) -> byte count
socket_close(handle) -> nil
```

Provider callbacks remain the event boundary; these methods do not create socket-promise variants. Unsupported and invalid providers return stable `socket/*` errors.

## Signed bit operations

The Rust core implements the signed 32-bit bit library:

```text
bit-and, bit-or, bit-xor, bit-not
bit-shift-left, bit-shift-right (distance 0..31)
```

Results wrap as signed 32-bit integers, and invalid shift distances return a stable range error. These names are directly compatible with the corresponding xtalk operation family.

## L0 numeric and truth predicates

The Rust core now provides the small bootstrap predicates `inc`, `dec`, `zero?`, `pos?`, `neg?`, `even?`, `odd?`, `nil?`, `true?`, and `false?`. It does not provide `some` or `some?`; use `any?` for predicate reduction and the collection protocols for membership.

## Collection membership and navigation

The core collection surface includes `keys` and `vals` for persistent maps, mutable objects, vectors, and lists. Membership is exposed by `IFind/find` and `IFind/has?`: maps and objects test keys, vectors and lists test valid numeric indexes, and sets test values. `has?` remains true for a present key whose value is nil.

## Rust protocol and persistent-vector layer

The WASM crate exposes a runtime-neutral `lang::protocol` module containing
the collection protocol traits (`IAssoc`, `IConj`, `ICons`, `ICount`, `IFind`,
`ILookup`, `INth`, and related persistence, metadata, display, equality, and
hash traits). `IFind::has` is derived from `find`, preserving the distinction
between an absent entry and an entry whose value is nil.

`lang::data::Vector` is a persistent, structurally shared vector with
association, append/pop, metadata, subviews, and mutable conversion support.
The protocol layer is independently testable and does not depend on the
evaluator or browser host APIs.

## Application and pair helpers

The bootstrap evaluator supports `identity`, `apply`, `key`, `val`, and `reverse`. `apply` accepts a captured Hara function or built-in arithmetic operator and flattens the final collection argument.

## Loop and recur

The evaluator supports the common single-binding tail-recursive form:

```text
(loop (state initial)
  (if condition
    (recur next-state)
    result))
```

`recur` is an internal evaluator value consumed by its enclosing `loop`; it cannot escape as a user-visible value, and arity mismatches return stable errors.

## Variadic functions

Function parameter vectors may use a final `& rest` pair:

```text
(fn [head & tail] ...)
```

Fixed parameters retain exact arity; extra arguments are bound as a Hara list under `rest`. This enables bootstrap helpers such as variadic `map`, `concat`, and `apply` without host-specific calling conventions.

## Binding vectors

`let` accepts either the compact pair form or a standard binding vector with multiple sequential name/value pairs:

```text
(let [x 19 y 23] (+ x y))
```

Bindings are evaluated left-to-right, so later initializers may refer to earlier names. The environment is restored after the body completes.

## Multi-state loops

`loop` accepts the same binding-vector shape as `let`, and `recur` may return one value per state binding:

```text
(loop [x 0 y 1]
  (if (< x 4)
    (recur (+ x 1) (+ y x))
    y))
```

The evaluator validates recur arity and restores all outer bindings after completion.

## Recur safety

`recur` is an internal control-flow value and cannot escape its enclosing `loop`, top-level evaluation, or function boundary. Escaped recur attempts return `recur must be inside loop`.

## Namespace declarations in resources

Registered module sources may begin with `(ns namespace ...)`. The Rust evaluator validates the namespace symbol and ignores declaration clauses at evaluation time; Runtime namespace selection and aliases remain explicit host-neutral APIs.
