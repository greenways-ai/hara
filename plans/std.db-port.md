# `xt.db` → `std.db` file-by-file port

## Goal

Port the database library from `zcaudate-xyz/foundation-base/src-lang/xt/db` into native Hara under `lib/src/std/db`, preserving the public behaviour of each source namespace while replacing xtalk-specific implementation details with ordinary `.hal` code.

This is a compatibility port, not a redesign. Structural cleanup can happen after parity is established.

## Porting rules

1. Port one source namespace at a time with its matching test namespace.
2. Preserve public function names, accepted inputs, return values and error data.
3. Prefer native Hara predicates, collections and `std.foundation.*` functions over xtalk compatibility shims.
4. Keep each namespace at the lowest dependency layer possible.
5. Add compatibility handling only where the xtalk boundary changed representation, such as keyword versus string map keys.
6. A file is complete only when its direct tests pass and downstream namespaces can load it.
7. Record intentional differences in this plan before moving to the next file.

## Dependency order

The order below is provisional. Before each port, inspect its `:require` graph and move it earlier or later as needed.

### Phase 1 — pure validation and data helpers

| Order | Source | Destination | Status |
|---:|---|---|---|
| 1 | `xt.db.text.base-check` | `std.db.text.base-check` | In progress |
| 2 | `xt.db.text.base-util` | `std.db.text.base-util` | Pending |
| 3 | `xt.db.text.base-scope` | `std.db.text.base-scope` | Pending |
| 4 | `xt.db.text.base-schema` | `std.db.text.base-schema` | Pending |
| 5 | `xt.db.text.base-flatten` | `std.db.text.base-flatten` | Pending |
| 6 | `xt.db.text.base-tree` | `std.db.text.base-tree` | Pending |
| 7 | `xt.db.text.base-graph` | `std.db.text.base-graph` | Pending |

### Phase 2 — SQL text generation

Provisional order: `sql-util`, `sql-raw`, `sql-tree`, `sql-view`, `sql-call`, `sql-graph`, `sql-table`, then `sql-manage`.

### Phase 3 — PostgREST and database-facing text layers

Port the `pgrest-*` namespaces after their base-tree and SQL dependencies are complete.

### Phase 4 — node, client and proxy layers

Port low-level client and proxy helpers before `xt.db.node.runtime`. Host-facing behaviour should use Hara protocols or native boundaries rather than embedding JavaScript assumptions.

### Phase 5 — system assembly

Port `xt.db.system.*` and the public `std.db` facade last, then run integration and parity tests across the complete namespace graph.

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

## Current slice: `base-check`

`xt.db.text.base-check` is first because it depends only on primitive string, collection and type operations. The native port preserves the original deliberately shallow UUID check: it validates the five groups and their lengths but does not validate hexadecimal characters or UUID version bits.

The port also accepts both `:type` and `"type"` in argument descriptors. This preserves native Hara map usage while remaining compatible with object-shaped descriptors arriving from xtalk or JSON boundaries.
