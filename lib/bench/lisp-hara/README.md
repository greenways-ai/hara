# lisp-hara benchmark

Balanced application-kernel comparison of the Hara Rust runtime tiers against
**SBCL**, **Chez Scheme**, **GNU Guile**, and **LuaJIT**. The original tiny
microbenchmarks remain in `lib/bench/runtime/`; this corpus exercises mutable
arrays, mutable objects, persistent data, recursion, and mixed control flow.

## Requirements

```shell
brew install sbcl chezscheme guile
```

Plus the hara benchmark binaries (built automatically unless `--no-build`):

- `rust/target/release/hara-runtime-benchmark`
- `target/runtime-benchmark/{vm,trace-checked,trace-native}/release/hara-bytecode-benchmark`

## Usage

```shell
python3 lib/bench/lisp-hara/run.py                      # smoke profile, all runtimes
python3 lib/bench/lisp-hara/run.py --profile standard   # full sampling
python3 lib/bench/lisp-hara/run.py --runtime sbcl-prepared --runtime hara-rust-trace-native-prepared
```

Runtime names carry an explicit `-eval` or `-prepared` lane. Compare only
within a lane. Hara's tree evaluator participates in `-eval`; bytecode and
trace tiers participate in both compile/execute and prepared execution.

Results default to `target/lisp-hara-benchmark.{json,md}` (gitignored
scratch — comparison evidence, not regression gating).

## Semantics

Mirrors the luajit-hara suite so numbers are comparable across suites:

- `workloads.json` carries per-language source fields (`hara_source`,
  `scheme_source`, `cl_source`) plus a shared `expected` checksum.
- `-eval` parses/loads and evaluates source on every measured call.
- `-prepared` reads and compiles/loads once, then invokes repeatedly.
- Mutable table rows use Hara `object`/`array` (through their canonical
  native calls), Scheme/Common Lisp hash tables and vectors, and Lua tables.
  Persistent transformations are named and reported separately.
- Unsupported runtime/workload combinations are retained in the feature
  coverage table with their error; they are never silently substituted.
- Lisp sources are hand-written **untyped** idiomatic equivalents (no
  SBCL type declarations or `optimize` declarations, portable R6RS-ish
  Scheme) — an implementation snapshot, not a source-normalized shootout.
- Scheme runners time with wall clock (Chez `current-time`) / run time
  (Guile `get-internal-run-time`); SBCL uses `get-internal-run-time`
  (CPU time, like the Lua runner's `os.clock`).

## Files

- `workloads.json` — the shared corpus
- `chez_runner.scm`, `guile_runner.scm`, `sbcl_runner.lisp` — per-runtime
  runners implementing the `ID SOURCE_HEX EXPECTED WINDOWS CALLS` contract
- `run.py` — the coordinator (windowed sampling, steady-state median,
  JSON + Markdown output)
