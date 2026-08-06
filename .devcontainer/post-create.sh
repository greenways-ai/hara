#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

# The conformance corpus and developer documentation are required by the
# normal test commands. Keep this narrower than --recursive so an unrelated
# application submodule cannot prevent the core environment from starting.
git submodule update --init specs docs

rustup target add wasm32-unknown-unknown wasm32-wasip1

python3 -m venv .venv
.venv/bin/python -m pip install --disable-pip-version-check \
  -r docs/requirements-docs.txt

npm --prefix core/rust/web ci

cat <<'EOF'

Hara cloud environment is ready.

  Java:  mvn -f core/java/pom.xml -Ptruffle package
  Rust:  cargo test --manifest-path core/rust/Cargo.toml
  Web:   npm --prefix core/rust/web run test:studio
  Docs:  .venv/bin/mkdocs serve -f docs/mkdocs.yml -a 0.0.0.0:8000

EOF
