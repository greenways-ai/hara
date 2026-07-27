---
template: home.html
title: Hara
hide:
  - navigation
  - toc
  - feedback
---

<section class="hara-home-section hara-home-intro">
  <p class="hara-label">01 / WHY HARA</p>
  <div>
    <h2>One language. Clear boundaries.</h2>
    <p>Hara keeps its core deliberately small. Host access, files, sockets and other effects are explicit capabilities rather than ambient magic, making programs easier to move, embed and reason about.</p>
  </div>
</section>

<section class="hara-home-section">
  <p class="hara-label">02 / THE SHAPE</p>
  <div class="hara-feature-grid">
    <article>
      <span>01</span>
      <h3>Compact L0 core</h3>
      <p>Functions, persistent collections, protocols, promises and explicit mutable markers form a focused language surface.</p>
    </article>
    <article>
      <span>02</span>
      <h3>Runtime-neutral contracts</h3>
      <p>The same value model and library boundaries are specified across Truffle, Rust and WebAssembly hosts.</p>
    </article>
    <article>
      <span>03</span>
      <h3>Built for embedding</h3>
      <p>Use Hara from a CLI, a JVM application, a browser module, an editor or a purpose-built host.</p>
    </article>
    <article>
      <span>04</span>
      <h3>Live development</h3>
      <p>A shared REPL session, RESP transport and editor integrations keep evaluation close to the running system.</p>
    </article>
  </div>
</section>

<section class="hara-home-section hara-quickstart">
  <div>
    <p class="hara-label">03 / QUICK START</p>
    <h2>From source to REPL.</h2>
    <p>Build the Truffle runtime with JDK 21 and Maven, evaluate a form, then enter the shared interactive session.</p>
  </div>
  <pre><code>mvn -f java/pom.xml -Ptruffle package
./hara eval '(+ 19 23)'
./hara</code></pre>
</section>

<section class="hara-home-section hara-paths">
  <p class="hara-label">04 / GO DEEPER</p>
  <div class="hara-path-grid">
    <a href="user-guide/">
      <span>Learn</span>
      <strong>User guide</strong>
      <p>Install, evaluate, use the REPL and write Hara programs.</p>
    </a>
    <a href="reference/l0-language/">
      <span>Specify</span>
      <strong>L0 language</strong>
      <p>Read the normative core language behaviour and data model.</p>
    </a>
    <a href="reference/rust-runtime/">
      <span>Embed</span>
      <strong>Rust and WASM</strong>
      <p>Understand cross-runtime values, providers and conformance.</p>
    </a>
    <a href="development/">
      <span>Contribute</span>
      <strong>Developer guide</strong>
      <p>Build, test, benchmark and extend the implementation.</p>
    </a>
  </div>
</section>
