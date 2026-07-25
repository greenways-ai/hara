# hara-chrome

Chrome (MV3) extension embedding the hara wasm runtime in a DevTools panel.
The panel mounts the shared studio environment (`rust/web/studio/` — kernel
broker, spaces/fs over IndexedDB, studio UI), with the chrome-specific powers
layered on top: `chrome.api` host calls route to the background service
worker, and home-directory / run-file actions live in the strip above the
studio. REPL and RESP evals go to the active kernel.

## Build

    npm install
    npm run build   # builds rust/raw + copies vendor files

## Load

`chrome://extensions` -> developer mode -> "Load unpacked" -> select this directory.
Open DevTools -> the "hara" panel.

## Test

    npm test               # node unit tests
    npm run test:browser   # playwright (uses xvfb-run)

## RESP bridge

    node bridge/resp-bridge.mjs        # resp=7355, ws=7356

Open the panel with `?resp=ws://127.0.0.1:7356` appended (or call
`connectResp(url, hara.evalSource)` from the console), then:

    redis-cli -p 7355 EVAL "(+ 1 2)"

## Home directory

The panel can load `.hal` sources from a local directory:

- "choose home" picks a directory (persisted in IndexedDB; restored on load
  when permission is still granted).
- A `project.hal` in that directory supplies `:source-paths` (defaults to `["."]`),
  which is where `require`d namespaces are resolved from.
- "run .hal file" evaluates a single file from the home directory.

## Trust boundaries

The RESP bridge binds to 127.0.0.1, but any local process can connect to it.
A connected client gets full hara eval inside the extension, which
transitively means full `chrome.debugger`/CDP control of open tabs —
comparable to launching Chrome with `--remote-debugging-port`. The bridge is
opt-in: it only evals when you start `bridge/resp-bridge.mjs` AND open the
panel with `?resp=` (or call `connectResp` manually).

## Known limitations

- Fractional/float CDP results are truncated to integers: the HTA1 wire
  format has no float tag, so `sanitize()` in the service worker coerces
  non-safe-integer numbers (`Math.trunc`, non-finite becomes 0).
- `requireSpecs` is a naive scanner, not a reader: it may over-match require
  specs inside strings/comments. Over-matching is harmless — it only causes
  extra namespace registrations.
- RESP bridge evals do not preload `require`s from the home directory; only
  namespaces already registered in the runtime are visible.
- Required namespaces must be load-time pure: no top-level deref of a pending
  host/call, since registration happens before any host call can resolve.
