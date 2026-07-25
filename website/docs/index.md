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

<div class="hara-home-section hara-glass hara-glass--dark">
  <div class="hara-kicker">04 / TRY HARA</div>
  <div id="hara-console" class="hara-frame hara-console" data-hara-component="console">
    <div class="hara-console-head">
      <span>REPL — LIVE EVAL · HARA.WASM</span>
      <span class="hara-index">TTY/00 · LIVE</span>
    </div>
    <div class="hara-console-log" data-console-log>
      <pre class="hara-tty"><span class="hara-tty-o">;; the real runtime — every form below evaluates in your browser</span>
<span class="hara-tty-p">hara ›</span> (+ 19 23)
<span class="hara-tty-v">=&gt; 42</span></pre>
    </div>
    <div class="hara-console-entry">
      <span class="hara-tty-p">hara ›</span>
      <input class="hara-console-input" data-console-input type="text"
             placeholder="(let (x 19) (+ x 23))" autocomplete="off"
             spellcheck="false" aria-label="hara eval input">
    </div>
    <div class="hara-command-deck" aria-label="Hara commands">
      <span class="hara-command-label">COMMANDS</span>
      <button type="button" class="hara-command" data-console-command="(+ 19 23)">ADD / 42</button>
      <button type="button" class="hara-command" data-console-command="(let (x 7) (* x 6))">BIND / 42</button>
      <button type="button" class="hara-command" data-console-command="(if true 1 0)">BRANCH / 1</button>
    </div>
    <div class="hara-strip">
      <span>RUNTIME <b data-status="runtime">WASM · LOADING</b></span>
      <span>FILE <b data-status="file">—</b></span>
      <span>SOCKET <b data-status="socket">—</b></span>
      <span>STATE <b data-status="state">BOOT</b></span>
    </div>
  </div>
</div>

<div class="hara-home-section hara-glass hara-glass--light">
  <div class="hara-frame hara-cta">
    <div class="hara-kicker">05 / TRY HARA</div>
    <div class="hara-home-actions">
      <a class="md-button md-button--primary" href="studio/">Try Hara</a>
      <a class="md-button" href="the-little-book-of-hal/">The Little Book of HAL</a>
    </div>
  </div>
</div>
