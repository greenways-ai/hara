# Offline Greenways Build demonstrator

`music-build.hal` models the deterministic dependency chain:

```text
source → music model → timeline → MIDI output
```

The stages are descriptions only; version 0.1 performs structural, graph,
identity and codec checks without executing transformations or loading remote
code.

Run the valid and invalid cases:

```text
hara spec check contrib/greenways/build/examples/music-build.hal --format edn
hara spec check contrib/greenways/build/examples/invalid-checker.hal --format edn
hara spec graph contrib/greenways/build/examples/invalid-cycle.hal --format edn
```

The invalid checker uses a branch name where an immutable 40-character commit
SHA is required. The invalid cycle returns the closed cycle path. A failure in
an upstream stage gives each direct dependent a `:blocked` status; independent
branches remain runnable.

Round-trip the valid build entirely offline:

```text
hara spec to-edn contrib/greenways/build/examples/music-build.hal --format edn
hara spec from-edn canonical-build.edn
```
