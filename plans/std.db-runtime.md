# `std.db` runtime architecture

## Implemented layers

### Provider-neutral connection API

`std.db.protocol/IDatabase` and `std.db` provide engine, provider, metadata,
execution, query, transaction helper and close operations. Direct SQLite,
PGlite and remote runtime connections implement the same protocol.

### Kernel services

`std.db.node.kernel-base` owns configured `db/primary`, optional `db/caching`
and `db/common` services. Drivers are registered separately, so the kernel can
load without SQLite or PGlite extensions.

Each database service owns a FIFO promise queue. Direct calls, batches,
transactions and close operations are serialized through that queue.

### Client and proxy

`std.db.node.client-base` exposes kernel, service, batch, transaction and
inspection actions. `std.db.node.proxy-base` forwards the same action IDs over
any Substrate transport. Legacy IDs such as `@xt.db/kernel-init` are retained.

### Local and worker runtimes

`std.db.node.runtime` supports:

- linked in-memory client/server nodes;
- SharedWorker, WebWorker and Node worker adapter boundaries;
- ready handshakes using `xt.db.default.transport` and
  `xt.db.default.worker`;
- transactional startup cleanup;
- runtime-owned dynamic services and finalizers;
- remote services opened as ordinary `std.db` connections.

### Message endpoint transport

`std.db.node.transport` defines a host-neutral endpoint contract:

```clojure
{:send   (fn [message] ...)
 :listen (fn [listener] ...)
 :close  (fn [] ...)}
```

Frames recursively encode keywords, symbols, maps, sets, vectors and errors as
structured-clone-safe tagged values. JavaScript wrappers exist for browser
Worker, SharedWorker/MessagePort and Node `parentPort` objects.

### Dynamic services

Named services can be listed, opened, reused, inspected and closed without
rebuilding primary/cache configuration. Kernel-owned primary and cache services
are protected from dynamic close operations. Runtime shutdown closes every
remaining dynamic service.

### Batches and transactions

`@xt.db/batch` runs mixed exec/query statement descriptors sequentially. A
transactional batch acquires the service queue once, runs `BEGIN`, every
statement and `COMMIT`, and rolls back on failure. Other requests cannot enter
the same connection until the batch settles.

### Supabase compatibility

The original `xt.db.node.kernel-supabase`, client and proxy action contract is
ported. The kernel owns response/error normalization, session state, sign-in,
sign-out, refresh and auto-refresh lifecycle. Hosts inject a `:request` adapter
for actual HTTP command execution.

Supabase services register runtime finalizers, ensuring refresh timers and
session resources stop before database and transport shutdown.

## Validation

Native HAL fixtures cover:

- kernel/client/proxy execution;
- FIFO serialization and transaction isolation;
- batch commit and rollback ordering;
- dynamic service lifecycle;
- runtime status inspection;
- worker ready handshake and frame serialization;
- Supabase session and action semantics.

Java/Truffle tests load every fixture through the Hara language. Separate real
SQLite WASM and PGlite tests exercise direct connections and remote runtime
connections.

## Remaining runtime work

1. Expose live JavaScript host-function injection in `@hara-lang/browser`, then
   bind the existing endpoint wrappers directly to evaluated HAL worker code.
2. Add a concrete remote PostgreSQL transport provider. PGlite remains the
   embedded PostgreSQL provider and is not treated as a network connector.
3. Port higher-level `xt.db` view/system actions onto the completed kernel,
   client and proxy substrate.
4. Add persistent browser storage capabilities for SQLite OPFS and PGlite data
   directories with explicit lifecycle and locking policies.
