# HARA website redesign implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Transform `hara.lang` into a live, browser-first coding environment with a universal glass system bar, a simplified Play / Learn / Kernel / Docs structure, and a homepage that lets visitors evaluate Hara immediately.

**Architecture:** Keep the existing Material for MkDocs site but override its header with a custom system bar. Move the Tron grid game to a fixed background layer and remove its parallax transform. Reuse the existing wasm console runtime for both the system-bar command palette and the LOG panel. Restructure `mkdocs.yml` navigation and add a new Kernel page.

**Tech Stack:** Material for MkDocs, Jinja2 templates, CSS in `website/docs/stylesheets/hara.css`, JS in `website/docs/javascripts/console.js`, Hara wasm runtime in `website/docs/rust/pkg/`.

## Global constraints

- All edits are under `website/`.
- Build must pass: `/tmp/docs-venv/bin/mkdocs build --strict -f website/mkdocs.yml --site-dir /tmp/site-k`.
- The Tron grid game must remain visible while scrolling.
- The top bar must appear on every page.
- No greenways.ai branding yet.
- Tagline on the homepage: **"A Modern Lisp for the Agentic Age."**

---

## Task 1: Fix the game canvas parallax

**Files:**
- Modify: `website/docs/stylesheets/hara.css`

**What:** Remove the `transform` on `.hara-game` so the fixed canvas does not slide out of view on scroll or pointer move.

- [ ] **Step 1: Remove the parallax transform**

  Replace the `.hara-game` rule with:

  ```css
  .hara-game {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
  }
  ```

- [ ] **Step 2: Build and screenshot**

  Run:
  ```bash
  /tmp/docs-venv/bin/mkdocs build --strict -f website/mkdocs.yml --site-dir /tmp/site-k
  ```

  Then load `/tmp/site-k/index.html` in a headless browser, scroll down 700 px, and verify the grid is still visible.

- [ ] **Step 3: Commit**

  ```bash
  git add website/docs/stylesheets/hara.css
  git commit -m "fix(website): pin game canvas so it stays visible while scrolling"
  ```

---

## Task 2: Create a reusable system-bar partial

**Files:**
- Create: `website/overrides/partials/system-bar.html`
- Modify: `website/overrides/main.html`
- Modify: `website/overrides/home.html`

**What:** Extract the current `hara-system-bar` markup into a Jinja partial and render it from `main.html` so it appears on every page. Remove the duplicate bar from `home.html`.

### Step 1: Create the partial

