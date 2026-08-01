# `xt.db` → `std.db` file-by-file port

## Goal

Port the database library from `zcaudate-xyz/foundation-base/src-lang/xt/db` into native Hara under `lib/src/std/db`, preserving the public behaviour of each source namespace while replacing xtalk-specific implementation details with ordinary `.hal` code.

This is a compatibility port, not a redesign. Structural cleanup can happen after parity is established.

The port has two coordinated tracks:

1. **Library parity** — move the pure `xt.db` namespaces in dependency order.
2. **Executable providers** — prove that the resulting `std.db` API can execute SQL through runtime-neutral HTA packages.

## Porting rules

1. Port one source namespace at a time with its matching test namespace.
2. Preserve public function names, accepted inputs, return values and error data.
3. Prefer native Hara predicates, collections and `std.foundation.*` functions over xtalk compatibility shims.
4. Keep each namespace at the lowest dependency layer possible.
5. Add compatibility handling only where the xtalk boundary changed representation, such as keyword versus string map keys.
6. A file is complete only when its direct tests pass and downstream namespaces can load it.
7. Record intentional differences in this plan before moving to the next file.
8. Database engines must live behind explicit extension/provider boundaries. HAL code must not receive JavaScript, Java, Rust or raw WASM implementation objects.

## Dependency order

The order below is provisional. Before each port, inspect its `:require` graph and move it earlier or later as needed.

### Phase 1 — pure validation and data helpers

| Order | Source | Destination | Status |
|---:|---|---|---|
| 1 | `xt.db.text.base-check` | `std.db.text.base-check` | Implemented; validation queued |
| 2 | `xt.db.text.base-util` | `std.db.text.base-util` | Implemented; validation queued |
| 3 | `xt.db.text.base-schema` | `std.db.text.base-schema` | Implemented with direct tests; validation queued |
| 4 | `xt.db.text.base-flatten` | `std.db.text.base-flatten` | Next dependency-ready port |
| 5 | `xt.db.text.base-scope` | `std.db.text.base-scope` | Dependency analysis completed; deferred behind smaller helpers |
| 6 | `xt.db.text.base-tree` | `std.db.text.base-tree` | Pending |
| 7 | `xt.db.text.base-graph` | `std.db.text.base-graph` | Pending |

### Phase 2 — SQL text generation

Provisional order: `sql-util`, `sql-raw`, `sql-tree`, `sql-view`, `sql-call`, `sql-graph`, `sql-table`, then `sql-manage`.

### Phase 3 — PostgREST and database-facing text layers

Port the `pgrest-*` namespaces after their base-tree and SQL dependencies are complete.

### Phase 4 — node, client and proxy layers

Port low-level client and proxy helpers before `xt.db.node.runtime`. Host-facing behaviour should use Hara protocols or native boundaries rather than embedding JavaScript assumptions.

### Phase 5 — system assembly

Port `xt.db.system.*` and the generic `std.db` facade last, then run integration and parity tests across the complete namespace graph. Provider-specific facades such as `std.db.sqlite` and `std.db.pglite` may land earlier to create executable vertical slices.

## Executable provider track

| Provider | Role | Status |
|---|---|---|
| `std.db.provider.sqlite` | Embedded SQLite compiled to WASM | Provider, package, HAL facade and real-engine tests implemented; CI queued |
| `std.db.provider.pglite` | Embedded PostgreSQL compiled to WASM | Provider, package, HAL facade and real-engine tests implemented; CI queued |
| remote PostgreSQL | Network connection to an external PostgreSQL server | Separate future provider; must use an explicit network-capable host bridge rather than treating PGlite as a remote connector |

### Shared provider contract

Both embedded providers expose asynchronous HTA operations with the same data boundary:

- `version`
- `open` using isolated in-memory storage
- parameterized `exec`
- parameterized row-returning `query`
- `close`
- transaction helpers implemented in HAL using `begin`, `commit` and `rollback`

Connections cross the HTA boundary as immutable descriptors containing an opaque provider-local integer ID. Engine objects remain inside their worker. Results use the portable shape `{:columns [...], :rows [[...]], :affected n}`.

### SQLite vertical slice

SQLite uses the official `@sqlite.org/sqlite-wasm` package through Hara's existing HTA worker transports. The package is pinned to `3.50.4-build1` because the current Hara managed-process baseline is Node 18, while newer package releases declare a newer Node requirement.

The initial provider is deliberately memory-only. Browser persistence is not selected implicitly; OPFS support will be added only after an explicit storage capability and persistence lifecycle are defined.

Validation has two levels:

1. A Node test invokes the real SQLite WASM engine, creates a table, inserts with bind parameters, selects rows and verifies stale-handle failure.
2. A Java/Truffle integration test requires `std.db.sqlite` as ordinary HAL code, dereferences its promises, executes SQL and verifies that process capability is denied when not granted.

### PGlite vertical slice

PGlite uses `@electric-sql/pglite` to run embedded PostgreSQL in WASM in Node and browser workers. It shares the SQLite connection/result contract, while preserving PostgreSQL parameter syntax (`$1`, `$2`, ...). Its Node test and Java/Truffle test mirror the SQLite validation path.

PGlite is an embedded PostgreSQL implementation, not a remote connector. Remote PostgreSQL will use a distinct provider with explicit network capability and a transport appropriate to the host, such as a managed process, HTTP bridge or WebSocket service.

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

## Implemented slice: `base-check`

`xt.db.text.base-check` is first because it depends only on primitive string, collection and type operations. The native port preserves the original deliberately shallow UUID check: it validates the five groups and their lengths but does not validate hexadecimal characters or UUID version bits.

The port also accepts both `:type` and `"type"` in argument descriptors. This preserves native Hara map usage while remaining compatible with object-shaped descriptors arriving from xtalk or JSON boundaries.

## Implemented slice: `base-util`

`xt.db.text.base-util` has been ported using immutable Hara maps and vectors. Route, view and ID helpers accept both keyword-keyed native maps and string-keyed boundary maps. `merge-views` preserves the source collision rule: on an existing table entry it merges the existing `select` and `return` categories rather than replacing the complete table descriptor.

## Implemented slice: `base-schema`

`xt.db.text.base-schema` now provides table listing, data/reference/reverse key classification, defaults, physical column ordering, table ordering, recursive type coercion and atom-backed caches. Classified metadata follows the input schema's key convention, using native keywords for keyword maps and xtalk-compatible strings such as `"ref_id"` for boundary maps.

## Next parity slice: `base-flatten`

`base-flatten` is now dependency-ready because its only `xt.db` dependency is `base-schema`. It will be ported before the larger `base-scope` namespace so nested object flattening can establish and test the schema-link semantics independently.
