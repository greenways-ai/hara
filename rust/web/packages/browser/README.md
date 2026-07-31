# @hara-lang/browser

Embeddable Hara runtime for browsers and CDN scripts.

```js
import { start } from "@hara-lang/browser";

const hara = await start();
console.log(hara.eval("(+ 19 23)"));
```

The release also provides an IIFE bundle for a plain script tag:

```html
<script src="https://unpkg.com/@hara-lang/browser@0.1.0/dist/hara.js"></script>
<script>
  Hara.start().then((hara) => console.log(hara.eval("(+ 19 23)")));
</script>
```

The Hara HAL catalog is embedded in the Wasm runtime. Host resources can be
registered before requiring them:

```js
const hara = await Hara.start({
  resources: {
    "app.config": "(ns app.config) (def answer 42)"
  }
});
```