- [ ] **Create `website/overrides/partials/system-bar.html`** with the new structure:

  ```html
  <header class="hara-system-bar" data-hara-component="console" data-md-component="header">
    <a class="hara-system-brand" href="{{ config.site_url | default(nav.homepage.url, true) }}">
      <svg class="hara-system-apple" viewBox="0 0 24 24" aria-hidden="true">
        <defs>
          <linearGradient id="appleGrad" x1="0" y1="0" x2="1" y2="1">
            <stop offset="0%" stop-color="#8ffff2"/>
            <stop offset="100%" stop-color="#41f5e4"/>
          </linearGradient>
        </defs>
        <path d="M18.7 8.1c-.8 0-1.6.3-2.3.3-.8 0-1.6-.3-2.5-.3-2.4 0-4.5 2.1-4.5 5.4 0 2.1.8 4.3 1.9 5.7 1 1.3 2 2.2 3.2 2.2.9 0 1.3-.6 2.5-.6 1.1 0 1.5.6 2.4.6 1.3 0 2.3-1.1 3.2-2.4.9-1.3 1.3-2.6 1.3-2.7 0-.1-2.4-.9-2.4-4 0-2.3 1.9-3.5 2-3.6-1.2-1.7-3-1.8-3.6-1.8-.6 0-1.5.2-2 .2zm-1.8-2.3c.9-1.1 1.5-2.6 1.3-4.1-1.3.1-2.8.9-3.7 1.9-.8.9-1.5 2.4-1.3 3.8 1.4.1 2.8-.8 3.7-1.6z"/>
      </svg>
      <span>HARA</span>
    </a>

    <nav class="hara-system-nav" aria-label="Primary">
      <a href="{{ nav.homepage.url | url }}">Play</a>
      <a href="{{ 'learn' | url }}">Learn</a>
      <a href="{{ 'kernel' | url }}">Kernel</a>
      <a href="{{ 'reference/l0-language' | url }}">Docs</a>
    </nav>

    <div class="hara-system-palette">
      <span class="hara-system-palette__prompt">>_</span>
      <input type="text" data-console-palette
             placeholder="try a hara form" autocomplete="off"
             spellcheck="false" aria-label="Hara command palette">
    </div>

    <div class="hara-system-runtime">
      <span class="hara-system-led" data-runtime-led></span>
    </div>

    <button class="hara-system-log-toggle" type="button" data-console-toggle
            aria-expanded="false" aria-controls="hara-system-console">
      LOG <span data-console-count>0</span>
    </button>

    <button class="hara-system-panel-toggle" type="button" data-panel-toggle
            aria-expanded="false" aria-controls="hara-system-panel"
            aria-label="Apps and system controls">
      <svg class="hara-system-dots" viewBox="0 0 16 16" aria-hidden="true">
        <circle cx="3" cy="3" r="1.5"/><circle cx="8" cy="3" r="1.5"/><circle cx="13" cy="3" r="1.5"/>
        <circle cx="3" cy="8" r="1.5"/><circle cx="8" cy="8" r="1.5"/><circle cx="13" cy="8" r="1.5"/>
        <circle cx="3" cy="13" r="1.5"/><circle cx="8" cy="13" r="1.5"/><circle cx="13" cy="13" r="1.5"/>
      </svg>
    </button>

    <section id="hara-system-console" class="hara-system-console" data-console-panel
             aria-label="Hara evaluation log" aria-hidden="true">
      <div class="hara-system-console__head">
        <span>SYSTEM LOG / HARA.WASM</span>
        <button type="button" data-console-close aria-label="Close evaluation log">×</button>
      </div>
      <div class="hara-console-log" data-console-log role="log" aria-live="polite">
        <div class="hara-console-line hara-tty-o">;; runtime boot requested</div>
      </div>
      <label class="hara-console-entry">
        <span class="hara-tty-p">hara ›</span>
        <input class="hara-console-input" data-console-input type="text"
               placeholder="(let (x 19) (+ x 23))" autocomplete="off"
               spellcheck="false" aria-label="Hara eval input">
      </label>
      <div class="hara-strip">
        <span>FILE <b data-status="file">—</b></span>
        <span>SOCKET <b data-status="socket">—</b></span>
        <span>STATE <b data-status="state">BOOT</b></span>
      </div>
    </section>

    <aside id="hara-system-panel" class="hara-system-panel" data-system-panel
           aria-label="System control panel" aria-hidden="true">
      <div class="hara-system-panel__head">
        <span>CONTROL CENTER</span>
        <button type="button" data-panel-close aria-label="Close control panel">×</button>
      </div>
      <div class="hara-system-panel__body">
        <div class="hara-system-tile">
          <span class="hara-system-tile__label">Runtime</span>
          <span class="hara-system-tile__value" data-status="runtime">WASM · LOADING</span>
        </div>
        <div class="hara-system-tile">
          <span class="hara-system-tile__label">Grid</span>
          <span class="hara-system-tile__value">Live</span>
        </div>
        <div class="hara-system-tile">
          <span class="hara-system-tile__label">Theme</span>
          <span class="hara-system-tile__value">Noir</span>
        </div>
        <div class="hara-system-panel__section">
          <span class="hara-system-panel__section-title">Apps</span>
          <a class="hara-system-tile" href="{{ 'hara-chrome' | url }}">hara-chrome</a>
          <a class="hara-system-tile" href="{{ 'hara-for-emacs' | url }}">hara-emacs</a>
        </div>
        <div class="hara-system-panel__section">
          <span class="hara-system-panel__section-title">Books</span>
          <a class="hara-system-tile" href="{{ 'the-little-book-of-hal' | url }}">The Little Book of HAL</a>
        </div>
        <div class="hara-system-panel__section">
          <span class="hara-system-panel__section-title">Notifications</span>
          <div class="hara-system-notify">hara.wasm ready</div>
          <div class="hara-system-notify">3 light cycles active</div>
        </div>
      </div>
    </aside>
  </header>
  ```

