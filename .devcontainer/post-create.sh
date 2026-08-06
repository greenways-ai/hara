#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

# The conformance corpus and developer documentation are required by the
# normal test commands. They live in sibling repositories that are cloned
# inside this repo for the dev container.
if [ ! -d hara-specs-registry ]; then
  git clone --depth 1 https://github.com/hara-lang/hara-specs-registry.git hara-specs-registry
fi
mkdir -p website
if [ ! -d website/hara-www ]; then
  git clone --depth 1 https://github.com/hara-lang/hara-www.git website/hara-www
fi

rustup target add wasm32-unknown-unknown wasm32-wasip1

npm --prefix core/rust/web ci

cat <<'EOF'

Hara cloud environment is ready.

  Java:   mvn -f core/java/pom.xml -Ptruffle package
  Rust:   cargo test --manifest-path core/rust/Cargo.toml
  Web:    npm --prefix core/rust/web run test:studio
  Docs:   npm --prefix website/hara-www run dev

EOF
