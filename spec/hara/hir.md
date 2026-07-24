# Foundation HIR

`std.lib.foundation` has a build-time binary artifact at `std/lib/foundation.hir`. The artifact is
generated from `lib/src/std/lib/foundation.hal` during Maven's `compile` phase:

```shell
hara compile-hir lib/src/std/lib/foundation.hal \
  --output target/classes/std/lib/foundation.hir
```

The version 1 artifact is deterministic and host-neutral. Its executable-foundation capability
flag requires the direct bootstrap lowerer. It contains the declared namespace,
logical source-resource name, source SHA-256, reader forms with metadata, and a checksum covering
the complete payload. It never contains Java serialization or context-bound Truffle objects.

Foundation resource loading is controlled by the `hara.HirMode` system property:

- `auto` (default) loads valid HIR and falls back to the packaged `.hal` source when HIR is absent
  or fails structural validation.
- `strict` requires a valid HIR artifact. This mode is intended for packaging tests and benchmarks.
- `off` bypasses HIR and loads the `.hal` source.

Both paths retain the ordinary namespace transaction, module revision, Var metadata, reload, and
Java/HAL provider behavior. Version 1 skips UTF-8 reading, Lisp parsing, unavailable-source span
mapping, macro expansion, and the general `HaraAnalyzer`. `FoundationHirLowerer` directly creates
Truffle nodes for the closed bootstrap subset and rejects unsupported binding or special forms.
The portable forms remain in the artifact so structural constants and Var metadata stay
host-neutral.

Run the comparison benchmark with:

```shell
hara foundation-hir-benchmark 20
```

It emits JSON containing median context construction, first callable foundation load, allocated
bytes, relative load speedup, and allocation reduction for source and strict-HIR modes. It also
reports a shared-engine lane, which measures normal Engine reuse without sharing context-bound ASTs.

On the 2026-07-25 direct-lowerer development run (30 in-process samples), source loading had a
3.890 ms median and HIR loading had a 1.871 ms median: 2.080x faster with 62.3% fewer allocated
bytes. With a reused Polyglot engine, HIR loading had a 1.228 ms median (3.168x versus the
cold-engine source lane). These are directional development figures, not cross-machine release
claims.
