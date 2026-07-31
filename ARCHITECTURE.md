# Current architecture

Hara has a portable language core and runtime-specific capability layers. The Truffle runtime is
the current implementation target.

```text
+-------------------+
| Hara source / REPL|
+---------+---------+
          |
          v
+-------------------+       +----------------------+
| reader + Truffle AST| ----> | language-neutral L0  |
+---------+---------+       | protocols / values   |
          |                 +----------+-----------+
          v                            |
+-------------------+                  v
| HaraContext        | ------> generated libraries
| namespaces / vars  |        core, bytes, promise,
+---------+---------+        string, file, socket
          |
          v
+-------------------+
| explicit provider |
| and capability edge|
+---------+---------+
          |
          +--> JVM flavor
          +--> pod provider
          +--> WASM provider (planned)
```

## Runtime layers

1. **Reader and AST** parse Hara forms into Truffle nodes.
2. **L0 core** evaluates control flow, functions, namespaces, protocols, collections, and promises.
3. **Generated libraries** expose portable operations through qualified names such as
   `bytes/count`, `promise/then`, and `file/read`.
4. **Capability/provider boundary** supplies host-dependent behavior. Unsupported and denied
   operations are stable, distinct errors.

The legacy Java interpreter remains in the source tree for migration and compatibility work, but it
is not the authoritative implementation of the current language surface.

## Extension boundary

```text
:require [extension.ns :as ext]
             |
             v
       manifest registry
          /        \
       pod          WASM
          \        /
           Hara values + promises
```

The earlier extension lifecycle and capability design is retained in the
[HTA contract](specs/01-lang/008-hta/draft/hal-hta-contract.md).
