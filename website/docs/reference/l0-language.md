# Hara L0 language contract

Version: `0.1`

This document is the normative, implementation-independent contract for the
Truffle-hosted Hara L0 runtime. The executable examples and expected values
are maintained in [`l0-conformance.edn`](l0-conformance.edn). An implementation
may use different storage or compiler techniques, but it must preserve the
observable behavior described here.

## 1. Source and reader

Hara source is read as immutable forms. Whitespace, commas, and semicolon
comments are discarded. Multiple forms may occur in one source. Supported
literal forms are nil, booleans, strings, characters, symbols, keywords,
integers, floating-point values, `N`-suffixed big integers, `M`-suffixed
big decimals, lists, vectors, maps, sets, quote, syntax-quote, unquote,
unquote-splicing, deref, metadata, and discard (`#_`).

Collections require matching delimiters. EOF, mismatched delimiters, invalid
dispatch, incomplete reader prefixes, invalid escapes, and invalid numeric
tokens are reader errors. Reader errors include the source name, line, and
column when a source name is available. Leading whitespace is not part of a
form's source span.

Aggregate forms may carry `:line`, `:column`, `:end-line`, and `:end-column`
metadata. This metadata is diagnostic information and does not change value
equality or symbol/keyword identity. Readable immutable values use canonical
printing and must round-trip through the reader. Ratios are rejected.

## 2. Values and evaluation

Nil and `false` are falsey; every other value, including zero, the empty
collection, and the empty string, is truthy. Expressions evaluate from left to
right. A `do` form returns its final expression, and a source containing
multiple top-level forms returns the final result.

The core special forms are `quote`, `if`, `do`, `when`, `when-not`, `and`,
`or`, `cond`, `let`, `letfn`, `binding`, `loop`, `recur`, `fn`, `def`,
`defn`, `defn-`, `declare`, `defmulti`, `defmethod`, `var`, `deref`, `set!`,
`throw`, `try`, `ns`, `in-ns`, `require`, `refer`, `use`, `alias`,
`defstruct`, `defprotocol`, `extend-type`, `protocol-call`, `field`, and
`apply`. `defn` is the only function-definition form; there is no `defn.xt`.

The ordinary collection functions `count`, `get`, `assoc`, `conj`, `cons`,
`nth`, and `empty` are protocol-backed language functions. `cons` follows the
public `(cons item collection)` argument order; the other update/lookup
functions place the collection first. These functions use the same context-
local protocol registry as `protocol-call`, so language-defined extensions are
visible without requiring Java interface implementation.

`let` initializers are evaluated in parallel against the enclosing lexical
environment. `letfn` installs all local function bindings before evaluating
the body, so self-recursion and mutual recursion work. Closures capture the
lexical values they reference. `recur` is valid only in tail position and
must match the enclosing `loop` or function arity.

Functions support fixed arities, variadic parameter vectors using a final
`&` binding, and multiple arity clauses. Exact arities take precedence over a
variadic fallback. `apply` spreads the final sequential argument into the
call. Invocation supports Hara functions, protocol `IFn` implementations,
multifunctions, and `defstruct` constructors.

The packaged `std/lib/foundation.hal` bootstrap defines `nil?`, `false?`, `true?`,
`empty?`, `first`, `second`, `rest`, and `not-empty` using ordinary L0 forms
and iterator operations. `rest` returns a lazy `Seq`, or nil when no values
remain. There is no separate `next` operation.

The same bootstrap provides ordinary names `map`, `filter`, `take`, `drop`,
`mapcat`, `keep`, `cycle`, `zip`, and `partition-pair`; these return lazy
`Seq` values. Their `iter-*` counterparts return raw one-shot iterators.
`map` is also a transform constructor: `((map f) collection)` is eager for
ordinary collections and lazy for an existing `Seq` or iterator. `seq` makes
the boundary explicit, so `(seq transform source)` is always lazy. Predicate
reductions `every?` and `any?` consume an iterator lazily until their result is
known.

