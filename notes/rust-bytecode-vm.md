# Staged bytecode VM for the Rust runtime — milestone 1 design

Working design note for GitHub issue #195. Non-normative: it does not change
the portable HAL contract in `specs/`; it describes how the Rust runtime
implements that contract for a small synchronous subset. For the areas it
covers, the normative successor is now
`specs/runtime/draft/hal-bytecode-vm.edn` (with its machine-checked corpus in
`specs/runtime/draft/conformance/bytecode-vm.edn`); where this note and that
spec disagree, the spec wins.

Status: milestone 2 (experimental, synchronous; closures and defn
lowering). The VM is disabled by default behind the `bytecode-vm` Cargo
feature and never replaces `Runtime::eval_native`. Milestone 1 covered
the closure-free synchronous subset; §17 records the milestone 2 delta.

## 1. Execution model

A synchronous stack machine. Compilation is a separate, explicit stage:

```text
source ──parse──▶ SpannedForm ──compile──▶ Program ──validate──▶ execute ──▶ Value
```

- The compiler accepts the already-read `Form` tree (no macros, no namespace
  rewriting) and emits a typed instruction program.
- The validator checks the program once, before any execution.
- The machine interprets the validated program to completion or failure.
  There is no suspension, yielding, or resumption in this milestone; the
  dispatch loop is one `match` over instructions inside `Machine::run`.
- Unsupported forms are compile-time errors. The VM never falls back to the
  tree-walking evaluator, so benchmark and differential numbers stay honest.

## 2. Program and function representation

```rust
pub struct Program {
    pub constants: Vec<Value>,          // literal pool, reuses core::Value directly
    pub functions: Vec<FunctionPrototype>,
    pub entry: FunctionId,              // index into `functions`
}

pub struct FunctionPrototype {
    pub name: Option<String>,
    pub arity: u16,                     // 0 for the entry function in this milestone
    pub local_count: u16,               // slot array size
    pub max_stack: u16,                 // declared operand-stack high-water mark
    pub code: Vec<Instruction>,
    pub source_map: SourceMap,          // per-instruction source positions
}
```

Milestone 1 emits exactly one prototype (the entry function). The
`functions`/`entry` structure exists now so the closure milestone can add
prototypes without changing `Program`.

Constants are `core::Value` directly. The alternative — a parallel VM value
model — was rejected: it would duplicate the value hierarchy the issue
explicitly forbids duplicating, and `Value` is already portable across native
and `wasm32` (it is the same type the wasm browser build uses).

## 3. Instruction encoding

A typed Rust enum, not packed bytes:

```rust
pub enum Instruction {
    Constant(u32),                    // push constants[i]
    Nil,                              // push Value::Nil
    True,                             // push Value::Bool(true)
    False,                            // push Value::Bool(false)
    LoadLocal(u16),                   // push locals[slot]
    StoreLocal(u16),                  // pop into locals[slot]
    Pop,                              // discard top
    Primitive { op: Primitive, argc: u8 }, // pop argc, push result
    Jump(u32),                        // ip = target
    JumpIfFalse(u32),                 // pop; if not truthy, ip = target
    Return,                           // return top of stack (height must be 1)
}
```

Deviation from the issue's suggested surface, deliberately: instead of ten
binary arithmetic/comparison opcodes there is one variadic
`Primitive { op, argc }` instruction. Hara's `+ - * / % mod = < <= > >=` are
variadic (`(+ 1 2 3)`, `(< 1 2 3)`) with defined fold order and exact error
messages ("integer overflow", "division by zero", "+ expects numbers",
"< expects at least two arguments"). Folding variadic calls into binary
chains in the compiler would duplicate that behavior (and get `(< 1 2 3)`
wrong without extra short-circuit machinery). `Primitive` pops `argc` values
and hands them to the shared `core::apply_primitive` boundary (§8), so the VM
inherits the exact semantics. `argc: u8` caps calls at 255 arguments; the
compiler rejects larger arities.