### Step 2: Wire the partial into `main.html`

- [ ] **Modify `website/overrides/main.html`** to override the `header` block:

  ```html
  {#-
    Site-wide theme extension: universal glass system bar + fixed Tron grid background.
  -#}
  {% extends "base.html" %}
  {% block header %}
    {% include "partials/system-bar.html" %}
  {% endblock %}
  {% block site_nav %}
  <div class="hara-bg" data-hara-component="parallax" data-hara-fixed aria-hidden="true">
    <canvas class="hara-game" data-hara-component="game"></canvas>
  </div>
  {{ super() }}
  {% endblock %}
  ```

### Step 3: Remove the duplicate bar from `home.html`

- [ ] **Modify `website/overrides/home.html`:**
  - Remove the entire `<header class="hara-system-bar" ...> ... </header>` block.
  - Remove `{% block header %}{% endblock %}` and `{% block tabs %}{% endblock %}` since `main.html` now supplies the header.
  - Keep the hero, intro window, and content wrapper.

- [ ] **Step 4: Build**

  ```bash
  /tmp/docs-venv/bin/mkdocs build --strict -f website/mkdocs.yml --site-dir /tmp/site-k
  ```

- [ ] **Step 5: Commit**

  ```bash
  git add website/overrides/main.html \
          website/overrides/home.html \
          website/overrides/partials/system-bar.html
  git commit -m "feat(website): universal system-bar partial"
  ```

---

## Task 3: Restyle the system bar

**Files:**
- Modify: `website/docs/stylesheets/hara.css`

**What:** Make the bar a minimal 48 px glass bar: HARA brand, nav links, command palette, status, LOG, 9-dot launcher. Fix the launcher clipping.

- [ ] **Step 1: Rewrite the `.hara-system-bar` rule**

  ```css
  .hara-system-bar {
    position: fixed;
    z-index: 20;
    top: 0;
    left: 0;
    right: 0;
    height: 48px;
    display: grid;
    grid-template-columns: auto auto 1fr auto auto auto;
    align-items: stretch;
    box-sizing: border-box;
    color: #c6d4e3;
    background: rgba(3, 5, 10, .82);
    border-bottom: 1px solid var(--hara-line);
    box-shadow: 0 12px 36px rgba(0, 0, 0, .3);
    -webkit-backdrop-filter: blur(20px) saturate(1.3);
    backdrop-filter: blur(20px) saturate(1.3);
  }
  ```

- [ ] **Step 2: Add nav link styles**

  Append after the brand styles:

  ```css
  .hara-system-nav {
    display: flex;
    align-items: stretch;
  }

  .hara-system-nav a {
    display: flex;
    align-items: center;
    padding: 0 .85rem;
    color: #aabaca;
    font-family: var(--hara-mono);
    font-size: .58rem;
    font-weight: 700;
    letter-spacing: .12em;
    text-transform: uppercase;
    text-decoration: none;
    transition: color .15s, background .15s;
  }

  .hara-system-nav a:hover,
  .hara-system-nav a:focus-visible {
    color: var(--hara-cyan-bright);
    background: rgba(65, 245, 228, .08);
    outline: none;
  }
  ```

