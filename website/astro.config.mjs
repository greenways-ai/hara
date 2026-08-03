import { defineConfig } from "astro/config";
import sitemap from "@astrojs/sitemap";
import starlight from "@astrojs/starlight";
import remarkHaraEval from "./scripts/remark-hara-eval.mjs";

export default defineConfig({
  site: "https://hara-lang.org",
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
      description: "A programmable kernel for building, inspecting, and changing live systems.",
      favicon: "/assets/hara-favicon.svg",
      head: [{ tag: "script", attrs: { type: "module", src: "/assets/docs-repl.js" } }],
      customCss: ["./src/styles/docs.css"],
      social: [{ icon: "github", label: "GitHub", href: "https://github.com/hara-lang/hara" }],
      sidebar: [
        { label: "Start", autogenerate: { directory: "docs/start" } },
        { label: "Learn Hara", autogenerate: { directory: "docs/hal-intro" } },
        { label: "Guides & reference", autogenerate: { directory: "docs/reference" } }
      ]
    })
  ]
});
