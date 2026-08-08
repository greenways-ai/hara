# `hara-hta`

`hara-hta` is the dependency-free canonical HTA1 codec for portable
[`hara-abi`](../abi/) values.

It exists for embedding hosts, native providers, package tooling, and durable
state boundaries that need exact Hara bytes without linking the executable
runtime, Wasmtime, Nginx, or application services.

The portable value profile contains:

```text
nil
booleans
signed integers
IEEE-754 floats
strings
bytes
keywords
decimals
vectors
records with unique keyword keys
```

Portable records use the existing HTA map wire tag. Their string field names
encode as keyword keys and are sorted by canonical encoded key bytes. Decoding
rejects maps with duplicate or non-keyword keys.

Runtime-only tags such as symbols, lists, sets, handles, namespaces, vars,
atoms, arrays, objects, characters, big integers, and regex values fail closed.
The crate shares HTA1's 64 MiB frame bound and 256-level nesting bound.

`hara-wasm` contains byte-for-byte cross-codec tests. A change to either codec
must preserve identical encoding for every portable value before it can merge.
