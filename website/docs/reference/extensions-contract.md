# Hara extension contract

This document describes the WASM provider boundary behind ordinary `:require` forms. A Hara
program does not construct transport messages or call a WASM engine directly.

## Discovery

```text
(:require [crypto.hash :as hash])
                 |
                 v
        +-------------------+
        | namespace registry |
        +-------------------+
          | already loaded?
          | yes       | no
          v           v
       reuse     search extension roots
                        |
                        v
                hara.extension.edn
                        |
                        v
                 validate manifest
                        |
                        v
                 select provider
                        |
                        v
                 create namespace
```

An extension package contains a `hara.extension.edn` manifest beside its provider artifact:

```clojure
{:namespace "crypto.hash"
 :version "0.1.0"
 :provider :wasm
 :module "math.wasm"
 :abi :core.v1
 :exports {"add" {:args [:i32 :i32] :returns :i32 :async true}}
 :capabilities []}
```

The portable fields are `namespace`, the optional registry `identity`, `version`, `provider`, `module`, `abi`, `exports`,
`capabilities`, and the optional `host-calls` and `handles` maps. Public handle tags are declared by
wire type:

```clojure
:handles {"tensor" {:tag math}}
```

Registered handles print compactly as `#math[:tensor 42]`. An unregistered handle uses the
provider-neutral fallback `#ht[:handle 42]`; the transport owner and type remain available
internally for ownership checks and diagnostics. Reading either tagged form creates inert data,
not a live capability, so source text cannot forge a provider-owned handle.

Browser hosts can load the same EDN descriptor and resolve its sibling WASM module directly:

```javascript
const context = await loadHtaExtension({
  worker: new Worker("./hta-worker.js", {type: "module"}),
  descriptorUrl: new URL("./hara.extension.edn", import.meta.url).toString()
});
```

The loader validates `:provider`, `:module`, `:abi`, and `:handles` before starting the worker.

The manifest is metadata, not executable Hara code. Extension namespaces are runtime-generated
namespaces, not `.hal` source files.

## Provider lifecycle

```text
discover -> describe -> start -> invoke -> result or error -> cancel -> shutdown
                 |                         |
                 |                         +--> timeout
                 +--> capability check
                         |
                  denied or granted
```

Arguments and results use Hara values and explicit opaque handles. Remote calls return promises:

```text
Hara call
   |
   v
provider invoke(request-id, function, args)
   |
   +----> pending promise
   +----> result(value) ------> promise realization
   +----> error(kind, message) -> promise rejection
```

WASM providers use a host engine. Compilation, instantiation, export calls, and memory access
remain behind the provider boundary. `:core.v1` is the direct scalar ABI. `:hta.v1` is the Hara
Transport Adaptor for stateful, promise-returning modules.

## HTA v1

HTA modules remain import-free. The host drives an exported mailbox rather than injecting ambient
WASM imports:

```text
Hara call -> hta_start -> task
                         |
                 hta_next_event
                    /          \
             settlement      host-call
                                 |
                   descriptor :host-calls check
                                 |
                        hta_deliver result/error
```

The required exports are `hta_abi_version`, `hta_alloc`, `hta_dealloc`, `hta_start`,
`hta_next_event`, `hta_deliver`, `hta_poll`, `hta_cancel`, `hta_drop_task`, and `hta_release`. Frames use the
canonical binary `HTA1` value encoding. Its portable value intersection is nil, booleans, signed
64-bit integers, UTF-8 strings, bytes, keywords, symbols, lists, vectors, sets, maps, and opaque `{owner, type, id}` handles. Map and
set elements are ordered by their encoded bytes so Rust, Java, and JavaScript produce identical
frames.

Each Truffle extension instance has one Java virtual-thread actor which exclusively owns its
nested GraalWasm context. Browser instances use one Web Worker per context. Both actors block on a
mailbox while idle; neither polls in a spin loop. Package completions enqueue a delivery back to
the owner before any WASM export is called. Pending `deref` suspends an explicit evaluator fiber; settlement resumes its retained continuation without replaying completed forms.

Host calls are explicit Hara operations such as:

```hal
(host/call "crypto.hash.sha256" "digest" input)
```

They are denied unless the descriptor contains, for example,
`:host-calls {"crypto.hash.sha256" ["digest"]}`. Rejections cross HTA as structured values with
`code`, `message`, `data`, `origin`, `retryable`, `stack`, and `cause` fields as applicable. Host
stack traces are not transported. A rejected settlement realizes the corresponding Hara promise
exceptionally; it is not a Rust panic or a Java exception thrown through WASM.

## Multi-target HTA packages

Extensions implemented by an external toolchain are distributed as built artifacts. The
development checkout may require package-manager dependencies to produce those artifacts, but a
Hara project must not require the development dependency tree at runtime.

For Noir, the current build produces deployable JavaScript and WASM assets:

```text
noir-loader.js              ~5 MB
noir-wasm.mjs              ~16 MB
barretenberg*.js            ~4 MB each
assets/*.worker.js
```

Although the development checkout needs `node_modules` to build these files, an installed Hara
project contains only the built result:

```text
project.hal
extensions/
  ledger/noir/
    hara.extension.edn
    node/
      worker.mjs
    browser/
      worker.mjs
    assets/
      noir-wasm.mjs
      barretenberg.js
      barretenberg-threads.js
      main.worker.js
      thread.worker.js
```

