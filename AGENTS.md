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
  (`lib/examples/`), benchmarks (`lib/bench/`). Notable namespaces:
  `std.foundation`, the `talo.*` compiler port, and the `std.ledger.*`
  consensus-free executable-chain experiments.
- `extensions/` — `hara-chrome`, `hara-vscode`, `hara-emacs`, `hara-lsp` (planned)
- `website/` — the published site, content included: `website/docs/` is the
  mkdocs docs_dir; infra (mkdocs.yml, overrides/, landing page) alongside.
  Apps and books join the site as monorepo sub-sites (see below).
- `notes/` — working documents, NOT published: design notes and
  `notes/superpowers/` (plans/specs written by the superpowers plugin).
  Put nothing here that belongs on the website.
- `specs/` — normative specifications:
  - `01-language-meta/000-metaspec/draft/hal-metaspec.edn` defines the self-describing metaspec contract and AI generation/repair workflow
  - `01-language-meta/001-language/metaspec/language-metaspec.edn` defines the language-spec document model
  - `00-unsorted/artifact/metaspec/artifact-metaspec.edn` defines artifact-format specs
  - `01-language-meta/001-language/draft/hal-langspec.edn` defines the small EDN-oriented data and reader contract
  - `00-unsorted/` holds platform, ecosystem, user-space, and broad execution material awaiting classification
  - `99-archive/planning/` contains non-normative historical source material
- `contrib/` — separately owned specifications and reference implementations.
  Greenways formats live under `contrib/greenways/`; Hara may publish verified
  snapshots but does not own their domain vocabulary.
- `books/`, `registry/` — book series and the planned extension registry
- `archive/` — legacy source repository, distinct from `specs/99-archive/`

## Build and test

Java/Truffle runtime:

```shell
mvn -f java/pom.xml -Ptruffle package        # build + full test suite
mvn -f java/pom.xml -Ptruffle -Dtest=hara.truffle.HaraL0ConformanceTest test
./hara eval '(+ 19 23)'                      # CLI smoke test (shaded jar)
./scripts/run-lib-tests                      # library .hal test harness
bash scripts/build-truffle-native            # native-image build (target/hara-truffle)
target/hara-truffle eval '(+ 19 23)'         # native-image smoke test
```

Rust runtime:

```shell
cargo test --manifest-path rust/Cargo.toml
cargo test --manifest-path rust/raw/Cargo.toml
bash rust/scripts/check-layout.sh
bash scripts/build-hara-wasm-raw             # raw wasm extension artifact
bash scripts/build-hara-wasm-web             # browser runtime → website/docs/rust/pkg/
bash scripts/build-demo-synth-wasm           # demo synth → website/docs/assets/wasm/
cd rust/web && npm ci && npm run test:hta    # browser loader tests
cd rust/web && npm run test:studio           # studio node tests (broker, hal, UI)
```

The `studio-hal` and `studio-broker` real-wasm integration tests need the
raw wasm artifact from `bash scripts/build-hara-wasm-raw` and self-skip
without it.

Apps:

```shell
cd extensions/hara-chrome && npm ci && npm run build && npm test
cd extensions/hara-chrome && npm run test:browser  # playwright (needs xvfb)
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

## Releasing the hara CLI

`scripts/install.sh` is the user-facing installer (`curl | sh`); it downloads
prebuilt binaries from GitHub releases. Test it locally with
`sh scripts/test-install.sh` (needs
`cargo build --release --manifest-path rust/Cargo.toml --bin hara` first).

To cut a release:

1. Bump `version` in `rust/Cargo.toml` and commit.
2. `git tag vX.Y.Z && git push origin vX.Y.Z` (tag version must match
   `rust/Cargo.toml`).
3. `.github/workflows/release.yml` builds Linux x86_64 + macOS arm64/x86_64
   binaries, publishes the GitHub release (prerelease while 0.x), and
   smoke-tests `scripts/install.sh` against it.

## Conventions

- Maven runs from the repo root via `-f java/pom.xml`; Surefire's working
  directory is the repo root, so tests use repo-relative paths
  (`specs/00-unsorted/platform-language/draft/conformance/...`, `lib/examples/...`, `website/docs/...`).
- The JVM runtime embeds `lib/src/**/*.hal` (std foundation) as classpath
  resources via `java/pom.xml`; the Rust runtime embeds
  `lib/src/std/foundation.hal` via `include_str!` in `rust/src/lib.rs`.
- `target/` at the repo root is CI scratch/build artifacts; Maven output is
  `java/target/`. Both are gitignored.
- The pages deploy (`.github/workflows/pages.yml`) also ships the raw HTA
  studio artifacts under `site-build/rust/`: the raw wasm module, the HTA
  loader, and `rust/web/studio/` (broker, host services, boot, UI, hal libs)
  for the website studio page.
- IDE state (`.idea/`, `.settings/`, `.classpath`, `.project`) is user-local
  and untracked.