- [ ] **Step 3: Add command palette styles**

  ```css
  .hara-system-palette {
    display: flex;
    align-items: center;
    gap: .5rem;
    justify-self: center;
    min-width: 280px;
    max-width: 520px;
    width: 40vw;
    padding: 0 .75rem;
    margin: 6px 0;
    color: #8b9db0;
    background: rgba(2, 4, 8, .55);
    border: 1px solid var(--hara-line);
    border-radius: 6px;
    font-family: var(--hara-mono);
    font-size: .62rem;
    transition: border-color .15s, box-shadow .15s;
  }

  .hara-system-palette:focus-within {
    border-color: rgba(65, 245, 228, .45);
    box-shadow: 0 0 12px rgba(65, 245, 228, .12);
  }

  .hara-system-palette__prompt {
    color: var(--hara-cyan);
  }

  .hara-system-palette input {
    flex: 1;
    min-width: 0;
    color: #c6d4e3;
    background: transparent;
    border: 0;
    font: inherit;
    letter-spacing: .02em;
    outline: none;
  }

  .hara-system-palette input::placeholder {
    color: #5a6f82;
  }
  ```

- [ ] **Step 4: Fix the 9-dot launcher clipping**

  Update `.hara-system-panel-toggle`:

  ```css
  .hara-system-panel-toggle {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 44px;
    margin-right: 8px;
    border: 0;
    border-left: 1px solid var(--hara-line);
    background: transparent;
    color: #aabaca;
    cursor: pointer;
  }

  .hara-system-panel-toggle svg {
    width: 16px;
    height: 16px;
    fill: currentColor;
  }
  ```

- [ ] **Step 5: Build and screenshot**

  ```bash
  /tmp/docs-venv/bin/mkdocs build --strict -f website/mkdocs.yml --site-dir /tmp/site-k
  ```

  Verify the bar renders on `/tmp/site-k/index.html`, `/tmp/site-k/learn/index.html`, and `/tmp/site-k/reference/l0-language/index.html`.

- [ ] **Step 6: Commit**

  ```bash
  git add website/docs/stylesheets/hara.css
  git commit -m "feat(website): minimal glass system bar styling"
  ```

---

## Task 4: Wire the top-bar command palette

**Files:**
- Modify: `website/docs/javascripts/console.js`

**What:** Allow the system-bar input to evaluate Hara forms through the same runtime and stream output to the LOG panel.

- [ ] **Step 1: Select the palette input and attach keydown**

  Near the existing input selector, add:

  ```js
  const paletteInput = document.querySelector('[data-console-palette]');
  ```

  After the existing `input.addEventListener('keydown', ...)` block, add:

  ```js
  if (paletteInput) {
    paletteInput.addEventListener('keydown', async (event) => {
      if (event.key !== 'Enter') return;
      const source = paletteInput.value.trim();
      paletteInput.value = '';
      await evaluate(source);
    });
  }
  ```

- [ ] **Step 2: Build and test**

  Build, load the site, click the top-bar palette, type `(+ 19 23)`, press Return, and verify the LOG panel opens and shows `=> 42`.

- [ ] **Step 3: Commit**

  ```bash
  git add website/docs/javascripts/console.js
  git commit -m "feat(website): top-bar command palette evaluates hara forms"
  ```

---

## Task 5: Restructure navigation and add the Kernel page

**Files:**
- Modify: `website/mkdocs.yml`
- Create: `website/docs/kernel.md`
- Create: `website/docs/learn.md` (optional landing for Learn section)
- Create: `website/docs/reference/index.md` (optional landing for Docs section)

**What:** Simplify the public nav to Play / Learn / Kernel / Docs, with Apps and Books surfaced through the 9-dot launcher.

