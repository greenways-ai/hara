import { defineConfig } from "astro/config";
import sitemap from "@astrojs/sitemap";
import starlight from "@astrojs/starlight";
import remarkHaraEval from "./scripts/remark-hara-eval.mjs";

export default defineConfig({
  site: "https://www.hara-lang.org",
  output: "static",
  outDir: "../target/www-astro",
  markdown: { remarkPlugins: [remarkHaraEval] },
  integrations: [
    sitemap(),
    starlight({
      title: "Hara",
      logo: {
        light: "./src/assets/hara-mark-light.svg",
        dark: "./src/assets/hara-mark-dark.svg",
        alt: "Hara"
      },
      description: "A small, high-performance Lisp for learning to build software from first principles.",
      favicon: "/assets/hara-favicon.svg",
      head: [
        { tag: "link", attrs: { rel: "stylesheet", href: "/docs-assets/stylesheets/syllabus.css" } },
        { tag: "script", attrs: { type: "module", src: "/assets/docs-repl.js" } },
        { tag: "script", attrs: { type: "module", src: "/docs-assets/javascripts/syllabus.js" } },
        { tag: "meta", attrs: { property: "og:site_name", content: "Hara / Docs" } },
        { tag: "meta", attrs: { property: "og:image", content: "https://www.hara-lang.org/og-hara-docs.jpg" } },
        { tag: "meta", attrs: { property: "og:image:secure_url", content: "https://www.hara-lang.org/og-hara-docs.jpg" } },
        { tag: "meta", attrs: { property: "og:image:type", content: "image/jpeg" } },
        { tag: "meta", attrs: { property: "og:image:width", content: "3840" } },
        { tag: "meta", attrs: { property: "og:image:height", content: "2016" } },
        { tag: "meta", attrs: { property: "og:image:alt", content: "Hara Docs — build live, across every host" } },
        { tag: "meta", attrs: { name: "twitter:card", content: "summary_large_image" } },
        { tag: "meta", attrs: { name: "twitter:image", content: "https://www.hara-lang.org/og-hara-docs.jpg" } },
        { tag: "meta", attrs: { name: "twitter:image:alt", content: "Hara Docs — build live, across every host" } }
      ],
      customCss: ["./src/styles/docs.css"],
      social: [{ icon: "github", label: "GitHub", href: "https://github.com/hara-lang/hara" }],
      sidebar: [
        {
          label: "Start here",
          items: [
            { label: "01 — Why Hara?", slug: "docs/start/orientation" },
            { label: "02 — Read Hara and build from scratch", slug: "docs/learn-programming" },
            { label: "03 — Try Hara in the browser", slug: "docs/getting-started/playground" },
            { label: "04 — Build Tic Tac Toe", slug: "docs/create/first-game" },
            { label: "05 — Choose your setup", slug: "docs/getting-started" }
          ]
        },
        {
          label: "Interactive courses",
          items: [
            { label: "Choose a learning path", slug: "docs/learn" },
            { label: "First Contact", slug: "docs/learn/first-contact" },
            { label: "Protocols for Builders", slug: "docs/learn/protocols" },
            { label: "Collection Protocols", slug: "docs/learn/protocols/collections" },
            { label: "State and Lifecycle Protocols", slug: "docs/learn/protocols/state-lifecycle" },
            { label: "Protocol Atlas", slug: "docs/learn/protocols/atlas" }
          ]
        },
        {
          label: "Build Tic Tac Toe",
          collapsed: true,
          items: [
            { label: "Part I — Draw the board", slug: "docs/create/tictactoe/board" },
            { label: "Part II — Organise the game", slug: "docs/create/tictactoe/files" },
            { label: "Part III — State and rules", slug: "docs/create/tictactoe/state" },
            { label: "Part IV — Pointer input", slug: "docs/create/tictactoe/input" },
            { label: "Part V — Rendering", slug: "docs/create/tictactoe/rendering" },
            { label: "Part VI — Make it live", slug: "docs/create/tictactoe/live" }
          ]
        },
        {
          label: "Hara language course",
          items: [
            { label: "Course map", slug: "docs/hal-intro" },
            { label: "01 — Basic data and persistent collections", slug: "docs/hal-intro/01-basic-data" },
            { label: "02 — Functions and atoms", slug: "docs/hal-intro/02-functions-and-atoms" },
            { label: "03 — Sequences, iterators, and streaming", slug: "docs/hal-intro/03-iterators-and-streaming" },
            { label: "04 — Coroutines and promises", slug: "docs/hal-intro/04-coroutines-and-promises" },
            { label: "05 — Arrays and objects", slug: "docs/hal-intro/05-array-and-object" },
            { label: "06 — Bytes and strings", slug: "docs/hal-intro/06-bytes-and-strings" },
            { label: "07 — I/O and files", slug: "docs/hal-intro/07-io-and-files" }
          ]
        },
        {
          label: "Work with Hara",
          collapsed: true,
          items: [
            { label: "CLI", slug: "docs/getting-started/cli" },
            { label: "Web", slug: "docs/getting-started/web" },
            { label: "JVM", slug: "docs/getting-started/jvm" },
            { label: "VS Code", slug: "docs/getting-started/vscode" },
            { label: "Emacs", slug: "docs/getting-started/emacs" },
            { label: "Runtime-driven development", slug: "docs/concepts/runtime-driven-development" },
            { label: "Live coding", slug: "docs/concepts/live-coding" },
            { label: "Unified representation", slug: "docs/concepts/unified-representation" }
          ]
        },
        {
          label: "Guides & reference",
          autogenerate: { directory: "docs/reference" }
        }
      ]
    })
  ]
});