A packed-byte encoding is a later optimization; the typed enum keeps
validation and disassembly exact and does not preclude a `&[u8]` view later.

## 4. Operand stack behavior

- One operand stack per machine (per frame in later milestones).
- Every instruction has a statically known stack effect:
  push +1 (`Constant`, `Nil`, `True`, `False`, `LoadLocal`), pop −1 (`Pop`,
  `StoreLocal`, `JumpIfFalse`), net `1 − argc` (`Primitive`), 0 (`Jump`),
  terminal (`Return`).
- `JumpIfFalse` consumes the condition; both `if` branches then produce
  exactly one value, so control-flow joins are height-consistent by
  construction.
- The machine runs with a `Vec<Value>` stack and never allocates per
  instruction; primitive arguments are gathered into a reused scratch buffer.
- The validator computes the exact stack height at every instruction and
  verifies the declared `max_stack` (§9).

## 5. Lexical slot allocation

Locals are numeric slots in a fixed-size `Vec<Value>` allocated at frame
entry (initialized to `Nil`). No string-keyed maps at runtime.

- The compiler keeps a scope stack. Each `let`/`loop` pushes a scope; every
  binding name maps to a fresh slot.
- Slots are monotonically allocated while a scope is open and freed (the
  high-water counter rewinds) when the scope closes, so sibling scopes reuse
  slots. `local_count` is the maximum simultaneously live slots.
- Shadowing allocates a new slot in the inner scope; name resolution searches
  scopes innermost-first, so the inner binding wins and the outer slot is
  untouched.
- `let` initializers compile in order; each name enters the scope only after
  its initializer is compiled, so a later initializer observes earlier
  bindings (current Hara behavior) but not later ones.
- Scope restore is compile-time only (pop the scope stack); the runtime does
  no environment save/restore, unlike the current evaluator's
  `Rc<RefCell<HashMap>>` cloning.
- Destructuring binding patterns are out of scope: a non-symbol binding name
  is a compile error.

## 6. Branch and jump semantics

- Jump operands are absolute instruction indexes (`u32`), patched by the
  compiler once target positions are known.
- `JumpIfFalse` pops the condition and branches when the value is not truthy.
  Truthiness is exactly `Value::truthy`: only `nil` and `false` are false.
- The validator rejects out-of-range targets and targets that would land
  with inconsistent stack heights (§9).

`if` lowering:

```text
  <condition>
  JumpIfFalse Lelse
  <then>
  Jump Lend
Lelse:
  <else>          ; or `Nil` when the else branch is missing
Lend:
```

`(if c t)` compiles the missing else as `Nil`, matching the evaluator.

## 7. `loop/recur` compilation

`loop` compiles like `let` (ordered binding initializers into fresh slots),
then records a loop context: the header instruction index (first body
instruction) and the binding slot list. Multiple body forms sequence like
`do` — matching the current evaluator, which wraps `body+` in `do` — and
the last form compiles in the loop's tail position.

`recur` with `n` arguments against loop slots `[s0 .. sn-1]`:

```text
  <arg0> <arg1> ... <argN-1>     ; all evaluated before any store
  StoreLocal sN-1                ; stores in reverse order
  ...
  StoreLocal s0
  Jump header
```

Because every argument is evaluated (loaded) before the first `StoreLocal`,
simultaneous-recurrence semantics hold: a new value never observes a
partially updated binding set. Reverse-order storing is what makes the
sequence correct when arg evaluation itself reads loop slots.

Compile-time rejections (mirroring `hal-langspec.edn` `:eval/recur-tail`,
which states recur is valid only in tail position with matching arity):

- `recur` with no enclosing loop: "recur must be inside loop".
- arity mismatch: "loop recur arity mismatch".
- `recur` not in tail position of its loop body: compile error. Tail
  positions are: the loop body, the branches of a tail `if`, the last form of
  a tail `do` or `let` body. Everything else (initializers, conditions,
  primitive arguments, non-final `do` forms) is non-tail.
- `recur` is compiled to jumps, never to a `Value::Recur` payload — the VM
  does not represent recur as a value or exception.