- [ ] **Step 1: Create `website/docs/kernel.md`**

  ```markdown
  # Kernel

  HARA is not only a language — it is a small Lisp kernel that runs in the browser.

  ## Sessions

  Local browser sessions backed by the HARA wasm runtime. Each session carries its own state, namespace, and evaluation history.

  ## State transitions

  Every meaningful change in a session is logged as a transition. These transitions can be replayed, verified, and shared without exposing the full session record.

  ## Agents

  Small autonomous HARA programs that interact through the kernel. Agents can compete, mediate, or collaborate inside a shared session.

  ## Proofs

  A lightweight proof system lets two parties verify that a session transitioned correctly without revealing the code or data that produced the transition.

  ## Competitions, mediation, and collaboration

  The kernel provides primitives for:

  - **Competitions** — agents compete in a sandboxed environment (e.g., multiplayer TRON).
  - **Mediation** — a neutral party validates transitions between untrusted participants.
  - **Collaboration** — multiple identities contribute to a shared session while keeping their local state consistent.
  ```

- [ ] **Step 2: Update `website/mkdocs.yml` nav**

  Replace the existing `nav:` block with:

  ```yaml
  nav:
    - Play: index.md
    - Learn:
        - Overview: learn.md
        - User guide: user-guide.md
        - Getting started: getting-started.md
        - Namespaces and modules: namespaces.md
        - Walkthroughs:
            - Namespace project: walkthroughs/service-project.md
            - Library namespaces: walkthroughs/libraries.md
            - Testing namespaces: walkthroughs/testing.md
    - Kernel: kernel.md
    - Docs:
        - Overview: reference/index.md
        - L0 language: reference/l0-language.md
        - Builtins: builtins.md
        - Namespace catalog: reference/namespaces.md
        - Runtime libraries: reference/runtime-libraries.md
        - Extensions: reference/extensions-contract.md
        - Extension overview: reference/extensions.md
        - REPL UX: reference/repl.md
        - RESP protocol: reference/resp-protocol.md
        - Native flavors: reference/native-flavors.md
        - JVM flavor: reference/jvm-flavor.md
        - Xtalk equivalence: reference/xtalk-equivalence.md
        - Rust/WASM runtime: reference/rust-runtime.md
        - Clojure core compatibility: reference/clojure-core-compatibility.md
        - Runtime benchmarks: reference/runtime-benchmarks.md
        - Developer guide: development.md
        - Foundation porting: foundation-porting.md
        - Java API: javadocs.md
    - Apps:
        - hara-chrome: '!include ../apps/hara-chrome/mkdocs.yml'
        - hara-emacs: '!include ../apps/hara-emacs/mkdocs.yml'
    - Books:
        - The Little Book of HAL: '!include ../books/the-little-book-of-hal/mkdocs.yml'
  ```

- [ ] **Step 3: Create optional landing pages**

  Create minimal `website/docs/learn.md` and `website/docs/reference/index.md` if they do not already exist. For now they can be short overviews that link to their sub-pages.

- [ ] **Step 4: Build**

  ```bash
  /tmp/docs-venv/bin/mkdocs build --strict -f website/mkdocs.yml --site-dir /tmp/site-k
  ```

- [ ] **Step 5: Commit**

  ```bash
  git add website/mkdocs.yml website/docs/kernel.md \
          website/docs/learn.md website/docs/reference/index.md
  git commit -m "feat(website): restructure nav to Play/Learn/Kernel/Docs"
  ```

---

## Task 6: Update the homepage to a live-coding entry surface

**Files:**
- Modify: `website/overrides/home.html`
- Modify: `website/docs/index.md`
- Modify: `website/docs/stylesheets/hara.css`

**What:** A centered cinematic hero with a floating command prompt and example chips, followed by a gallery of runnable sound/visual/agent demos. The START button opens the full LOG/REPL panel.

