# Cross-runtime benchmark

`workloads.json` is the canonical corpus. Every adapter receives the exact same
`source` string and validates its displayed result before reporting timing.

```shell
scripts/run-runtime-benchmarks --profile smoke
scripts/run-runtime-benchmarks --profile standard --reference
scripts/run-runtime-benchmarks --profile standard --check-regressions \
  --runtime bb \
  --runtime hara-rust-bytecode \
  --runtime hara-rust-trace-checked \
  --runtime hara-rust-trace-native
```

The runner records process startup, first evaluation, warm-up samples,
convergence, steady-state throughput, peak resident memory when available, and
runtime payload size. Workloads with an `iterations` field also report normalized
nanoseconds per iteration.

The three Rust execution tiers are built into separate target directories. This
prevents a later Cargo feature build from silently changing the binary measured
by an earlier adapter:

- `hara-rust-bytecode` — plain bytecode VM;
- `hara-rust-trace-checked` — guarded checked Trace IR;
- `hara-rust-trace-native` — guarded Wasmtime/Cranelift trace backend.

Absolute performance values are machine-specific evidence. The optional rules
in `regression-baselines.json` compare runtimes measured in the same invocation,
using ratios rather than historical wall-clock values. They are intended for
the standard profile; smoke results are diagnostic only.
