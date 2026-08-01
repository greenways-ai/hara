# `xt.db` → `std.db` file-by-file port

## Goal

Port the database library from `zcaudate-xyz/foundation-base/src-lang/xt/db` into native Hara under `lib/src/std/db`, preserving public behaviour while replacing xtalk-specific implementation details with ordinary `.hal` code.

This is a compatibility port, not a redesign. Structural cleanup can follow parity.

The work has two coordinated tracks:

1. **Library parity** — move the pure `xt.db` namespaces in dependency order.
2. **Executable providers** — prove that `std.db` can execute SQL through runtime-neutral HTA packages.

## Porting rules

1. Port one source namespace at a time with its matching tests.
2. Preserve public names, accepted inputs, returned data and error data.
3. Prefer native Hara predicates, collections and `std.foundation.*` functions over xtalk shims.
4. Keep each namespace at the lowest dependency layer possible.
5. Support keyword-keyed native maps and string-keyed xtalk/JSON boundary maps where representations differ.
6. A file is complete only when direct tests pass and downstream namespaces can load it.
7. Database engines live behind explicit extension/provider boundaries. HAL code never receives JavaScript, Java, Rust or raw WASM engine objects.

## Dependency order

The order is revised whenever a file's actual `:require` graph warrants it.

### Phase 1 — validation and data helpers

| Order | Source | Destination | Status |
|---:|---|---|---|
| 1 | `xt.db.text.base-check` | `std.db.text.base-check` | Implemented with direct tests; CI queued |
| 2 | `xt.db.text.base-util` | `std.db.text.base-util` | Implemented with direct tests; CI queued |
| 3 | `xt.db.text.base-schema` | `std.db.text.base-schema` | Implemented with direct tests; CI queued |
| 4 | `xt.db.text.base-flatten` | `std.db.text.base-flatten` | Implemented with recursive-link tests; CI queued |
| 5 | `xt.db.text.base-scope` | `std.db.text.base-scope` | Deferred behind smaller query/tree helpers |
| 6 | `xt.db.text.base-tree` | `std.db.text.base-tree` | Pending |
| 7 | `xt.db.text.base-graph` | `std.db.text.base-graph` | Pending |

### Phase 2 — SQL text generation

| Order | Source | Destination | Status |
|---:|---|---|---|
| 1 | `xt.db.text.sql-util` | `std.db.text.sql-util` | Implemented with dialect and AST tests; exercised against SQLite WASM |
| 2 | `xt.db.text.sql-raw` | `std.db.text.sql-raw` | Implemented with statement tests; exercised against SQLite WASM |
| 3 | `xt.db.text.sql-tree` | `std.db.text.sql-tree` | Next SQL dependency slice |
| 4 | `xt.db.text.sql-view` | `std.db.text.sql-view` | Pending |
| 5 | `xt.db.text.sql-call` | `std.db.text.sql-call` | Pending |
| 6 | `xt.db.text.sql-graph` | `std.db.text.sql-graph` | Pending |
| 7 | `xt.db.text.sql-table` | `std.db.text.sql-table` | Pending |
| 8 | `xt.db.text.sql-manage` | `std.db.text.sql-manage` | Pending |

### Later phases

- Port `pgrest-*` after base-tree and SQL dependencies.
- Port client/proxy helpers before `xt.db.node.runtime`.
- Port `xt.db.system.*` after text, client and runtime layers.

## Provider-neutral API

`std.db.protocol/IDatabase` now defines engine, provider, metadata, exec, query and close operations. `std.db` exposes the normal application API:

```clojure
(db/engine connection)
(db/provider connection)
(db/info connection)
(db/exec connection sql parameters)
(db/query connection sql parameters)
(db/begin connection)
(db/commit connection)
(db/rollback connection)
(db/close connection)
```

`std.db.sqlite/open` and `std.db.pglite/open` return promises resolving to typed `SQLiteConnection` and `PGliteConnection` values implementing this protocol. Generic protocol behaviour has an engine-independent HAL test; both real-engine JVM tests perform SQL through `std.db` rather than provider-specific execution calls.

## Executable provider track

| Provider | Role | Status |
|---|---|---|
| `std.db.provider.sqlite` | Embedded SQLite compiled to WASM | Provider, typed HAL connection, real-engine tests, packaging and build descriptor implemented; CI queued |
| `std.db.provider.pglite` | Embedded PostgreSQL compiled to WASM | Provider, typed HAL connection, real-engine tests, packaging and build descriptor implemented; CI queued |
| remote PostgreSQL | External PostgreSQL connection | Future distinct provider with explicit network capability; PGlite is not used as a remote connector |

### Shared provider contract

Both embedded providers expose asynchronous HTA operations:

- `version`
- `open` using isolated in-memory storage
- parameterized `exec`
- parameterized row-returning `query`
- `close`

Connections cross the HTA boundary as descriptors containing opaque provider-local integer IDs. HAL wraps those descriptors in typed connections; engine objects remain inside workers. Results use `{:columns [...], :rows [[...]], :affected n}`.

### SQLite vertical slice

SQLite uses official `@sqlite.org/sqlite-wasm`, pinned to `3.50.4-build1` for Hara's current Node 18 managed-process baseline. The initial provider is memory-only. Browser OPFS persistence will be added only with an explicit storage capability and lifecycle.

Validation:

1. Node invokes the real SQLite WASM engine, creates a table, inserts with parameters, selects rows and rejects stale handles.
2. Java/Truffle requires `std.db.sqlite`, opens a typed connection, runs `std.db.text.sql-raw` generated SQL through `std.db/query`, verifies results and checks process-capability denial.

### PGlite vertical slice

PGlite uses `@electric-sql/pglite` `0.5.4` to run embedded PostgreSQL WASM in Node and browser workers. It shares the same typed connection and result contract while retaining PostgreSQL `$1`, `$2`, ... parameters.

PGlite is embedded PostgreSQL. A remote PostgreSQL provider will instead use an explicit network-capable process, HTTP or WebSocket transport.

### Packaging

Both providers have `hara.build.edn` command adapters and deterministic packaging. The packager copies Node and browser workers, discovers every emitted chunk/WASM/data file, and writes those paths into the extension manifest's `:assets` list so `hara extension install` cannot silently omit indirect build artifacts.

## Per-file checklist

- [ ] Read source namespace and original tests.
- [ ] List direct internal and external dependencies.
- [ ] Add the destination `.hal` namespace.
- [ ] Port all public definitions.
- [ ] Add direct success, failure and boundary tests.
- [ ] Compare returned data shapes with the xtalk implementation.
- [ ] Run the narrow test namespace.
- [ ] Run the broader library test suite.
- [ ] Update this plan and move to the next dependency-ready file.