- [ ] **Step 1: Replace the hero in `home.html`**

  Replace the `.hara-home-intro` block with:

  ```html
  <section class="hara-home-intro">
    <div class="hara-home-title">
      <div class="hara-home-kicker">A Modern Lisp for the Agentic Age.</div>
      <h1>HARA</h1>
    </div>

    <div class="hara-home-playground">
      <div class="hara-home-palette">
        <span class="hara-home-palette__prompt">hara ›</span>
        <input type="text" data-console-palette
               placeholder="(+ 19 23)" autocomplete="off"
               spellcheck="false" aria-label="Try Hara">
      </div>
      <div class="hara-home-chips">
        <button type="button" data-console-command="(+ 19 23)">math</button>
        <button type="button" data-console-command="(map inc [1 2 3])">lists</button>
        <button type="button" data-console-command="(osc 440)">sound</button>
        <button type="button" data-console-command="(agent :wander)">agent</button>
      </div>
    </div>

    <a class="hara-start" href="#hara-console" data-hara-start data-console-command="(+ 19 23)">START</a>
  </section>

  <section class="hara-home-demos" aria-label="Live demos">
    <h2 class="hara-home-demos__title">Make something now</h2>
    <div class="hara-home-demos__grid">
      <article class="hara-demo-card" data-demo="tone">
        <h3>Sound</h3>
        <p>Play a tone or sequence in the browser.</p>
        <button type="button" data-console-command="(osc 440)">Run</button>
      </article>
      <article class="hara-demo-card" data-demo="shape">
        <h3>Visuals</h3>
        <p>Draw shapes and patterns with Hara.</p>
        <button type="button" data-console-command="(rect 50 50 100 100)">Run</button>
      </article>
      <article class="hara-demo-card" data-demo="agent">
        <h3>Agents</h3>
        <p>Spawn a lightweight agent on the grid.</p>
        <button type="button" data-console-command="(agent :wander)">Run</button>
      </article>
    </div>
  </section>
  ```

- [ ] **Step 2: Simplify `website/docs/index.md`**

  Replace its current content with a short welcome and links:

  ```markdown
  ---

  Welcome to HARA — a modern Lisp that runs in your browser.

  - [Try the live environment](studio.md)
  - [Read the user guide](user-guide.md)
  - [Explore the Kernel](kernel.md)
  ```

- [ ] **Step 3: Add homepage playground styles**

  In `website/docs/stylesheets/hara.css`, add:

  ```css
  .hara-home-playground {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
    width: min(640px, 90vw);
  }

  .hara-home-palette {
    display: flex;
    align-items: center;
    gap: .6rem;
    width: 100%;
    padding: .9rem 1rem;
    background: rgba(2, 4, 8, .72);
    border: 1px solid var(--hara-line-strong);
    border-radius: 10px;
    backdrop-filter: blur(12px);
    font-family: var(--hara-mono);
    font-size: .95rem;
    color: #c6d4e3;
  }

  .hara-home-palette__prompt {
    color: var(--hara-cyan);
    font-weight: 700;
  }

  .hara-home-palette input {
    flex: 1;
    min-width: 0;
    background: transparent;
    border: 0;
    color: inherit;
    font: inherit;
    outline: none;
  }

  .hara-home-chips {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: .6rem;
  }

  .hara-home-chips button {
    padding: .35rem .7rem;
    background: rgba(65, 245, 228, .08);
    border: 1px solid rgba(65, 245, 228, .25);
    border-radius: 999px;
    color: var(--hara-cyan-bright);
    font-family: var(--hara-mono);
    font-size: .65rem;
    cursor: pointer;
    transition: background .15s, box-shadow .15s;
  }

  .hara-home-chips button:hover {
    background: rgba(65, 245, 228, .16);
    box-shadow: 0 0 12px rgba(65, 245, 228, .15);
  }

  .hara-home-demos {
    position: relative;
    z-index: 2;
    padding: 5rem 5vw;
    background: linear-gradient(180deg, rgba(2, 4, 8, 0) 0%, rgba(2, 4, 8, .92) 15%, rgba(2, 4, 8, .96) 100%);
  }

  .hara-home-demos__title {
    margin-bottom: 1.5rem;
    text-align: center;
    font-family: var(--hara-mono);
    font-size: .75rem;
    font-weight: 700;
    letter-spacing: .18em;
    text-transform: uppercase;
    color: #8b9db0;
  }

  .hara-home-demos__grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: 1.25rem;
    max-width: 960px;
    margin: 0 auto;
  }

  .hara-demo-card {
    padding: 1.5rem;
    background: rgba(3, 5, 10, .72);
    border: 1px solid var(--hara-line);
    border-radius: 12px;
    backdrop-filter: blur(12px);
    color: #c6d4e3;
  }

  .hara-demo-card h3 {
    margin: 0 0 .5rem;
    font-family: var(--hara-mono);
    font-size: .9rem;
    color: var(--hara-cyan-bright);
  }

  .hara-demo-card p {
    margin: 0 0 1rem;
    font-size: .85rem;
    color: #8b9db0;
    line-height: 1.5;
  }

  .hara-demo-card button {
    padding: .4rem 1rem;
    background: rgba(65, 245, 228, .1);
    border: 1px solid rgba(65, 245, 228, .3);
    border-radius: 6px;
    color: var(--hara-cyan-bright);
    font-family: var(--hara-mono);
    font-size: .65rem;
    letter-spacing: .08em;
    cursor: pointer;
    transition: background .15s, box-shadow .15s;
  }

  .hara-demo-card button:hover {
    background: rgba(65, 245, 228, .2);
    box-shadow: 0 0 14px rgba(65, 245, 228, .18);
  }
  ```

