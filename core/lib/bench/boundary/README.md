# Boundary benchmarks

This suite reports transport layers separately instead of subtracting one
end-to-end number from another. HTA payloads remain immutable maps/vectors;
Hara `object` and `array` are exercised by the language corpus, not used as
the Hoplite request contract.

```shell
node lib/bench/boundary/run.mjs
node lib/bench/boundary/run.mjs --standard
cargo run --release --manifest-path core/rust/Cargo.toml --bin hara-wasm-boundary-benchmark
```

The raw-Wasm rows self-skip when the artifact from
`bash packaging/scripts/build-hara-wasm-raw` is absent.
