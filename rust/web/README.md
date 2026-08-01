# rust/web

Browser-side loaders and UIs for the hara wasm runtimes, served as static
assets. The pages deploy copies the runtime-facing pieces under
`site-build/rust/` (see `.github/workflows/pages-www.yml`).

## Pieces

- `packages/browser/` — the publishable `@hara-lang/browser` SDK. It wraps the
  wasm-bindgen runtime, exposes `Hara.start()` for ESM and CDN script embeds,
  and carries the generated HAL catalog in its release bundle.
- `packages/hta/` — the publishable `@hara-lang/hta` package: HTA1 codecs,
  browser hosts, and reusable Node/browser provider transports. `hta.js`
  remains a compatibility re-export for static browser consumers.
- `packages/noir/` — the publishable `@hara-lang/noir` compile/prove/verify
  adapter. The Noir extension consumes it through generated worker entries.
- `hta-worker.js` — the raw HTA worker: `HtaContext` drives one
  raw wasm instance (`rust/raw`) inside a Web Worker over the `HTA1` binary
  wire format, with handles and the promise-provider contract
  (`specs/01-lang/008-hta/draft/hal-hta-contract.md`).
- `index.html` / `playground.js` — the wasm-bindgen playground page
  (in-browser runtime plus Noir proving).
- `noir-loader.js` — Noir circuit loader/backends for the playground and the
  Noir extension. `entries/noir-*.mjs` combine `@hara-lang/hta` and
  `@hara-lang/noir` into self-contained provider workers;
  `build:package:noir` builds the deterministic HARP archive.
- Greenways Studio is maintained in `greenways-ai/studio-tooling`; the website
  build consumes its digest-pinned HARP packages through
  `scripts/fetch-greenways-studio`.

## Test

    npm run test:hta       # HTA loader unit tests
    npm run test:noir      # builds + tests the noir loader
    npm run test:browser   # playwright browser smoke
