# @hara-lang/noir

Noir compile, prove, and verify adaptation for Hara providers.

```js
import { callNoir } from "@hara-lang/noir";

const artifact = await callNoir(loaderUrl, "compile", [{
  name: "balance",
  source: "fn main() {}"
}]);
```

Provider transports belong to `@hara-lang/hta`; production workers bundle
both packages through the Hara web build.