A package identity such as `hara/ledger.noir` is the immutable registry owner/name coordinate;
`ledger.noir` remains the namespace used by Hara source. A multi-target descriptor declares
target-specific entry points and the shared assets required by those entry points:

```clojure
{:namespace "ledger.noir"
 :identity "hara/ledger.noir"
 :version "0.1.0"
 :provider :hta
 :abi :hta.v1

 :targets
 {:node
  {:module "node/worker.mjs"
   :runtime :process}

  :browser
  {:module "browser/worker.mjs"
   :runtime :web-worker}}

 :assets
 ["assets/noir-wasm.mjs"
  "assets/barretenberg.js"
  "assets/barretenberg-threads.js"
  "assets/main.worker.js"
  "assets/thread.worker.js"]

 :exports
 {"compile" {:args [:value] :returns :value :async true}
  "prove"   {:args [:value :map] :returns :value :async true}
  "verify"  {:args [:value :value] :returns :boolean :async true}}

 :capabilities []}
```

The host selects the target. Hara source does not branch on its host environment:

```text
hara JVM/native image
        -> :node target
        -> managed Node subprocess
        -> HTA over stdin/stdout

Hara in browser
        -> :browser target
        -> Web Worker
        -> HTA over postMessage
```

Both workers implement the same request operations, values, errors, promises, and artifact
formats. A compiled circuit uses the exact format identity `hara/ledger.noir/v1`; the final
`/v1` versions the circuit envelope independently of the extension release. Only the transport
differs. Target modules and every path in `:assets` are package-relative
paths and must remain inside the extension directory. The runtime validates the selected target
and all declared assets before starting the worker.

This multi-target descriptor is the required packaging contract for Noir. Both the JVM/native
registry and browser loader validate it, select their host target, and reject missing or escaping
assets before worker startup.

The reference package at `lib/examples/extensions/crypto/hash/sha256` is a real, import-free Rust WASM
extension:

```hal
(ns crypto.example
  (:require [crypto.hash.sha256 :as sha]))

(deref (sha/digest (bytes 97 98 99)))
```

Build its artifact with `scripts/build-crypto-hash-sha256-wasm` and install the resulting `.wasm`
beside the example descriptor as `sha256.wasm`.

Loading a manifest does not grant authority:

```text
manifest requests capability
          |
          v
      host policy
       /       \\
    grant     deny
      |         |
 provider    stable denied error
```

The runtime preserves distinct errors for missing or malformed manifests, unsupported providers,
denied capabilities, crashes, timeouts, cancellation, and remote failures. The Hara namespace is
backed by a capability-checked WASM provider without exposing the engine or HTA mailbox at the
call site.

## Project installation and build workflow

The nearest `project.hal` fixes the project extension root at `extensions/`. A namespace maps to a
package directory by replacing dots with path separators, so `ledger.noir` is discovered
at `extensions/ledger/noir/hara.extension.edn`. `hara.extensions.path` and
`HARA_EXTENSION_PATH` remain explicit additional roots for launchers and test harnesses.

Source checkouts may carry a `hara.build.edn`; installed packages do not. The canonical command
adapter is explicit and contains no shell interpolation:

```clojure
{:adapter :command
 :command ["npm" "run" "build:noir"]
 :working-directory "../../web"
 :output "../../web/dist/extensions/ledger/noir"}
```

The packaging commands are:

```text
hara extension check BUILT-PACKAGE
hara --allow-process extension build SOURCE-PACKAGE
hara extension install BUILT-PACKAGE
hara --allow-process extension test BUILT-PACKAGE
```

`check` validates the descriptor and every declared file. `build` runs the declared adapter and
validates its output. `install` finds the nearest project, atomically copies only the descriptor,
target modules, and declared assets, and records SHA-256 hashes in `hara.install.edn`; it refuses to
overwrite an installed namespace. `test` validates the package and, for a Node HTA target, performs
the protocol handshake. Process-backed operations are denied unless `--allow-process` is present.

## Packaged answer-42 demonstration

The import-free WASM demonstration is named `demo.000-answer-42`. Its runnable descriptor
template lives at
`lib/examples/extensions/demo/000-answer-42/hara.extension.edn`; an installed bundle places that
descriptor beside `answer-42.wasm` under the same namespace-derived directory. Requiring it
generates its namespace directly from the declared WASM exports:

```hal
(ns proof.example
  (:require [demo.000-answer-42 :as answer]))

(answer/add 20 22)
```

Applications opt into the WASM surface through `:require`. Runtime bundles can be installed under
an explicit root in `hara.extensions.path` or `HARA_EXTENSION_PATH`, using the same
namespace-derived directory layout.

The template works with the repository's raw Rust fixture copied as `answer-42.wasm`. The initial
`:core.v1` ABI supports import-free core-WASM functions using `:i32`, `:i64`, `:f32`, `:f64`,
`:boolean`, and `:void`. The demonstration is deliberately separate from Noir, leaving
`ledger.noir` available for the compiler, prover, and verifier integration.

Classpath discovery rejects duplicate namespace packages, malformed manifests, unknown providers,
unknown modules, and denied capability requests before installing any exported vars.
