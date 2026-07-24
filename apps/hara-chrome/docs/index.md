# hara-chrome

A Chrome MV3 DevTools extension that embeds the hara wasm runtime in a
panel REPL and exposes the Chrome API to hara:

```clojure
(require [chrome.api :as api])
```

Features:

- **Panel REPL** — the raw hara wasm runtime in a Web Worker; host calls
  round-trip to the service worker where real `chrome.*` calls execute.
- **`chrome.api`** — the full Chrome API surface, callable from hara
  (including `chrome.debugger`/CDP for automation scripts).
- **Home directory** — pick a local directory and `load`/`require` `.hal`
  files from it.
- **RESP endpoint** — a local bridge (RESP TCP ⟷ WebSocket) lets external
  tooling evaluate hara inside the extension.

For development details see the
[repository](https://github.com/hoebat/hara.lang/tree/main/apps/hara-chrome).
