# chrome-hara design

A Chrome (MV3) extension that embeds the hara Rust runtime (compiled to
WebAssembly) in a DevTools panel, exposes the Chrome API to hara code as
`(require [chrome.api :as api])`, loads `.hal` automation scripts from a
user-picked home directory, and optionally exposes a RESP eval endpoint via a
local bridge.

Implementation plan: `docs/superpowers/plans/2026-07-24-chrome-hara.md`.
Tracking: GitHub issue #180.

## Runtime embedding

The extension ships the actual Rust hara evaluator — `rust/src/core.rs`,
`kernel`, `lang`, `task`, `fiber` — built by the `rust/raw` crate into
`hara_wasm_raw.wasm` and vendored into `chrome-hara/vendor/` by
`scripts/sync-runtime.mjs`. It runs in a Web Worker spawned by the DevTools
panel. The REPL's evals, fibers, promise suspension, and `require` machinery
all execute inside the Rust/wasm evaluator; JS is only transport and host
services.

It is the **raw HTA build**, not the wasm-bindgen build the playground uses:
the bindgen `Runtime` never wires `host/call` and only offers synchronous
`eval` — the wrong fit for async CDP automation. Same Rust core either way.
No native Rust binary is involved; the one external process (RESP bridge) is
plain Node.js.

## Reuse vs. divergence from the browser hara wasm

```
                        ┌──────────────────────────────────────────┐
                        │   SHARED RUST SOURCE (100% reused)        │
                        │   rust/src/core.rs · kernel · lang ·      │
                        │   task · fiber · hta.rs                   │
                        │   (eval, fibers, promises, require,       │
                        │    host/call special form)                │
                        └───────┬──────────────────────┬───────────┘
                                │ #[path] include      │ crate root
                                ▼                      ▼
              ┌──────────────────────────┐ ┌──────────────────────────┐
              │ RAW HTA BUILD            │ │ WASM-BINDGEN BUILD        │
              │ rust/raw →               │ │ wasm → hara_wasm.js/pkg   │
              │ hara_wasm_raw.wasm       │ │ Runtime class             │
              │ C ABI: hta_start/        │ │ sync eval() → String      │
              │ deliver/next_event       │ │ foundation bootstrapped   │
              │ host/call WIRED ✓        │ │ host/call NOT wired ✗     │
              │ require + resources ✓    │ │ require: std/ext only     │
              └───────┬──────────────┬───┘ └──────────┬───────────────┘
                      │              │                │
        reused verbatim│              │ pattern only   │
                      ▼              ▼                ▼
     ┌────────────────────┐ ┌───────────────┐ ┌───────────────────────┐
     │ CHROME-HARA        │ │ HTA SMOKE     │ │ PLAYGROUND            │
     │ (this extension)   │ │ rust/web/     │ │ rust/web/             │
     │                    │ │ hta-browser.* │ │ playground.js         │
     │ vendor/ = copied:  │ │               │ │ index.html            │
     │  hta.js            │ │ sha256        │ │                       │
     │  hta-worker.js     │ │ hostcall demo │ │ textarea + eval btn   │
     │  hara.wasm         │ │               │ │ sync, stringly        │
     └────────────────────┘ └───────────────┘ └───────────────────────┘
```

Everything below the wasm boundary is new in chrome-hara:

```
 hara source ──► [Rust/wasm evaluator in worker]   ← same in all three
                        │ host/call event
                        ▼
   playground:     n/a (host/call unavailable)
   hta smoke:      hostCalls = { "crypto.hash.sha256/digest": ... }   (hardcoded, in-page)
   chrome-hara:    hostCalls = Proxy ──► chrome.runtime Port ──► service worker
                                              (new)                    (new)
                                              host-bridge.js           background.js
                                              toPlain/fromPlain        chrome.* proxy
                                                                       chrome.debugger/CDP

   chrome-hara extras (all new, nothing reused):
     • DevTools panel REPL (panel.js/html) — async rewrite of the playground idea
     • register-resource HTA target + core require machinery (Rust)
     • chrome.api namespace (src/hara/api.hal)
     • home-dir .hal preloader (src/home.js)
     • RESP bridge (bridge/resp-bridge.mjs) + src/resp-client.js
```

Summary of the split:

