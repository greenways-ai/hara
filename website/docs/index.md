---
template: home.html
title: Hara
hide:
  - navigation
  - toc
  - feedback
---

Hara is a small, runtime-neutral language for live systems.

<div class="hara-home-actions">
  <a class="md-button md-button--primary" href="user-guide/">Start building</a>
  <a class="md-button" href="development/">Read the developer guide</a>
</div>

<div class="hara-home-grid">
  <a class="hara-home-card" href="user-guide/">
    <span class="hara-card-index">01 / LANGUAGE</span>
    <strong>Small core.<br><em>Wide horizon.</em></strong>
    <span>Learn the values, forms, protocols, promises, and explicit boundaries.</span>
  </a>
  <a class="hara-home-card" href="reference/extensions-contract/">
    <span class="hara-card-index">02 / EXTENSIONS</span>
    <strong>One require.<br><em>Any world.</em></strong>
    <span>Load portable WASM extensions without changing the Hara call site.</span>
  </a>
  <a class="hara-home-card" href="javadocs/">
    <span class="hara-card-index">03 / RUNTIME</span>
    <strong>Build systems<br><em>that stay alive.</em></strong>
    <span>Explore the Truffle runtime, embedding boundary, and Java API.</span>
  </a>
</div>

```clojure
(ns live.system
  (:require [std.lib.promise :as promise]))

(promise/then (promise/run discover) render)
```
