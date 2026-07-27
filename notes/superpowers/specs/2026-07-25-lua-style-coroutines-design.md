# Lua-style coroutines for hara — design

Date: 2026-07-25
Status: approved (design), pending implementation plan

## Goal

Add full Lua-style coroutines to hara: `create` / `resume` / `yield` / `status` with
bidirectional value passing, yield at any call depth inside a coroutine body, and an
`await` helper for composing with `std.lib.promise`. First implementation targets the
Truffle runtime only; the Rust/WASM runtime follows later by extending its existing CPS
fiber (`wasm/src/fiber.rs`). The parity gap is documented in the spec.

## Approach

**Virtual-thread rendezvous** (chosen over an analyzer-level state-machine transform and
a CPS rewrite of the Truffle evaluator).

Each coroutine body runs on a Java 21 virtual thread. `resume` and `yield` exchange
values through a two-queue rendezvous (in/out `SynchronousQueue`s): resume parks the
caller and unparks the coroutine; yield does the reverse. Because the JVM preserves the
coroutine's entire call stack while parked, `yield` suspends from any depth — nested
functions, loops, `map` callbacks — with **zero changes to the analyzer, AST, or
evaluator**. The feature is a pure library, installed exactly like `std.lib.task`.

Rejected alternatives:

- **Analyzer-level state-machine transform** (`yield` as an L0 special form, forms
  rewritten into a resumable state machine, like legacy `std.lib.generate`): thread-free
  and deterministic, but invasive (analyzer, AST nodes, L0 spec, conformance corpus) and
  leaky around macros, `try/catch`, and `recur`. Not reusable for the Rust port either,
  since that is a different evaluator in a different language.
- **CPS rewrite of the Truffle evaluator** (mirroring `wasm/src/fiber.rs`): one unified
  model, but a high-regression-risk rewrite of a working evaluator for one library
  feature, and it does not accelerate the WASM port.

## API surface

Namespace `std.lib.coroutine` (suggested require alias `co`):

| Function | Semantics |
| --- | --- |
| `(create f)` | Wrap function `f` in a coroutine. Does not start it. Status `:suspended`. |
| `(resume co & args)` | Start or continue `co`. First resume passes `args` to `f`; later resumes deliver `args` as the return value of the suspending `yield`. Returns the yielded value(s), or the body's final return value. Synchronous: when `resume` returns, the coroutine has run to its next yield point or completed. |
| `(yield & vals)` | Suspend the current coroutine, handing `vals` to the resumer. Throws if called outside any coroutine. |
| `(status co)` | `:suspended` \| `:running` \| `:dead`. Lua's `normal` (this coroutine resumed another) collapses into `:running`. |
| `(coroutine? x)` | Predicate. |
| `(close co)` | Mark dead and interrupt the parked thread; the body's `finally` clauses run on the coroutine's own thread. Only valid on a `:suspended` coroutine — throws on `:running` (the coroutine is executing; interrupting a running thread would not stop it deterministically). No-op on `:dead`. (Lua 5.4 `coroutine.close` parity; prevents leaked parked threads.) |
| `(await p)` | Suspend the coroutine until promise `p` settles; return its value, throw on failure. On the JVM: a plain blocking deref on the coroutine's virtual thread. Outside a coroutine, behaves as a plain deref. `close` cannot fire during an `await` because the coroutine is `:running` then and `close` throws on `:running`. |

### Resume contract

`resume` returns plain values. The caller distinguishes a yielded value from the body's
final return by checking `(status co)` afterwards: `:suspended` means the value came
from `yield` (more coming), `:dead` means it was the final return. (Lua's model;
tagged `[:yielded v]` / `[:returned v]` tuples were considered and rejected as
un-Lua-like, and promise-wrapped results were rejected because resume is synchronous —
there is no outstanding async work to deref.)

### Multi-value packing

Hara is expression-oriented, so Lua's multiple return values pack into vectors:

- `(yield a b c)` yields `[a b c]`; `(yield a)` yields `a`; `(yield)` yields `nil`.
- `(resume co a b c)` makes the suspending `yield` return `[a b c]`; a single arg is
  delivered as-is; no args deliver `nil`.

### yield vs await on promises

Both are deliberate and distinct:

- `(yield p)` where `p` is a promise passes the promise **object** to the resumer as an
  ordinary value; the coroutine stays parked until someone resumes it manually. This is
  the scheduler pattern: the resumer receives the pending operation and owns scheduling
  policy (when to resume, with what value, batching, cancellation). Hand-rolled
  schedulers deliver the settlement via e.g. `(resume co @p)`; the runtime enforces
  nothing.
- `(await p)` is the value-receiving convenience: suspend until settlement and continue
  with the result. Straight-line async code inside coroutines, no `then` chains.

## Semantics (the portable contract)

These rules are host-neutral and are what the Rust/WASM port (and future xtalk targets)
must honor:

1. **Cooperative transfer, no preemption.** Only one coroutine runs at a time; control
   moves explicitly at `resume`/`yield`. Between yield points a coroutine runs alone.
2. **Yield at any depth.** `yield` works from any function called within a coroutine
   body, not just the body's top-level frame. (This is the expensive guarantee: non-JVM
   hosts must make their whole call stack suspendable — CPS fiber for Rust,
   async-everywhere compilation for JS/Python xtalk targets, native coroutines for Lua.)