This is a deliberate, spec-aligned tightening over the current evaluator,
which detects some misuse only at runtime; both paths error, and the
differential tests compare error categories (§15).

## 8. Primitive dispatch

Arithmetic and comparison semantics are **not** reimplemented in the VM.
`core.rs` gains a small value-level boundary, shared with the existing
evaluator:

```rust
pub enum Primitive { Add, Subtract, Multiply, Divide, Remainder,
                     Equal, Less, LessOrEqual, Greater, GreaterOrEqual }

pub(crate) fn apply_primitive(primitive: Primitive, arguments: &[Value])
    -> Result<Value, String>;
```

The existing `arithmetic`/`comparison`/`=` arms of `core::eval` are re-pointed
at `apply_primitive` after evaluating their argument forms, so there is one
implementation of:

- i64-only arithmetic with `checked_*` ops ("integer overflow",
  "division by zero", "{op} expects numbers", "{op} expects arguments");
  `mod` and `%` share the `%` operator spelling in error messages;
- chained variadic comparisons ("< expects at least two arguments");
- `=` via the existing `PartialEq for Value` ("= expects at least 2 arguments").

The machine pops `argc` values into a scratch buffer and calls
`apply_primitive` directly. No forms are cloned, no temporary symbols or
environments are built.

## 9. Validation rules

`validate(&Program)` runs before any execution and rejects:

- empty code, or code that does not end in an executed `Return`;
- `Constant` indexes outside `constants`;
- `LoadLocal`/`StoreLocal` slots outside `local_count`;
- jump targets outside the code vector;
- any instruction unreachable from index 0 (the compiler emits no dead code;
  unreachable code in a hand-built program is malformed);
- stack underflow along any path;
- inconsistent stack heights at control-flow joins (every instruction must
  have one unique height);
- `Return` at a stack height other than exactly 1;
- programs exceeding defined limits: `MAX_CONSTANTS` (2^24),
  `MAX_INSTRUCTIONS` (2^24), `MAX_LOCALS` (2^16 − 1, inherent to `u16`),
  `MAX_OPERAND_STACK` (4096);
- a declared `max_stack` that disagrees with the computed high-water mark.

Validation is a single abstract-interpretation pass carrying stack heights
across a worklist of instruction indexes. After it passes, the machine
indexes without re-checking; malformed programs produce `ValidationError`,
never panics.

## 10. Source maps and diagnostics

The compiler works on `kernel::parser::read_forms` output (`SpannedForm`)
and records the `Position` (offset, line, column) of the originating form
for every emitted instruction. `SourceMap` is a parallel
`Vec<Option<Position>>` indexed by instruction offset.

- Compile errors carry the form's position and render like parse errors:
  `message [line L, column C]`.
- Runtime errors (`VmError`) carry the failing instruction index and its
  source position, rendered the same way.
- The disassembler prints offsets, operands, constant previews, jump
  destinations, and source positions deterministically.

## 11. Native/WASM constraints

- No new dependencies. The VM uses `core::Value`, `kernel::Form`, and the
  parser — all already wasm-compatible.
- No `unsafe`, no host-specific machinery, no threads, no floating-point
  reinterpretation beyond what `Value` already does.
- All indexing safety comes from the validator; the machine uses checked
  indexing that converts validator-covered failures into `VmError`s rather
  than panicking.
- Verified with `cargo build --target wasm32-unknown-unknown --features
  bytecode-vm --lib`.

## 12. Closures and upvalues (delivered in milestone 2)

Milestone 2 (§17) delivered function values, by-value captures, and
direct/static calls without upvalue load/store instructions: captures are
copied into prefixed local slots at closure creation, so the existing
`LoadLocal`/`StoreLocal` machinery covers them.

- `Program.functions` holds one prototype per `fn`/`defn` body;
  `closure.rs` remains deliberately absent (no separate upvalue
  representation was needed).
