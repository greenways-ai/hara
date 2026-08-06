#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

# The conformance corpus and developer documentation are required by the
# normal test commands. They live in sibling repositories that are cloned
# inside this repo for the dev container.
if [ ! -d hara-specs ]; then
  # Pin to the last hara-specs commit that still contains the local conformance
  # corpus. Later main reorganised the repo into a specs service/website.
  mkdir hara-specs
  git -C hara-specs init
  git -C hara-specs remote add origin https://github.com/hara-lang/hara-specs.git
  git -C hara-specs fetch --depth 1 origin 3012cbe74edd92b24783f1968a55dcf612c05364
  git -C hara-specs checkout 3012cbe74edd92b24783f1968a55dcf612c05364
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
