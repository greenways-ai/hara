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

## Byte format (version 1)

All integers are big-endian. A `string` is a 4-byte length followed by that many UTF-8 bytes.
A `count` is a 4-byte signed integer limited to 1,000,000 items. A `nullable-string` is a
boolean flag followed by a `string` when the flag is set.

Envelope:

| Field | Type |
|-------|------|
| magic | `48 49 52 00` ("HIR\0") |
| format version | u16, currently `1` |
| capability flags | u16, `1` = executable foundation |
| payload length | u32 |
| payload SHA-256 | 32 bytes |
| payload | payload-length bytes |

Payload:

| Field | Type |
|-------|------|
| namespace | string |
| resource | string |
| source SHA-256 | 32 bytes |
| form count | count |
| forms | form-count values |

Every value is prefixed with a one-byte opcode:

| Opcode | Value | Payload |
|--------|-------|---------|
| 0 | nil | — |
| 1 | false | — |
| 2 | true | — |
| 3 | long | 8 bytes |
| 4 | double | 8 bytes IEEE 754 |
| 5 | big-integer | decimal string |
| 6 | big-decimal | decimal string |
| 7 | string | string |
| 8 | character | u32 code point |
| 9 | symbol | nullable-string namespace, string name, metadata |
| 10 | keyword | nullable-string namespace, string name, metadata |
| 11 | list | count, items, metadata |
| 12 | vector | count, items, metadata |
| 13 | map | count, key/value pairs, metadata |
| 14 | set | count, items, metadata |
| 15 | ordered map | count, key/value pairs in insertion order, metadata |
| 16 | ordered set | count, items in insertion order, metadata |
| 17 | regex | pattern string |

Metadata is a boolean flag; when set, a single map value follows.

Canonical ordering: plain map and set entries are written in the unsigned lexicographic order
of their encoded key or element bytes, so the artifact never depends on host collection
iteration order. Ordered maps and sets keep insertion order because order is semantic there.
Decoders must accept entries in any order.

Regex values encode only the pattern string, which any host can recompile. Patterns created
with host-specific flags cannot be represented; encoders must reject them.

`HirArtifactTest.goldenBytesLockThePortableFormat` locks this layout with a golden byte
vector covering every opcode.
