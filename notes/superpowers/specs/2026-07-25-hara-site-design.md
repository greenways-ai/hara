# HARA website design — a Lisp kernel for the agentic web

## Vision

HARA is a small, fast Lisp kernel that runs in the browser. It is both a live programming language and the runtime fabric underneath sessions, agents, and shared worlds. You land on the site and the kernel is already running in front of you — no install, no sign-up, no tutorial wall.

The site should feel like stepping into a Tron-inspired live-coding OS: dark, glowing, responsive, and immediately interactive. The first impression is the language; underneath it is a kernel for local sessions, state transitions, agent interactions, competitions, mediations, and collaborative work.

## Target audience

- Musicians and interactive artists who want to code audio, visuals, and instruments in the browser.
- Creative coders looking for a small, fast, embeddable Lisp.
- Curious developers who want to try a new language without friction.

## Site structure (top level)

Keep the public navigation extremely small. Advanced destinations live in the 9-dot launcher.

| Nav item | Purpose |
|---|---|
| **Play** (`/`) | Full-screen live Hara environment. This is the homepage. |
| **Learn** | Guided interactive tour, *The Little Book of HAL*, and short tutorials. |
| **Docs** | Reference: language spec, builtins, runtimes, RESP, extensions. |
| **≡ (9-dot)** | Launcher for Apps (hara-chrome, hara-emacs, hara-vscode), Books, Registry, theme, status, notifications. |

Removed from the top nav: Walkthroughs, Build, Reference sub-pages, Apps, Books. These become sections inside Learn, Docs, or the launcher.

## Universal top system bar

A single glass status bar appears on every page. It replaces the Material header and tabs everywhere.

```
[HARA]    [>_ type a hara form…]    [Learn] [Kernel] [Docs]    [● ready] [LOG 0] [≡]
```

### Left: brand + nav
- **HARA** wordmark (with the existing cyan apple-style logo mark). Click returns to Play.
- **Learn** — interactive tutorials and *The Little Book of HAL*.
- **Kernel** — sessions, state transitions, agent interactions, proofs, competitions, mediation, collaboration.
- **Docs** — language reference, runtimes, extensions, protocols.

### Center: live command palette
- A single input/button that reads `>_ try a hara form`.
- Clicking it focuses a small inline input in the bar.
- Typing a form and pressing Return evaluates it in the embedded Hara wasm runtime.
- Output streams to the LOG panel.
- Rotates micro-examples: `(+ 19 23)`, `(map inc [1 2 3])`, `(osc 440)`, etc.

### Right: status + tools
- **Status dot** — green when wasm ready, amber while booting, red on error.
- **LOG** — toggle for the evaluation/log panel.
- **≡ (9-dot)** — opens the app/control launcher from the right.

### Visual treatment
- Height: 48 px.
- Background: `rgba(3, 5, 10, .82)` with `backdrop-filter: blur(20px)`.
- Bottom border: 1 px subtle cyan line (`var(--hara-line)`).
- No internal vertical dividers.
- Text/icons in muted silver; hover/active state adds cyan glow + subtle wash.
- Monospace type used only for the command palette and status.

### 9-dot clipping fix
- The bar and its grid use `box-sizing: border-box`.
- The rightmost launcher button is a fixed 44 px square with at least 8 px safe margin from the viewport edge.
- On narrow screens the wordmark collapses to the logo only.

## Play (homepage)

The homepage is the studio. There is no separate marketing hero — the environment is the marketing.

- Full-viewport live coding workspace.
- The Tron grid game runs as a subtle animated background layer.
- A central REPL/editor area where visitors can type and evaluate Hara immediately.
- Example chips below the prompt: "Play a tone", "Draw a square", "Spawn a synth", etc.
- First-visit onboarding: one floating tooltip — *"Type `(+ 1 2)` and press Return."*

## Studio page

`/studio` is a deeper IDE layout:
- File explorer / session history on the left.
- Editor pane in the center.
- REPL/output panel on the right or bottom.
- Quick access to wasm extension demos (synth, csound, graphics).

## Learn

- Short interactive tour: "5 minutes of Hara" with runnable examples.
- *The Little Book of HAL* as an embedded, chapter-driven book with live code cells.
- Tutorials organized by creative domain: sound, visuals, instruments, algorithms.

## Kernel

The Kernel section explains HARA as a runtime fabric, not just a language:

- **Sessions** — local browser sessions backed by the Hara wasm runtime.
- **State transitions** — logged, verifiable transitions between session states.
- **Agents** — small autonomous programs that interact through Hara.
- **Proofs** — session validity without requiring the full record to be shared.
- **Competition / mediation / collaboration** — patterns for multi-party sessions.

## Docs

- Reference: language spec (L0), builtins, namespaces, runtime libraries.
- Native flavors: JVM, Rust/WASM, native image.
- Extension contract and RESP protocol.
- Runtime benchmarks.

## 9-dot launcher contents

| Section | Items |
|---|---|
| **Apps** | hara-chrome, hara-emacs, hara-vscode, hara-lsp |
| **Books** | *The Little Book of HAL* |
| **Registry** | Browse wasm extensions (future) |
| **System** | Theme, runtime status, notifications |

## Responsive behavior

- Desktop: full bar with command palette expanded.
- Tablet: command palette collapses to an icon; tapping opens a centered palette overlay.
- Mobile: only logo, Learn, Docs, status dot, and ≡. The command palette moves to a bottom sheet.

## Why this design

- **Zero setup:** the call-to-action is a REPL prompt, not a download button.
- **Creative-first:** examples and onboarding lead with sound and visuals, not systems programming.
- **One identity:** the same bar and runtime exist on every page; the site feels like a single OS.
- **Cool factor:** the live grid, glowing borders, and instant evaluation make the site feel alive.

## Open questions

1. Should `/` and `/studio` be the same page, or should `/` be a lighter "try it now" surface and `/studio` the full IDE?
2. Which creative example should appear first in the command palette rotation?
3. Should the 9-dot launcher also expose user settings (e.g., keybindings, theme)?
