# Greenways Build DSL

> **Greenways contribution.** Greenways owns this artifact format. Hara hosts
> and verifies the published specification but does not make it part of the HAL
> language contract.

The contribution's normative specification is
[`greenways-buildspec.edn`](greenways-buildspec.edn). It conforms to the sibling
HAL artifact meta-spec and deliberately leaves the HAL language specification
unchanged.

Version 0.1 supports only `build`, `artifact`, `use-spec`, `stage`, and `check`.
It describes construction; it does not execute transformations. Source is read
as HAL, normalized to qualified canonical EDN, and checked locally without
network access or arbitrary checker loading.

Checker implementation identity and checker policy are distinct. An immutable
implementation identifies a GitHub repository, full 40-character commit SHA,
repository-relative source path and qualified entrypoint. A variation has its
own ID, configuration and digest.

The CLI surface is:

```text
hara spec check FILE [--format text|edn]
hara spec to-edn FILE [--format text|edn]
hara spec from-edn FILE [--format text|edn]
hara spec normalize FILE [--format text|edn]
hara spec graph FILE [--format text|edn]
hara spec obligations FILE [--format text|edn]
```

Exit status 0 means every required obligation passed, 1 means a valid build has
failed, unknown or blocked obligations, and 2 means the tool could not read or
resolve its input.