The bootstrap also provides `get-in`, `assoc-in`, `update`, and `update-in` for
persistent nested values. These are ordinary `.hal` functions built on the
collection protocol functions; they do not introduce mutable update semantics.

Collection navigation also includes `last`, `reverse`, `key`, `val`, `keys`,
and `vals`. Membership is protocol-based: use `IFind/find` to retrieve an
entry and `IFind/has?` to distinguish absence from a present nil value.
`keys` and `vals` return lazy `Seq` values, while `reverse` returns a
persistent list and does not mutate its input.

`reduce` eagerly consumes an iterator with either `(reduce function value)`
or `(reduce function initial value)`. The two-argument form uses the first
source element as its accumulator and rejects an empty source; the three-
argument form returns the initial value for an empty source. The callback is an
ordinary Hara function receiving accumulator and element.

Destructuring supports nested positional vector patterns, vector rest
bindings, map `:keys`, `:strs`, `:syms`, `:as`, and `:or` patterns in function,
`let`, and `loop` bindings. Missing sequential or map values produce nil
unless a map default applies.

## 3. Exceptions and cleanup

`throw` propagates a guest value. `try` supports ordered typed catch clauses
and an optional `finally`; typed catches match Hara struct types and the
documented scalar/generic exception categories. Unmatched guest values
propagate. `finally` executes during normal completion and exception
unwinding. Arity, unbound symbol, invalid form, reader, protocol, and
interop failures are stable Hara errors and retain source sections where the
offending form has a source span.

## 4. Persistent collections and iteration

Lists, vectors, maps, sets, queues, tuples, ordered collections, and sorted
collections are persistent values. Literal `[]`, `{}`, lists, and sets never
become mutable merely because they cross a host boundary. Protocol operations
such as count, lookup, nth, assoc, dissoc, conj, cons, first, and
empty preserve the collection-family rules tested by the conformance suites.

The ordinary `dissoc` function accepts a collection followed by one or more
keys and returns persistent updates. `peek` and `pop` expose first-element
navigation through the collection protocol; they never mutate the input.

The bootstrap provides lazy `range`, `repeat`, `repeatedly`, and `iterate`
generators. These return `Seq` values; their `iter-*` primitives return raw
iterators. Infinite forms remain lazy until consumed by `take`, `iter-next`, or
another iterator operation.

`take-while` and `drop-while` are lazy iterator combinators. They evaluate
predicates only as demand advances and close their source when it is exhausted
or the stopping predicate is reached.

`partition-all` emits lazy persistent vector chunks and retains a final partial
chunk; `partition` emits only complete chunks and discards an unmatched tail.
Both require a positive chunk size.

`map` and `iter-map` accept one or more sources. With multiple sources they
advance them in lockstep and stop at the shortest source, invoking the function
with one value from each source.

`interpose` lazily inserts a separator between source values. `interleave`
round-robins multiple sources and stops when the shortest source is exhausted.

Hara is iterator-first. It does not require Clojure `ISeq`/`Seq` semantics.
The core iterator forms are:

* `iter`, `iter-has?`, `iter-next`, and `iter-close`;
* lazy `concat`, `iter-map`, `iter-filter`, `iter-take`, `iter-drop`, and
  `iter-zip`, `iter-cycle`, `iter-partition-pair`, `iter-mapcat`, and
  `iter-keep`.

`iter-cycle` re-acquires a replayable source only after its current iteration
is exhausted. `iter-partition-pair` emits two-element persistent vectors and
drops an unmatched final element.

Iterator sources are acquired only when demanded. `iter-next` reports a stable
exhaustion error, and closing a wrapper closes its acquired source iterators.
The language does not include mandatory transducers, `transduce`, or
`eduction`.

## 5. Explicit marker values and bytes

Persistent literals remain distinct from target-like mutable values:

* `array` creates a mutable indexed marker.
* `object` creates a mutable string-keyed marker.