- **Reused 1:1** — the entire Rust evaluator source, the raw wasm artifact,
  `hta.js` (HTA1 codec, `HtaContext`, manifest loader), `hta-worker.js`.
  Zero changes to `rust/web/`; its tests pass untouched.
- **Reused by pattern** — the fiber-suspending host/call demo from
  `hta-browser.js` became the port-forwarding Proxy bridge; the playground's
  textarea-REPL idea became an async DevTools panel.
- **New** — everything extension-shaped (manifest, service worker, panel),
  the `require`/resource Rust machinery, `chrome.api`, home-dir loading,
  RESP bridge.
- **Deliberately not reused** — the wasm-bindgen `Runtime` (sync-only, no
  host/call) and the `std.lib.foundation` bootstrap (raw runtime stays
  core-only, which is why `api.hal` avoids foundation fns).

## Component architecture

```
┌─ chrome-hara (MV3 extension) ──────────────────────────────────────┐
│ DevTools panel page (extension page)                               │
│   studio UI (mountStudio) ── KernelBroker                          │
│        │        ▲ host/call events                                 │
│        │   kernels: hta-worker.js + hara.wasm (one worker each)    │
│        │   hostCalls = studio services (IndexedDB/fetch, in-panel) │
│        │             merged over port proxy (chrome.*, hara/echo)  │
│        ▼        │                                                  │
│ Service worker: chrome.* host-call implementations                 │
│   chrome.debugger.attach/sendCommand, tabs, generic chrome proxy   │
└────────────────────────────────────────────────────────────────────┘
   ▲ outbound WebSocket (panel dials out, ?resp=ws://...)
   │
chrome-hara bridge (node): RESP TCP server ⟷ WS ⟷ extension
```

- The panel hosts the wasm workers (MV3 service workers cannot spawn nested
  workers) and mounts the shared studio environment (`rust/web/studio/`):
  a `KernelBroker` owns one worker per kernel, and `mountStudio` renders
  the space/file/editor/REPL UI. The service worker owns `chrome.debugger`
  and the generic `chrome.*` proxy; only those calls travel over the
  long-lived `chrome.runtime` Port — the studio store/http/json services
  are answered in-panel. Panel lifetime = automation session lifetime.
- Manifest: `devtools_page`, permissions `debugger`, `tabs`, `storage`;
  CSP `extension_pages: script-src 'self' 'wasm-unsafe-eval'`.
- `(require [chrome.api :as api])` works because core eval gained a
  thread-local namespace-source provider (`with_namespace_source` in
  `rust/src/core.rs`) and the raw runtime gained a `register-resource`
  HTA target; the panel registers `src/hara/api.hal` at startup.
- Home directory: `showDirectoryPicker()` + a require preloader that
  resolves `ns.name` → `<home>/<source-path>/ns/name.hal` (`-` → `_`,
  `project.hal` `:source-paths`, mirroring the JVM `HaraProject`
  convention) and registers sources before eval.
- RESP: MV3 extensions cannot listen on TCP, so `bridge/resp-bridge.mjs`
  (node) bridges RESP2 TCP ⟷ WebSocket; the panel dials the WS outbound.

## Trust boundaries

- The RESP bridge (127.0.0.1) gives any local process full hara eval in the
  extension, which transitively means full `chrome.debugger`/CDP control of
  open tabs — comparable to launching Chrome with `--remote-debugging-port`.
  It is opt-in: the user must start the bridge AND open the panel with
  `?resp=ws://...`.
- Extension-side: no `externally_connectable`, no content scripts — only the
  extension's own pages can reach the service worker, and the generic proxy
  is bounded by the declared permissions (`debugger`, `tabs`, `storage`).
- Chrome shows its "debugging this browser" infobar while `chrome.debugger`
  is attached — inherent to the API.

## Known limitations

- HTA1 has no float tag: fractional CDP results are truncated to integers
  (`sanitize()` in `src/background.js`).
- `requireSpecs` is a scanner, not a reader — it can over-match inside
  strings/comments (harmless: extra registrations).
- RESP bridge evals do not preload requires from the home directory.
- Required namespaces must be load-time pure (no top-level `deref` of a
  pending `host/call` during a `require` load).
- The RESP bridge is a subset (no v4 streaming, no sessions) and requires an
  open hara panel.
