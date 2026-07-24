---
template: home.html
title: Hara
hide:
  - navigation
  - toc
  - feedback
---

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

<div class="hara-home-section">
  <div class="hara-kicker">04 / LIVE SESSION</div>
  <div class="hara-frame hara-console">
    <div class="hara-console-head">
      <span>REPL — HARA/WASM</span>
      <span class="hara-index">TTY/00 · LIVE</span>
    </div>
    <pre class="hara-tty"><span class="hara-tty-p">hara ›</span> (ns live.system
          (:require [std.lib.promise :as promise]))

<span class="hara-tty-p">hara ›</span> (promise/then (promise/run discover) render)
<span class="hara-tty-v">=&gt; #&lt;promise :pending&gt;</span>
<span class="hara-tty-o">;; the system stays up. the code keeps moving.</span></pre>
    <div class="hara-strip">
      <span>RUNTIME <b>WASM</b></span>
      <span>EVAL <b>0.4MS</b></span>
      <span>STATE <b>LIVE</b></span>
      <span>UPTIME <b>∞</b></span>
    </div>
  </div>
</div>

<div class="hara-home-section">
  <div class="hara-frame hara-cta">
    <div class="hara-kicker">05 / ENTER THE GRID</div>
    <p>The grid is patient. Bring a form, leave with a system.</p>
    <div class="hara-home-actions">
      <a class="md-button md-button--primary" href="user-guide/">Start building</a>
      <a class="md-button" href="books/the-little-book-of-hal/">The Little Book of HAL</a>
    </div>
  </div>
</div>
