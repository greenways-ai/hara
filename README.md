# hara.lang

Hara is a programmable, runtime-neutral kernel for building, inspecting, and
changing live systems. Programs communicate with the kernel through HAL (Hara
Lisp), an EDN-compatible, host-neutral notation and data format. The current
supported runtime is Truffle: it provides a compact L0 core, persistent data,
explicit mutable `array`/`object` markers, protocols, promises, bytes,
capability-gated I/O, and a JLine REPL.

```text
Hara source
    |
    v
Truffle parser / AST
    |
    +--> runtime-neutral core
    |
    +--> explicit libraries (bytes, promise, file, socket, string)
    |
    +--> host capability boundary
```

## Repository layout

This repository (`hara-lang/hara`) is the workspace. It keeps the language
runtime (`java/`, `rust/`, `lib/`), the landing-page website (`website/`), and
several Git submodules at the root:

- [`java/`](java/) — the Java/Truffle runtime (Maven project, CLI, native-image).
- [`rust/`](rust/) — the Rust/embedding runtime: native CLI, wasm builds, web
  loader, and in-tree wasm extensions (`rust/extensions/`).
- [`lib/`](lib/) — hara-language source and workloads: the std foundation and
  Talo compiler port (`lib/src`, `lib/test`), demo projects
  ([`lib/examples/`](lib/examples/)), and benchmark suites
  ([`lib/bench/`](lib/bench/)).
- [`website/`](website/) — the landing page for `www.hara-lang.org`.
- [`docs/`](docs/) — published documentation site (Material for MkDocs). Lives
  in the [`hara-lang/hara-docs`](https://github.com/hara-lang/hara-docs)
  submodule.
- [`extensions/`](extensions/) — editor and browser apps (`hara-chrome`,
  `hara-vscode`, `hara-emacs`, `hara-world`, planned `hara-lsp`). Lives in the
  [`hara-lang/hara-extensions`](https://github.com/hara-lang/hara-extensions)
  submodule.
- [`specs/`](specs/) — normative specs: prose (`.md`), machine-checked corpora,
  and spec-shaped data. Lives in the
  [`hara-lang/hara-specs`](https://github.com/hara-lang/hara-specs) submodule.
- [`contrib/`](contrib/) — independently owned artifact formats developed with
  Hara's metaspec and verifier. Greenways formats live under
  `contrib/greenways/`.
- [`archive/`](archive/) — legacy material kept for history. Lives in the
  [`hara-lang/hara-archive`](https://github.com/hara-lang/hara-archive)
  submodule.
- [`notes/`](notes/) — working documents (not published): design notes and
  `notes/superpowers/` plans/specs.
- [`books/`](books/) — planned book series (*The Little Book of HAL*).
- [`registry/`](registry/) — planned hara wasm extension registry.
- [`scripts/`](scripts/) — repo-level build/benchmark scripts.

## Start here

- [User guide](docs/docs/user-guide.md) — install, run, evaluate, use the REPL, and write Hara.
- [Namespaces and modules](docs/docs/namespaces.md) — organize projects, require code, and control aliases.
- [Namespace catalog](docs/docs/reference/namespaces.md) — discover every shipped namespace family.
- [Developer guide](docs/docs/development.md) — build, test, debug, and contribute.
- [Java API and Javadocs](docs/docs/javadocs.md) — public entry points and generated API docs.
- [HAL meta-spec](specs/01-lang/000-metaspec/draft/README.md) — the self-describing contract for metaspec documents.
- [HAL language draft](specs/01-lang/001-language/draft/README.md) — the small EDN-oriented data and reader contract.
- [Planning archive](specs/99-archive/planning/README.md) — earlier runtime, extension, interop, and tooling designs.
- [Hara for Emacs](extensions/hara-emacs/README.md) — project-aware evaluation, sessions, completion, docs,
  and a RESP-backed REPL.

## Quick start

Install the native Rust CLI with Homebrew:

```shell
brew install hara-lang/tap/hara
hara eval '(+ 19 23)'
```

The separately packaged Truffle native image is available as
`brew install hara-lang/tap/hara-truffle`. Neither Homebrew formula requires a
JVM at runtime.

To build the Truffle runtime from source, install JDK 21 and Maven:

```shell
mvn -f java/pom.xml -Ptruffle package
./hara eval '(+ 19 23)'
./hara
```

The `hara` command starts the JLine REPL in the shared `ROOT` session and exposes that same
session through RESP on `127.0.0.1:1311`. Use `--offline` to start without the listener,
`headless` for a listener without terminal UI, and `remote HOST:PORT` for a client connection. The CLI also supports `run <file>`, `stdin`, and `help`. For a native-image build, see the
[developer guide](docs/docs/development.md); native mode intentionally removes dynamic JVM services.

The Makefile also mirrors the main repository and CI workflows:

```shell
make java-test java-conformance
make rust-test rust-raw-test rust-layout
make lib-test

make wasm-web
make hta-test
make studio-test

make chrome-build chrome-test
make docs-build
make www-build
```

Run `make web-install` or `make chrome-install` before the corresponding Node
workflows on a fresh checkout. `make check-all` runs the core Java, Rust, raw
WASM, portable library, HTA, and Studio checks. Runtime performance entry
points are available as `runtime-benchmark`, `truffle-benchmark`,
and `parity-benchmark`; each accepts additional arguments
through `ARGS`.

Per-component builds:

```shell
cargo test --manifest-path rust/Cargo.toml                 # Rust runtime
cd extensions/hara-chrome && npm ci && npm run build       # Chrome extension
cd docs && mkdocs build -f mkdocs.yml                      # docs site
```

## Cloud development environment

The repository includes a Dev Container definition for GitHub Codespaces and
VS Code's **Dev Containers: Reopen in Container** command. On first creation it
installs JDK 21 with Maven, Node.js 22, stable Rust with the browser and WASI
targets, Python documentation tooling, the core Git submodules, and the web
test dependencies. No host toolchain setup is required.

The setup is reproducible from the repository root:

```shell
bash .devcontainer/post-create.sh
```

The container forwards the Hara RESP port (`1311`) and the MkDocs preview port
(`8000`). See the commands printed by the setup script for the main checks.

## Current runtime boundary

The language does not expose ambient JVM host interop. JVM reflection, compilation, mutable
classpath access, files, and sockets are explicit capabilities or provider services. This keeps the
core portable to future runtimes such as WASM hosts.

The old interpreter/Foundation/TCP architecture is retained as
[`archive/legacy-docs/README.legacy.md`](archive/legacy-docs/README.legacy.md) for historical
reference only; it is not the current language guide.

## Cloning this workspace

Because several sections are Git submodules, clone with:

```shell
git clone --recurse-submodules https://github.com/hara-lang/hara.git
```

Or, after a normal clone, run:

```shell
git submodule update --init --recursive
```

## Status

Hara is an active experimental runtime. The L0 slice and focused conformance suites are the source
of truth. Provider discovery and WASM execution are documented contracts with
implementation work still in progress.

## License

Hara-owned source code is available under the [Eclipse Public License 2.0](LICENSE).
Some directories contain separately licensed or provenance-sensitive material; see
[the license inventory](LICENSES/README.md). Run `bash scripts/check-licenses` to
validate the repository's license metadata and documented exceptions.
