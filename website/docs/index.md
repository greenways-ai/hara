---
template: home.html
title: Hara
hide:
  - navigation
  - toc
  - feedback
---

<div class="hara-home-section hara-glass hara-glass--dark-soft">
  <div class="hara-kicker">00 / DEMOS</div>
  <div class="hara-demo-grid">
    <div class="hara-frame hara-demo" data-hara-demo="synth">
      <div class="hara-console-head">
        <span>SYNTH — WASM · SELF-HOSTED</span>
        <span class="hara-index">DEMO/01</span>
      </div>
      <canvas class="hara-demo-canvas" data-demo-canvas></canvas>
      <div class="hara-demo-actions">
        <button type="button" class="md-button md-button--primary" data-demo-run>Run</button>
        <span class="hara-demo-note">demo-synth.wasm · rust/extensions/demo-synth</span>
      </div>
      <div class="hara-strip">
        <span>WASM <b data-status="wasm">IDLE</b></span>
        <span>AUDIO <b data-status="audio">IDLE</b></span>
        <span>STATE <b data-status="state">IDLE</b></span>
      </div>
    </div>
    <div class="hara-frame hara-demo" data-hara-demo="csound">
      <div class="hara-console-head">
        <span>CSOUND — WASM · FROM CDN</span>
        <span class="hara-index">DEMO/02</span>
      </div>
      <canvas class="hara-demo-canvas" data-demo-canvas></canvas>
      <div class="hara-demo-actions">
        <button type="button" class="md-button md-button--primary" data-demo-run>Run</button>
        <span class="hara-demo-note">@csound/browser · jsdelivr</span>
      </div>
      <div class="hara-strip">
        <span>WASM <b data-status="wasm">IDLE</b></span>
        <span>AUDIO <b data-status="audio">IDLE</b></span>
        <span>STATE <b data-status="state">IDLE</b></span>
      </div>
    </div>
  </div>
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

<div class="hara-home-section hara-glass hara-glass--light">
  <div class="hara-frame hara-cta">
    <div class="hara-kicker">04 / EXPLORE</div>
    <div class="hara-home-actions">
      <a class="md-button md-button--primary" href="studio/">Try Hara</a>
      <a class="md-button" href="the-little-book-of-hal/">The Little Book of HAL</a>
    </div>
  </div>
</div>