- `Frame` (locals + stack base + return address) gives the machine a call
  stack; `Closure` values wrap a nested `Machine` through the shared
  `core::native_function` boundary.
- `recur` across a function boundary stays rejected; the loop context stack
  is per-prototype.

## 13. Future exception handling

`try/catch/finally` needs protected ranges on the code vector: a handler
table (`Vec<(start, end, handler_ip)>`) per prototype, plus stack-unwind
logic in the machine. The source map and validator already treat the code
vector as the single source of truth, so the table attaches cleanly. Out of
scope here; the machine's error type is designed to carry the instruction
index a handler table would need.

## 14. Future suspension and resumability

`VmOutcome` is the seam:

```rust
pub enum VmOutcome { Returned(Value), Failed(VmError) }
```

Later milestones add `Suspended(continuation)` variants. Because the machine
state is plain data (ip, stack, locals, frame), suspending means serializing
or parking that state — no CPS transform of the instruction set is needed.
The dispatch loop is a `loop { match ... }` that can return at any point, so
adding suspension does not rewrite instruction dispatch, only adds exit
points. This mirrors how `fiber.rs` already separates `Step` state from
driving.

## 15. Coexistence with the current evaluator

- The VM is additive: new `rust/src/vm.rs` + `rust/src/vm/*` modules, a
  `bytecode-vm` feature (non-default), and feature-gated free functions:

  ```rust
  pub fn compile_bytecode(source: &str) -> Result<Rc<vm::Program>, String>;
  pub fn execute_bytecode(program: &Rc<vm::Program>) -> Result<String, String>;
  pub fn eval_bytecode_native(source: &str) -> Result<String, String>;
  ```

  Programs are returned inside `Rc` because compiled closures share the
  program with their executing machines.
- `Runtime::eval_native` and the fiber/`core::eval` path are unchanged in
  behavior. The only `core.rs` edit is extracting `apply_primitive` (§8),
  which the existing evaluator also calls — semantics shared, not forked.
- `eval_bytecode_native` accepts only closed, namespace-independent forms:
  literals, lexical locals, the ten primitives, `if`, `do`, `let`,
  `loop/recur`, `fn` values with by-value captures, calls to function
  values, `defn` as a lowered top-level statement, `()` (nil), metadata
  passthrough, and big-integer/decimal/regex literals as constants.
  Everything else (symbols that are not locals or lowered defns, `def`,
  `quote`, collections as runtime constructors — deferred, protocols,
  `try`, promises, namespaces) is a typed compile error. No silent
  fallback.
- Differential tests run each supported source through `Runtime::eval_native`
  and the VM and compare displayed values, or normalized error categories
  when the two paths legitimately phrase errors differently (compile-time vs
  runtime detection of recur misuse).

## 16. Conditions required before making the VM the default

Not in this milestone. Minimum bar for a later default-on discussion:

1. ~~Closures/upvalues~~ (milestone 2), multi-arity calls, and namespace
   interop compiled, so real programs (not just closed arithmetic) run.
2. Differential parity over the L0 conformance corpus
   (`specs/language/draft/conformance/l0.edn`), not only the milestone
   subset.
3. Exception and suspension stories (§13, §14) implemented or proven
   unnecessary per call site.
4. Execute-only benchmarks showing a real win over the fiber evaluator on
   the `lib/bench/runtime/workloads.json` corpus, with compile cost
   amortized by caching.
5. A fallback strategy for forms the VM still rejects, decided explicitly
   (hybrid dispatch vs full coverage).

## Open decisions recorded at milestone 1

- `Primitive { op, argc }` instead of ten binary opcodes (§3).
- `recur` misuse is a compile error, not a runtime error (§7) — matches the
  langspec, differs in phrasing from the current evaluator.
- Constant pool stores `Value` directly (§2) — no duplication of the value
  model.
- Collection literals deferred even though the evaluator supports them;
  adding them later is additive (`Vector`, `Map`, `Set` construction
  instructions or primitives) and does not change this milestone's
  instruction semantics.

## 17. Milestone 2 — closures, calls, and defn lowering

