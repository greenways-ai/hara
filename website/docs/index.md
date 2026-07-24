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