3. **Bidirectional plain values.** Resume arguments flow in, yielded values flow out.
   No host objects (threads, queues) appear in the semantics.
4. **Status as data.** `:suspended` / `:running` / `:dead` only.
5. **Errors rethrow at the resume site** with the coroutine's stack preserved; the
   coroutine is then `:dead` and cannot be retried.
6. **Resuming a `:dead` or `:running` coroutine throws.** Yielding outside a coroutine
   throws.

## JVM implementation

New files:

- `src/main/java/hara/truffle/StdLibCoroutine.java` — `@HaraExport` static methods plus
  a package-private `HaraCoroutine` holder (state enum, in/out `SynchronousQueue`s,
  virtual thread handle, result/throwable slots). Modeled on `StdLibTask.java`.
- `src/main/java/hara/truffle/StdLibCoroutineLibraryProvider.java` — library provider.
- `src/test/java/hara/truffle/StdLibCoroutineTest.java` — focused tests (below).

Modified files:

- `src/main/resources/META-INF/services/hara.truffle.HaraLibraryProvider` — register the
  provider (library installs lazily on first namespace use).
- `spec/hara/runtime-libraries.md` — `std.lib.coroutine` contract entry.
- `spec/hara/xtalk-equivalence.md` — add `x:coroutine-create` / `x:coroutine-resume` /
  `x:coroutine-yield` / `x:coroutine-status` rows (equivalence metadata, per the file's
  existing convention).
- No `wasm-truffle-parity.edn` or `hara-core-symbols.json` changes: the parity file is an
  executable corpus that must not gain uncovered cases (the Truffle-only status is recorded
  in `runtime-libraries.md` instead), and the core-symbols inventory covers L0 plus eagerly
  referred `std.lib.foundation` only — lazy providers like `std.lib.task` are not listed.
- `docs/reference/` — coroutines documentation page (location confirmed against docs
  layout during implementation).

Explicitly **not** touched: `HaraAnalyzer.java`, `HaraNodes.java`,
`spec/hara/l0-language.md`, `spec/hara/l0-conformance.edn`. Coroutines are a library;
L0 is unchanged.

### Mechanism details

- The virtual thread starts lazily on the first `resume`, not at `create`.
- Rendezvous: two `SynchronousQueue`s. `resume` puts args on *in* and takes from *out*;
  `yield` does the reverse. Both use interruptible `take()`, which `close` relies on.
- `yield` locates the current coroutine through a `ThreadLocal<HaraCoroutine>` set by
  the thread wrapper; absent → throw.
- The thread wrapper catches any throwable from the body, marks the coroutine `:dead`,
  and hands the throwable across the rendezvous for rethrow at the resume site.
- `close` on a `:suspended` coroutine marks it dead and interrupts the parked thread;
  the interruptible `take()` wakes and a cancellation exception unwinds the body,
  running `finally` clauses on the coroutine's thread. On `:running` it throws; on
  `:dead` it is a no-op.
- `try/finally` inside bodies behaves naturally: finally runs on normal return, on
  error, and on `close`.
- Nested coroutines (a coroutine resuming another) need no extra code; the rendezvous
  serializes naturally. Any thread may resume any suspended coroutine.

### Known limitations (documented in the contract)

- **Dynamic vars do not cross the resume boundary.** Bindings established around a
  `resume` do not propagate into the coroutine; the body sees root bindings plus its own
  internal bindings. Revisitable if real code needs it.
- An abandoned suspended coroutine holds a parked virtual thread until `close` or
  process exit. `close` is the explicit remedy.

## Rust/WASM path (future work, documented not implemented)

`wasm/src/fiber.rs` already implements a CPS evaluator that suspends on a pending
promise and resumes on settlement — a one-shot coroutine. The port adds a `Yield` step
variant alongside the existing `Wait` step plus guest-facing builtins, implementing the
same contract above. No Rust code changes are part of this design; the parity gap is
recorded in `spec/hara/runtime-libraries.md`.

### xtalk portability note

The contract maps onto xtalk as `x:coroutine-*` operations. Host outlook: Lua is a
near-identity mapping; JS/Python targets implement stackful yield via async-everywhere
compilation (plain generators are stackless and insufficient for yield-at-any-depth);
Rust/WASM uses the CPS fiber.

## Testing

`StdLibCoroutineTest` (JUnit, modeled on `StdLibFoundationTest`):

- create/resume/yield round-trips; status transitions across the lifecycle
- bidirectional values; multi-value packing rules (0, 1, many args each way)
- yield from a nested helper (depth > 1)
- error in body → rethrown at resume, status `:dead`, subsequent resume throws
- resume on `:dead` / reentrant resume on `:running` → throws
- yield outside a coroutine → throws
- `close` on a suspended coroutine → finally runs, status `:dead`, resume throws;
  `close` on dead is a no-op
- nested coroutines (coroutine resumes another)
- `await` with an already-settled and a later-settled promise; `await` failure rethrows
- generator-pipeline integration test (coroutine as lazy producer consumed by a loop)

Verification: `mvn -q -Ptruffle -Dtest=hara.truffle.StdLibCoroutineTest test`, then full
`mvn -q test`, plus a `./hara eval` REPL smoke test.
