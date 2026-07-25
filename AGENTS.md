# AGENTS.md

Repo layout and per-component build/test commands. See `README.md` for the
component map and `website/docs/development.md` for the developer guide.

## Layout

- `java/` — Java/Truffle runtime (Maven, JDK 21)
- `rust/` — Rust/embedding runtime (native CLI, wasm builds, web loader,
  `rust/extensions/` in-tree wasm extensions). The old `wasm/` tree was
  removed — never reference it; everything is `rust/`. `rust/web/` holds the
  browser loaders plus the shared studio environment (`rust/web/studio/`,
  mounted by the website studio page and the hara-chrome panel).
- `lib/` — hara-language sources (`lib/src`, `lib/test`), examples
  (`lib/examples/`), benchmarks (`lib/bench/`)
- `apps/` — `hara-chrome`, `hara-vscode`, `hara-emacs`, `hara-lsp` (planned)
- `website/` — the published site, content included: `website/docs/` is the
  mkdocs docs_dir; infra (mkdocs.yml, overrides/, landing page) alongside.
  Apps and books join the site as monorepo sub-sites (see below).
- `docs/` — working documents, NOT published: design notes and
  `docs/superpowers/` (plans/specs written by the superpowers plugin).
  Put nothing here that belongs on the website.
- `spec/hara/` — normative specs:
  - `*.md` — prose specs; mirrored to `website/docs/reference/` (kept in
    sync by `hara.spec.DocumentationContractTest`)
  - `corpora/*.edn` — machine-checked conformance/parity corpora (consumed
    by Java and Rust test suites via repo-relative paths)
  - `data/` — spec-shaped data (`foundation.edn`, symbol tables)
- `books/`, `registry/` — book series and the planned extension registry
- `archive/` — legacy material, kept for history only

## Build and test

Java/Truffle runtime:

```shell
mvn -f java/pom.xml -Ptruffle package        # build + full test suite
mvn -f java/pom.xml -Ptruffle -Dtest=hara.truffle.HaraL0ConformanceTest test
./hara eval '(+ 19 23)'                      # CLI smoke test (shaded jar)
```

Rust runtime:

```shell
cargo test --manifest-path rust/Cargo.toml
cargo test --manifest-path rust/raw/Cargo.toml
bash rust/scripts/check-layout.sh
bash scripts/build-hara-wasm-raw             # raw wasm extension artifact
cd rust/web && npm ci && npm run test:hta    # browser loader tests
cd rust/web && npm run test:studio           # studio node tests (broker, hal, UI)
```

The `studio-hal` and `studio-broker` real-wasm integration tests need the
raw wasm artifact from `bash scripts/build-hara-wasm-raw` and self-skip
without it.

Apps:

```shell
cd apps/hara-chrome && npm ci && npm run build && npm test
cd apps/hara-chrome && npm run test:browser  # playwright (needs xvfb)
```

Website:

```shell
pip install -r website/requirements-docs.txt
mkdocs build --strict -f website/mkdocs.yml
```

## Adding an app/book sub-site

1. Create `<dir>/mkdocs.yml` (with `site_name` and `nav`) and `<dir>/docs/`.
2. Add to `website/mkdocs.yml` nav:
   `- <Title>: '!include ../<dir>/mkdocs.yml'` (path relative to
   `website/mkdocs.yml`).
3. Verify with `mkdocs build --strict -f website/mkdocs.yml`.

## Conventions

- Maven runs from the repo root via `-f java/pom.xml`; Surefire's working
  directory is the repo root, so tests use repo-relative paths
  (`spec/hara/corpora/...`, `lib/examples/...`, `website/docs/...`).
- The JVM runtime embeds `lib/src/**/*.hal` (std foundation) as classpath
  resources via `java/pom.xml`; the Rust runtime embeds
  `lib/src/std/lib/foundation.hal` via `include_str!` in `rust/src/lib.rs`.
- `target/` at the repo root is CI scratch/build artifacts; Maven output is
  `java/target/`. Both are gitignored.
- The pages deploy (`.github/workflows/pages.yml`) also ships the raw HTA
  studio artifacts under `site-build/rust/`: the raw wasm module, the HTA
  loader, and `rust/web/studio/` (broker, host services, boot, UI, hal libs)
  for the website studio page.
- IDE state (`.idea/`, `.settings/`, `.classpath`, `.project`) is user-local
  and untracked.