- [ ] **Step 4: Build and screenshot**

  Verify the homepage shows the new prompt and chips.

- [ ] **Step 5: Commit**

  ```bash
  git add website/overrides/home.html \
          website/docs/index.md \
          website/docs/stylesheets/hara.css
  git commit -m "feat(website): homepage live-coding entry surface"
  ```

---

## Task 7: Verify the build and screenshots

**Files:**
- All of the above.

- [ ] **Step 1: Full build**

  ```bash
  /tmp/docs-venv/bin/mkdocs build --strict -f website/mkdocs.yml --site-dir /tmp/site-k
  ```

  Expected: `Documentation built in` with no errors.

- [ ] **Step 2: Headless checks**

  Run a script that:
  1. Serves `/tmp/site-k` on a local port.
  2. Visits `/`, `/learn/`, `/kernel/`, `/reference/l0-language/`.
  3. Asserts the system bar is present and `window.__haraGame` is defined on each page.
  4. Scrolls each page by 700 px and confirms the grid is still visible in a screenshot.
  5. Types `(+ 19 23)` into the top-bar palette and confirms the LOG panel opens with `=> 42`.

- [ ] **Step 3: Commit any final fixes**

  If the check script required any changes, commit them with a clear message.

---

## Spec coverage check

| Spec requirement | Task(s) |
|---|---|
| Remove HARA//OS branding | Task 2 |
| Universal glass system bar | Tasks 2, 3 |
| Top-bar command palette | Tasks 2, 4 |
| Play / Learn / Kernel / Docs structure | Task 5 |
| Kernel page content | Task 5 |
| 9-dot launcher clipping fix | Task 3 |
| Game canvas visible while scrolling | Task 1 |
| Homepage live-coding surface | Task 6 |
| Tagline "A Modern Lisp for the Agentic Age." | Task 6 |

## Follow-up work (not in this plan)

- **Inline hara-emacs editor on demo cards:** each demo card expands into a small editor with Emacs keybindings and autocomplete. Edits evaluate directly onto the Tron grid canvas, turning the homepage into a live instrument.
- **Studio IDE:** `/studio` becomes the full multi-pane environment with file explorer, hara-emacs editor, and REPL.

## Placeholder scan

- No `TBD`, `TODO`, or "implement later" items in the immediate tasks.
- Each step contains concrete code or exact commands.
- File paths are exact.
