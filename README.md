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

- [`java/`](java/) — the Java/Truffle runtime (Maven project, CLI, native-image).
- [`rust/`](rust/) — the Rust/embedding runtime: native CLI, wasm builds, web
  loader, and in-tree wasm extensions (`rust/extensions/`).
- [`lib/`](lib/) — hara-language source and workloads: the std foundation and
  Polis compiler port (`lib/src`, `lib/test`), demo projects
  ([`lib/examples/`](lib/examples/)), and benchmark suites
  ([`lib/bench/`](lib/bench/)).
- [`apps/`](apps/) — editor and browser apps:
  [`hara-chrome`](apps/hara-chrome/) (Chrome DevTools extension),
  [`hara-vscode`](apps/hara-vscode/), [`hara-emacs`](apps/hara-emacs/), and the
  planned [`hara-lsp`](apps/hara-lsp/) language server.
- [`website/`](website/) — the published site: mkdocs config, theme
  overrides, landing page, and the documentation content itself in
  [`website/docs/`](website/docs/). Apps and books plug in as sub-sites
  (monorepo plugin).
- [`docs/`](docs/) — working documents (not published): design notes and
  `docs/superpowers/` plans/specs.
- [`spec/`](spec/hara/) — normative specs: prose (`.md`), machine-checked
  corpora ([`spec/hara/corpora/`](spec/hara/corpora/)), and spec-shaped
  data ([`spec/hara/data/`](spec/hara/data/)).
- [`books/`](books/) — planned book series (*The Little Book of HAL*).
- [`registry/`](registry/) — planned hara wasm extension registry.
- [`scripts/`](scripts/) — repo-level build/benchmark scripts.
- [`archive/`](archive/) — legacy material kept for history (old Clojure
  reference sources, one-off refactor scripts, C experiments, legacy docs).

## Start here

- [User guide](website/docs/user-guide.md) — install, run, evaluate, use the REPL, and write Hara.
- [Namespaces and modules](website/docs/namespaces.md) — organize projects, require code, and control aliases.
- [Namespace catalog](website/docs/reference/namespaces.md) — discover every shipped namespace family.
- [Developer guide](website/docs/development.md) — build, test, debug, and contribute.
- [Java API and Javadocs](website/docs/javadocs.md) — public entry points and generated API docs.
- [Language specification](spec/hara/l0-language.md) — normative L0 behavior.
- [Runtime libraries](spec/hara/runtime-libraries.md) — the portable library contract.
- [Rust/WASM mapping](spec/hara/rust-runtime.md) — the cross-runtime value, provider, and conformance design.
- [Extensions](spec/hara/extensions-contract.md) — WASM, manifests, HTA, and capabilities.
- [REPL UX](spec/hara/repl.md) — history, completion, docs, and slash-command design.
- [Hara for Emacs](apps/hara-emacs/README.md) — project-aware evaluation, sessions, completion, docs,
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
[developer guide](website/docs/development.md); native mode intentionally removes dynamic JVM services.

Per-component builds:

```shell
cargo test --manifest-path rust/Cargo.toml          # Rust runtime
cd apps/hara-chrome && npm ci && npm run build      # Chrome extension
cd website && mkdocs build -f mkdocs.yml            # docs site
```

## Current runtime boundary

The language does not expose ambient JVM host interop. JVM reflection, compilation, mutable
classpath access, files, and sockets are explicit capabilities or provider services. This keeps the
core portable to future runtimes such as WASM hosts.

The old interpreter/Foundation/TCP architecture is retained as
[`archive/legacy-docs/README.legacy.md`](archive/legacy-docs/README.legacy.md) for historical
reference only; it is not the current language guide.

## Status

Hara is an active experimental runtime. The L0 slice and focused conformance suites are the source
of truth. Provider discovery and WASM execution are documented contracts with
implementation work still in progress.
