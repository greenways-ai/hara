# lisp-hara benchmark

Comparison of the hara Rust runtime tiers against other Lisp-family
implementations: **SBCL** (Common Lisp, native compiler), **Chez Scheme**
(native compiler), and **GNU Guile** (bytecode VM). Modelled on
`lib/bench/luajit-hara/` — same corpus shape, same runner contract, same
windowed sampling and steady-state analysis.

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
python3 lib/bench/lisp-hara/run.py --runtime sbcl --runtime hara-rust-trace-native
```

Runtimes: `sbcl`, `chez`, `guile`, `hara-rust-native`,
`hara-rust-bytecode`, `hara-rust-trace-checked`, `hara-rust-trace-native`.

Results default to `target/lisp-hara-benchmark.{json,md}` (gitignored
scratch — comparison evidence, not regression gating).

## Semantics

Mirrors the luajit-hara suite so numbers are comparable across suites:

- `workloads.json` carries per-language source fields (`hara_source`,
  `scheme_source`, `cl_source`) plus a shared `expected` checksum.
- Every runner re-parses and re-evaluates the workload source on every
  call, matching hara's `eval_native` per-call semantics. SBCL evals in
  `:compile` mode (its default), Chez compiles eval'd forms, Guile
  compiles eval'd forms to bytecode — so per-call cost includes each
  implementation's compile step, exactly as the LuaJIT suite includes
  `load`.
- The `hara-rust-*` VM tiers compile once and execute only (their
  `first` value is the first execution, not compilation).
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
