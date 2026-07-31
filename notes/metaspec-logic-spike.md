# Metaspec logic spike

Tracking: hara-lang/hara#199

This is a non-normative working note. The EDN documents under `specs/` remain
authoritative.

## Why this layer exists

The spike tests one concrete pipeline:

```text
HAL ^{:schema ...} metadata and metaspec declarations
  -> std.typed normalized schema data
  -> lossless path graph and typed relation tuples
  -> imperative checks, Datalog derivation, or miniKanren search
```

The original EDN remains available at `:graph/document`; vector paths are node
IDs. The relational representation is therefore an index, not a replacement
serialization.

## Current comparison

| Concern | Imperative linter | Datalog | miniKanren |
| --- | --- | --- | --- |
| Deterministic lint findings | Best existing oracle | Natural finite derivation | Possible but verbose |
| Explanation/provenance | Explicitly constructed | Natural rule provenance | Requires goal instrumentation |
| Recursive closure | Manual traversal | Natural fixpoint | Natural fair search |
| Reverse lookup | Separate indexes | Query-variable projection | Native bidirectional use |
| Candidate generation | Explicit repair builders | Finite derived candidates | Natural when bounded |
| Negation | Direct predicates | Stratified finite negation | Requires constructive design or finite exclusion |
| Termination | Controlled loops | Finite facts and range-restricted rules | Caller must bound generative searches |

## Promotion boundary

- Keep `hara.metaspec.lint` and `hara.metaspec.verify` authoritative during the
  experiment.
- Promote `std.typed.schema` only after its surface grammar is reconciled with
  every existing `^{:schema ...}` annotation.
- Use Datalog for finite validation, dependency closure, missing evidence, and
  explainable derived obligations.
- Use miniKanren for interactive forward/reverse exploration and bounded repair
  generation.
- Do not add another normative specification unless it enables a new automated
  decision that the current metaspec cannot express.
