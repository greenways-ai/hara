#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
OUT="$ROOT/apps/hara-world/runtime"

cargo build --manifest-path "$ROOT/rust/Cargo.toml" \
  --target wasm32-unknown-unknown --release --lib
mkdir -p "$OUT"
wasm-bindgen --target web --out-dir "$OUT" \
  "$ROOT/rust/target/wasm32-unknown-unknown/release/hara_wasm.wasm"

echo "Built Hara browser runtime in $OUT"
