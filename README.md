# hara.lang

Hara is a small, runtime-neutral language for live systems. The current supported language
implementation is the Truffle runtime: it provides a compact L0 core, persistent data, explicit
mutable `array`/`object` markers, protocols, promises, bytes, capability-gated I/O, and a JLine REPL.

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
- [HAL meta-spec](specs/metaspec/draft/README.md) — the shape and authority rules for language specifications.
- [HAL language draft](specs/language/draft/README.md) — the portable language contract and executable evidence.
- [Planning archive](specs/archive/planning/README.md) — earlier runtime, extension, interop, and tooling designs.
- [Hara for Emacs](extensions/hara-emacs/README.md) — project-aware evaluation, sessions, completion, docs,
  and a RESP-backed REPL.

## Quick start

Requirements: JDK 21 and Maven.

```shell
mvn -f java/pom.xml -Ptruffle package
./hara eval '(+ 19 23)'
./hara
```

The `hara` command starts the JLine REPL in the shared `ROOT` session and exposes that same
session through RESP on `127.0.0.1:1311`. Use `--offline` to start without the listener,
`headless` for a listener without terminal UI, and `remote HOST:PORT` for a client connection. The CLI also supports `run <file>`, `stdin`, and `help`. For a native-image build, see the
[developer guide](docs/docs/development.md); native mode intentionally removes dynamic JVM services.

Per-component builds:

```shell
cargo test --manifest-path rust/Cargo.toml                 # Rust runtime
cd extensions/hara-chrome && npm ci && npm run build       # Chrome extension
cd docs && mkdocs build -f mkdocs.yml                      # docs site
```

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
