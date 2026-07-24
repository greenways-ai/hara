---
template: home.html
title: Hara
hide:
  - navigation
  - toc
  - feedback
---

<div class="hara-home-actions">
  <a class="md-button md-button--primary" href="studio/">Try Hara</a>
  <a class="md-button" href="user-guide/">Start building</a>
</div>

<div class="hara-home-grid">
  <a class="hara-home-card" href="reference/l0-language/">
    <span class="hara-card-index">01 / SMALL</span>
    <strong>Small core.<br><em>Wide horizon.</em></strong>
    <span>A handful of forms, designed once and held — values, protocols, promises, explicit boundaries.</span>
  </a>
  <a class="hara-home-card" href="reference/runtime-benchmarks/">
    <span class="hara-card-index">02 / FAST</span>
    <strong>JIT-warm.<br><em>Wasm-quick.</em></strong>
    <span>Truffle compiles hot paths on the JVM; the Rust runtime keeps wasm eval near-native.</span>
  </a>
  <a class="hara-home-card" href="development/">
    <span class="hara-card-index">03 / EMBEDDABLE</span>
    <strong>Drops in<br><em>anywhere.</em></strong>
    <span>A JVM library, a browser module, a Chrome panel — the same language behind every host.</span>
  </a>
  <a class="hara-home-card" href="reference/extensions-contract/">
    <span class="hara-card-index">04 / WASM NATIVE</span>
    <strong>One require.<br><em>Any world.</em></strong>
    <span>Portable wasm extensions load by namespace — no changes at the call site.</span>
  </a>
</div>

<div class="hara-home-section">
  <div class="hara-kicker">04 / TRY HARA</div>
  <div class="hara-frame hara-console">
    <div class="hara-console-head">
      <span>REPL — ONE REQUIRE, ANY WORLD</span>
      <span class="hara-index">TTY/00 · LIVE</span>
    </div>
    <pre class="hara-tty"><span class="hara-tty-p">hara ›</span> (require '[demo.000-answer-42 :as answer])  <span class="hara-tty-o">;; core wasm · abi core.v1</span>
<span class="hara-tty-v">=&gt; :loaded</span>
<span class="hara-tty-p">hara ›</span> (answer/add 19 23)
<span class="hara-tty-v">=&gt; 42</span>
<span class="hara-tty-p">hara ›</span> (require '[crypto.hash.sha256 :as sha])       <span class="hara-tty-o">;; wasm · abi hta.v1</span>
<span class="hara-tty-v">=&gt; :loaded</span>
<span class="hara-tty-p">hara ›</span> (sha/digest (.bytes "grid"))
<span class="hara-tty-v">=&gt; #&lt;promise :pending&gt;</span>
<span class="hara-tty-p">hara ›</span> (require '[chrome.api :as api])               <span class="hara-tty-o">;; host extension</span>
<span class="hara-tty-v">=&gt; :loaded</span></pre>
    <div class="hara-strip">
      <span>CORE.V1 <b>LOADED</b></span>
      <span>HTA.V1 <b>LOADED</b></span>
      <span>HOST <b>LOADED</b></span>
      <span>STATE <b>LIVE</b></span>
    </div>
  </div>
</div>

<div class="hara-home-section">
  <div class="hara-frame hara-cta">
    <div class="hara-kicker">05 / TRY HARA</div>
    <div class="hara-home-actions">
      <a class="md-button md-button--primary" href="studio/">Try Hara</a>
      <a class="md-button" href="the-little-book-of-hal/">The Little Book of HAL</a>
    </div>
  </div>
</div>