Only values created by these markers accept restricted dot calls. Array methods
are `get`, `set`, `push-first`, `push-last`, `pop-first`, `pop-last`, `insert`,
`remove`, `clone`, `slice`, `map`, `filter`, `fold-left`, and `fold-right`.
Object methods are `has?`, `get`, `set`, `delete`, `clone`, `assign`, `keys`,
`vals`, and `pairs`. A call has the form `(. target (method arguments...))` and
does not expose host members or reflection. Marker arrays also implement the
ordinary `ICount` and `INth` collection protocols. Mutable values have identity
semantics and are not specified as thread-safe.

Bytes are an ordinary value category constructed with `(bytes ...)`. Elements
use signed-byte storage and accept the checked `-128..255` input domain.
Operations live in `std.lib.bytes`: `bytes/count`, `bytes/get`, `bytes/set`,
`bytes/copy`, `bytes/slice`, `bytes/u8`, and `bytes/s8`. `bytes/get` returns an
unsigned element in the range `0..255` and accepts an optional fallback for an
invalid index; without a fallback it reports a bounds error. Protocol
`INth/nth` exposes the signed stored element. `bytes/set` mutates and returns
the same byte array after checked conversion. Copy and
slice allocate independent storage. Readable bytes print as `(bytes ...)`,
and equality/hashing use byte content. Raw connector transport preserves
bytes as bytes.

## 6. Numbers

The numeric categories are fixed-width integral values, floating-point values,
`java.math.BigInteger`, and `java.math.BigDecimal`. Arithmetic promotes to a
representation capable of preserving the operation result; primitive pairs
use specialized Truffle paths and big-number cases use generic fallback.
`+`, `-`, `*`, `/`, and `mod` are variadic with the documented identities and
unary behavior and are also callable Vars, so they can be passed to functions
such as `reduce`. Division is ratio-free: `(/ 2)` evaluates to integer `0`.
Division or remainder by zero is an error. Numeric equality and hashing
normalize equivalent integral/decimal representations, decimal scale, and
signed zero according to the conformance cases. NaN and infinities are valid
floating values with the specified comparison behavior.

The comparison and equality operators `<`, `<=`, `>`, `>=`, `=`, and `not=` are
also callable Vars. They require at least two arguments and apply pairwise from
left to right, so they can be passed to iterator consumers such as `reduce`.

Ratios, implicit complex numbers, and an implicit irrational-number tower are
not L0 numeric categories. They may be explicit library or host values later.

## 7. Protocols, structs, and multimethods

### 7.1 Collection operation matrix

The collection profile distinguishes language values from dispatch categories. A
Java object participates only when it implements a Hara protocol adapter; being
a `java.util.Collection` does not implicitly grant language operations. Foreign
polyglot values likewise require an explicit protocol extension.

| Receiver family | Lookup / invocation | Persistent update | Count / empty | Indexed access | Iteration |
| --- | --- | --- | --- | --- | --- |
| Persistent maps (HAMT, ordered, sorted, trie) | keys preserve present-`nil` versus missing fallback; maps are callable | `assoc`/`dissoc` return the same concrete family and preserve the original | supported; `empty` preserves the family | unsupported unless the concrete ordered/sorted contract declares it | entries are Hara `IPair` values |
| Persistent sets (HAMT, ordered, sorted) | callable membership returns the stored value or fallback | `conj`/removal return the same family and preserve the original | supported; `empty` preserves the family | ordered/sorted forms expose only their declared indexed contract | yields unique values |
| Vector | callable `nth` | `assoc` and `conj` return a new vector | supported | supported; negative and past-end indexes fail | insertion order |
| List | lookup and `nth` through its sequential contract | `cons`/`push-first` prepend; navigation returns new lists | supported | supported by its declared sequential contract | head to tail |
| Queue | sequential lookup | `conj`/`push-last` enqueue and preserve the original | supported | supported by its declared sequential contract | head to tail |
| Tuple | callable `nth` | fixed-arity operations return the canonical resulting tuple arity | supported; empty is canonical `Tup0` | supported | tuple order |
| Mutable Hara collections and `array`/`object` markers | supported only by their declared protocols or restricted dot methods | mutation returns the same identity and is immediately visible | declared collection protocols only | arrays support indexed access; objects use string keys | declared protocol only |
| Bytes | byte lookup supports a fallback; `INth` exposes signed storage | `bytes/set` mutates the byte buffer and returns its identity | count supported | checked bounds | byte order |
| `nil` | lookup returns `nil` or the supplied fallback | `conj` creates a singleton list; `assoc` is unsupported | count is zero and empty is `nil` | unsupported | empty iterator where explicitly requested |
| Primitive scalar | no collection lookup or update | unsupported | unsupported unless explicitly extended | unsupported | unsupported |
| Foreign polyglot value | only explicitly installed foreign extensions | only explicitly installed foreign extensions | only explicitly installed foreign extensions | only explicitly installed foreign extensions | only explicitly installed foreign extensions |

