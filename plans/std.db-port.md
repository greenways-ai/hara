# `xt.db` → `std.db` file-by-file port

## Goal

Port `zcaudate-xyz/foundation-base/src-lang/xt/db` into native Hara under `lib/src/std/db`, preserving public behavior while replacing xtalk-specific implementation details with ordinary `.hal` code.

The work has three coordinated tracks:

1. **Graph and SQL parity** — schema-aware graph walking, canonical trees, planning and SQL generation.
2. **Executable providers** — SQLite WASM and PGlite/PostgreSQL WASM behind `std.db`.
3. **Runtime parity** — kernel, client, proxy, worker transports, dynamic services and Supabase actions.

## Porting rules

1. Port one source namespace with direct tests.
2. Preserve public names, input shapes, output shapes and error data.
3. Prefer native Hara collections and `std.foundation.*` functions.
4. Support keyword-keyed native maps and string-keyed xtalk/JSON maps at boundaries.
5. Keep engine objects inside providers; HAL receives typed or remote connection values.
6. A slice is complete only when direct fixtures and downstream integration paths are present.

## Graph-query compiler

The key graph path is now implemented:

```text
compact query
  → std.db.text.base-scope
  → std.db.text.base-graph
  → canonical graph IR
  → std.db.text.base-tree
  → reusable select/count/return/combined plans
  → std.db.text.sql-graph
  → std.db.text.sql-tree / sql-view / sql-call
  → SQLite or PostgreSQL SQL
```

| Source | Destination | Status |
|---|---|---|
| `xt.db.text.base-check` | `std.db.text.base-check` | Implemented with direct tests |
| `xt.db.text.base-util` | `std.db.text.base-util` | Implemented with direct tests |
| `xt.db.text.base-schema` | `std.db.text.base-schema` | Implemented with caches/coercion tests |
| `xt.db.text.base-flatten` | `std.db.text.base-flatten` | Implemented with recursive link tests |
| `xt.db.text.base-scope` | `std.db.text.base-scope` | Implemented: scopes, links, forward/reverse joins and canonical trees |
| `xt.db.text.base-graph` | `std.db.text.base-graph` | Implemented: compact/canonical graph normalization |
| `xt.db.text.base-tree` | `std.db.text.base-tree` | Implemented: planning, controls, placeholder binding and validation |
| `xt.db.text.sql-util` | `std.db.text.sql-util` | Implemented: SQL AST, values and dialect options |
| `xt.db.text.sql-raw` | `std.db.text.sql-raw` | Implemented and exercised against SQLite WASM |
| `xt.db.text.sql-graph` | `std.db.text.sql-graph` | Implemented: recursive linked returns and schema-aware predicates |
| `xt.db.text.sql-tree` | `std.db.text.sql-tree` | Implemented: planned select/count/return/bulk/combined SQL |
| `xt.db.text.sql-view` | `std.db.text.sql-view` | Implemented compatibility facade with tree-or-SQL output |
| `xt.db.text.sql-call` | `std.db.text.sql-call` | Implemented over provider-neutral `std.db` connections |
| `xt.db.text.sql-table` | `std.db.text.sql-table` | Pending |
| `xt.db.text.sql-manage` | `std.db.text.sql-manage` | Pending |

### Graph validation

- Direct HAL fixtures cover scopes, graph normalization, recursive predicates, nested return SQL, planning, views and calls.
- `HaraDbGraphTest` executes the complete fixture stack through the Hara Truffle runtime.
- `std.db Graph Compiler` is a focused GitHub Actions workflow.
- `HaraSqliteProcessExtensionTest` builds a real `User → Team` forward edge and `User → Profile` reverse edge, compiles a compact graph query, executes the generated SQLite SQL and parses the nested JSON result.

## Provider-neutral database API

`std.db.protocol/IDatabase` defines engine, provider, metadata, exec, query and close. `std.db` exposes:

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

SQLite, PGlite and remote runtime services implement the same protocol.

## Executable providers

| Provider | Role | Status |
|---|---|---|
| `std.db.provider.sqlite` | Embedded SQLite WASM | Provider, typed HAL connection, worker bundles, packaging and real-engine tests implemented |
| `std.db.provider.pglite` | Embedded PostgreSQL WASM | Provider, typed HAL connection, worker bundles, packaging and real-engine tests implemented |
| remote PostgreSQL | External PostgreSQL | Future distinct network-capable provider; PGlite is not treated as a remote connector |

Both embedded providers return `{:columns [...] :rows [[...]] :affected n}` and support parameterized `exec`, `query`, transactions and close.

## Runtime status

The original base `xt.db.node` architecture has been ported onto `std.lib.substrate`:

- kernel/client/proxy action routing
- local, SharedWorker, WebWorker and Node worker lifecycle adapters
- structured-clone-safe frame/value/error encoding
- typed remote `std.db` connections
- primary/cache and named dynamic services
- runtime inspection and finalizers
- per-service FIFO execution mutex
- server-side batches and non-interleaved transactions
- Supabase kernel/client/proxy actions around an injected HTTP/session adapter

See `plans/std.db-runtime.md` for runtime details.

## Next dependency slices

1. Run and repair focused graph, runtime, SQLite and PGlite workflows.
2. Port `sql-table` and `sql-manage`.
3. Port remaining PostgREST text layers.
4. Port `xt.db.system.*` assembly and higher-level facade behavior.
5. Add persistent SQLite OPFS only after storage/locking capability semantics are explicit.
