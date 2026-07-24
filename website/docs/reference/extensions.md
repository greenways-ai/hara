# Hara extensions

Hara extensions are loaded through ordinary namespace clauses. Runtime extensions are packaged
as WASM modules; the program does not construct an engine or transport directly.

```clojure
(ns app
  (:require [crypto.hash :as hash]))

(deref (hash/sha256 data))
```

An extension package contains a `hara.extension.edn` manifest beside its provider artifact:

```clojure
{:namespace "crypto.hash"
 :version "0.1.0"
 :provider :wasm
 :module "math.wasm"
 :abi :core.v1
 :exports {"add" {:args [:i32 :i32] :returns :i32}}
 :capabilities []}
```

The runtime first checks the nearest project's fixed `extensions/` directory and then configured
extension roots. It validates the provider handshake
and generates the requested namespace during `:require`. Extension namespaces are runtime
generated and are not `.hal` source files.

WASM providers implement the extension lifecycle:

```text
discover -> describe -> invoke -> result or error -> cancel -> shutdown
```

Arguments and results use Hara values and explicit opaque handles. Remote calls return promises.
WASM providers use a host WASM engine. Compilation, instantiation, export calls, and memory
access remain behind the provider boundary. `:core.v1` invokes low-level scalar exports directly.
`:hta.v1` uses an import-free, host-driven mailbox and returns Hara promises. On the JVM one virtual
thread owns each nested GraalWasm context; in a browser one Web Worker owns each context. Explicit
`host/call` events are checked against the descriptor's optional `host-calls` allowlist before a
result or structured rejection is delivered to the module.

Loading a manifest does not grant authority. The host applies capability policy and resource
limits, preserving distinct errors for unsupported providers, denied capabilities, malformed
manifests, crashes, timeouts, cancellation, and remote failures.

An official build can separate its source namespace from its registry coordinate: `ledger.noir` is
required by Hara code while `hara/ledger.noir` identifies the maintained package.

External-toolchain extensions may instead declare host-selected HTA targets and a closed list of
built assets. The Noir package uses a managed Node process with HTA over stdin/stdout on
JVM/native-image hosts and a Web Worker with HTA over `postMessage` in browsers. Hara code sees the
same exports, values, errors, promises, and artifacts on both targets. Compiled circuits use
`hara/ledger.noir/v1` as their format identity. Installed projects contain
the built JavaScript, WASM, and worker assets, but not `node_modules`. See the multi-target HTA
package contract for the descriptor and directory layout. Package authors can validate and install
built artifacts with `hara extension check`, `build`, `install`, and `test`; build and
process-backed tests require `--allow-process`.
