# Hara language metadata migration fixtures

These fixtures define the one-release compatibility boundary for serialized Hara compiler and runtime metadata.

- `legacy.edn` is accepted only at named read boundaries.
- `canonical.edn` is the only shape written by Hara after HARA-2.
- A canonical key wins when both spellings are present.
- Compatibility is shallow at generic map boundaries. Source forms and application values are not recursively rewritten.
- Plugin coordinates and runtime type identifiers are canonicalized explicitly because their keyword values are protocol identifiers rather than source data.

The namespace mapping is mechanical:

```text
:tahto/name          -> :lang/name
:tahto.hara/name     -> :lang.hara/name
:tahto.standard/name -> :lang.standard/name
:tahto.eval/name     -> :lang.eval/name
```

String wire markers such as `$tahto`, historical source paths, and ordinary text are outside this fixture. The migration reserves Hara's former keyword vocabulary for the Greenways Tahto fabric without changing arbitrary user program literals.