Scope (GitHub issue #202): `fn` values, lexical captures, direct calls,
and `defn` lowering. Still synchronous; still feature-gated; exceptions
(#203) and suspension (#204) remain future work.

### 17.1 Instruction additions

```rust
Closure { prototype: u32, captures: u8 }, // pop captures, push fn value
Call { argc: u8 },                        // pop argc args + callee, push result
CallStatic { prototype: u32, argc: u8 },  // pop argc args, push result
```

`FunctionPrototype` gains `capture_count`; capture slots are pre-allocated
as locals `arity .. arity + capture_count - 1`, so a function body reads
its captured environment through ordinary `LoadLocal`s. There are no
upvalue instructions and no `closure.rs`. Validation additionally rejects:
bad closure/callstatic prototype indexes, capture-count mismatches
between the closure instruction and the prototype, callstatic arity
mismatches, and programs exceeding `MAX_CAPTURES` (255).

### 17.2 Compiler: context stack and free-variable pre-pass

The compiler was restructured from one flat function context into a
context stack: constants and the prototype table are shared across the
whole program, while each function under construction gets its own scope
stack, loop contexts, jump patches, and source map. Compiling a nested
`fn` pushes a fresh context; closing it appends a finished prototype.

Captures are computed by a free-variable pre-pass (`collect_free`) over
the `fn` body: symbols referenced but not bound within the function
(params, let's, loops, nested fn params bind; special-form and primitive
operators are not references) resolve against the enclosing context's
scope stack. Each captured name is declared as a slot in the new function
before its body compiles, and the enclosing function emits the matching
`LoadLocal`s before `Closure`. Captures are **by value at closure-creation
time**; nested closures capture through intermediate scopes because the
intermediate function's capture slots are themselves resolvable names.

### 17.3 Machine: calls through the shared function boundary

The machine holds `Rc<Program>` and a `Frame` per active call (locals,
operand-stack base, return address). `Closure` copies the capture values
into a new frame's prefixed slots and wraps the prototype in
`core::native_function`, producing an ordinary `Value::Function` whose
invocation runs a nested machine. `Call` dispatches through the shared
`core::call_function` boundary, so arity errors ("function expects 1
arguments"), `<fn>` display, and non-function callees ("value is not
callable") behave exactly as in the evaluator — the VM does not
reimplement them. `CallStatic` skips the callee value entirely for
compiler-known targets (defn calls and self-recursion).

Consequence: recursion depth is bounded by the host stack (nested
machines), not by the fiber's trampoline. The evaluator reaches depth
10000 on self-recursion; the corpus keeps VM recursion at depth ≤ 1000.
Deep-recursion support belongs to the suspension milestone (#204), where
a real call stack replaces nested machines.

### 17.4 defn lowering

The evaluator's `defn` materializes a Var and mutates the namespace; the
VM has neither. For the closed, namespace-independent subset, `defn`
**lowers to a direct slot binding**, matching the evaluator's observable
early-binding behavior:

- Legal only as a non-final top-level statement (a top-level `do` is
  transparent for statement position). Result-position defn errors with
  "defn in result position requires var semantics"; nested defn with
  "defn is only supported as a top-level statement".
- The name binds a slot holding the function value; calls compile to
  `CallStatic`. Self-recursion compiles to a `CallStatic` self-call.
- Redefinition shadows the earlier binding (early binding means later
  forms see the new slot; already-compiled bodies keep the old one),
  matching the evaluator.
- Forward references fail as "unbound symbol", matching the evaluator.
- Referencing the defn'd name as a value inside its own body errors with
  "defn self-reference in value position is not supported"; `#'f` remains
  "unsupported operator: var".
- Variadic (`&`) and multi-arity `fn`/`defn` and destructuring parameters
  are typed compile errors.

### 17.5 Documented divergence: builtin-named defn

Under the evaluator's early binding, a call to a builtin-named symbol
resolves to the builtin even after a same-named `defn` — the new var
never shadows the primitive (`(do (defn count [n] 42) (count 5))` fails
with "count expects a collection"). The VM's lowered slot shadows the
builtin for all subsequent forms. Both behaviors are internally
consistent; the VM's matches lexical shadowing intuition, the evaluator's
is a resolution-order quirk. Recorded in
`differential_tests.rs::defn_builtin_collision_is_a_documented_divergence`.

### 17.6 Error surface additions

New corpus error categories (pinned in
`specs/runtime/draft/conformance/bytecode-vm.edn`): "function arity",
"not callable", "fn params shape". Call arity and callability errors are
runtime errors carrying instruction index and source position; shape and
lowering errors are compile errors.

### 17.7 Benchmarks (milestone 2, arm64, rustc 1.97.1)

Median per call, 15 windows; raw JSON in `rust/target/vm-bench/`
(gitignored scratch). Workloads from `lib/bench/runtime/workloads.json`
exactly as written; every call checks the expected checksum.

| workload      | existing evaluator | compile+execute | execute-only | speedup (exec-only) |
|---------------|-------------------:|----------------:|-------------:|--------------------:|
| noop          |          1.914 ms  |        534 ns   |      51 ns   | ~37 000×            |
| arithmetic    |         30.6 ms    |        540 µs   |     534 µs   | ~57×                |
| function-call |        456.7 ms    |        767 µs   |     752 µs   | ~607×               |

Raw samples (ns):

- noop existing: [1722945,1733115,1748736,1766958,1888640,1903075,1933851,1911112,1913998,1934482,1932137,1930312,1950960,1962385,1950302]
- noop compile+execute: [527,534,546,572,553,536,553,547,534,501,506,504,506,500,467]
- noop execute-only: [57,63,59,58,62,56,51,52,50,46,46,42,41,41,41]
- arithmetic existing: [30615154,30325808,30640858,31016868,30497300,30337308,30821316,30297943,32835316,31176785,30640297,30590585,30284041,30049922,30439431]
- arithmetic compile+execute: [535164,550964,535602,521483,524033,561981,542704,531975,526400,543912,563850,529362,543122,543381,539710]
- arithmetic execute-only: [529389,533575,522502,534675,532400,526202,541958,532572,533679,575262,551841,538239,539420,536208,552831]
- function-call existing: [466815722,446421194,456703347,467016264,474069819,505645514,481448069,455694750,443059583,442556402,439168236,457404764,463732375,446484319,445612847]
- function-call compile+execute: [855777,808291,754055,756166,768958,783194,760013,762250,756472,764902,767291,802930,793944,774861,755972]
- function-call execute-only: [864750,783527,752402,754819,765805,782000,738125,736416,766416,737486,750597,747889,759013,765527,729097]

Interpretation:

- The machine refactor (frames, `Rc<Program>`, capture pre-pass) did not
  regress the milestone-1 loop path: arithmetic execute-only holds at
  ~534 µs (~57× over the evaluator).
- Function calls are where the evaluator's costs (environment cloning,
  boxed continuations, form reconstruction) concentrate: 2500 nested
  `((fn [x] (+ x 1)) acc)` calls per loop take ~457 ms there vs ~752 µs
  here — ~600×. Compile cost is noise at this workload size
  (compile+execute ≈ execute-only), so for closed hot code, caching a
  compiled program is not yet the deciding factor; execution is.
- No performance threshold is asserted in CI; these are evidence for the
  issue, not gates.

## Open decisions recorded at milestone 2

- Captures as prefixed local slots instead of an upvalue representation
  (§17.1) — simpler validator and machine; revisit only if mutation of
  shared captured state is ever required (Hara values are persistent, so
  by-value capture matches language semantics).
- `Closure` values wrap nested machines through `core::native_function`
  (§17.3) — maximal reuse of the shared call boundary at the cost of a
  host-stack recursion bound; the suspension milestone replaces this with
  an explicit call stack.
- defn lowering instead of Var materialization (§17.4) — closed-world
  subset only; namespace-interop programs still need the evaluator.
