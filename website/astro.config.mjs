import { defineConfig } from "astro/config";
import sitemap from "@astrojs/sitemap";
import starlight from "@astrojs/starlight";

export default defineConfig({
  site: "https://hara-lang.org",
  output: "static",
  outDir: "../target/www-astro",
  integrations: [
    sitemap(),
    starlight({
      title: "Hara",
      description: "A programmable kernel for building, inspecting, and changing live systems.",
      favicon: "/assets/hara-favicon.svg",
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