Duplicate map keys retain one entry with the last associated value. Duplicate set
items retain one value. Invalid indexes and missing protocol implementations are
errors identifying the protocol/method, receiver category, and searched dispatch
path. Mutable and persistent operations must never silently cross their identity
boundary.

Hara remains iterator-first: this matrix does not imply Clojure `ISeq`,
transducers, `transduce`, or `eduction` semantics.

### 7.2 Protocol dispatch and language extensions

Protocols are language descriptors with context-local dispatch registries.
Java interfaces are optional adapters and fast paths, not the language
definition. `defprotocol` declares methods; `extend-type` installs language
implementations; `protocol-call` performs dispatch. Dispatch supports Hara
values, adapted Java values, primitives, nil, and foreign values. Replacing a
method or extension invalidates affected dispatch assumptions.

`defstruct` creates immutable `HaraStruct` values. Struct metadata is separate
from fields and survives `with-meta`; metadata does not affect value equality
or hashing. `IFn` is a language protocol and can be extended to structs.
`defmulti`/`defmethod` dispatch by Hara equality and support `:default`.

## 8. Vars, namespaces, macros, and modules

Definitions live in namespace Vars. `var`, `deref`, `set!`, `alter-var-root`,
and `binding` implement root and dynamic binding behavior. Dynamic bindings
are restored after normal completion and guest errors. Namespace aliases and
referred Vars preserve live Var identity.

`defmacro` runs in the context-local compile-time registry. Syntax-quote,
unquote, unquote-splicing, variadic macros, `macroexpand-1`, and recursive
`macroexpand` are supported. Literal `require` loads filesystem or packaged
`.hal` modules during analysis when necessary, supports aliases, `:refer`,
`:refer-macros`, and `:reload`, and rolls back failed loads transactionally.
Already compiled Truffle call targets are immutable; a newly compiled source
observes a reloaded macro/module definition.

The packaged L0 bootstrap is intentionally small and language-level: its
current functions are defined in `std/lib/foundation.hal`. The complete Foundation
stdlib and KMI/L1 port are later migration work, not hidden Java semantics.

## 9. Host and Native Image boundary

Level 0 has no guest-visible JVM or general host interop. `host-symbol`,
`host-get`, and `host-call` are unbound symbols. Polyglot array/member access is
available only for values that explicitly expose that interop; guest dot calls
remain restricted to `array` and `object` markers.
Native Image must include the same language resources and supported adapters;
reflection, generated classes, mutable classpaths, and unrestricted host
loading are not required by the core profile.

The JVM and Native Image profiles must execute the same conformance manifest.
Native Image startup, binary size, reachability/build-report, and benchmark
results are release evidence rather than language semantics.

## 10. Conformance and intentional differences

`l0-conformance.edn` is the stable executable corpus. Each case has an ID,
category, source/setup, and expected value, type, readable form, or error.
Implementations classify mismatches as a bug, a capability difference, or an
approved specification revision. The current intentional differences from
Clojure are: no mandatory ISeq/Seq, no ratios, no transducers/
`transduce`/`eduction`, no `defrecord`, no `deftype`, and no `defn.xt`.
