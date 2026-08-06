# @hara-lang/hta

Portable HTA1 codecs, manifests, browser host contexts, and provider
transports.

```js
import { decodeHta, encodeHta } from "@hara-lang/hta";
import { serveNodeProvider } from "@hara-lang/hta/provider/node";
import { serveBrowserProvider } from "@hara-lang/hta/provider/browser";
```

The provider helpers accept an async `(operation, arguments) => value`
function and implement HTA framing, cancellation, result encoding, and
structured errors for their respective runtime.

The `@hara-lang/hta/worker` and `@hara-lang/hta/shared-worker` exports provide
the raw Wasm worker entry points. The repository-level `hta*.js` files are
compatibility shims for existing static URLs.
