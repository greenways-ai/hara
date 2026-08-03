import { defineConfig } from "astro/config";
import sitemap from "@astrojs/sitemap";
export default defineConfig({ site: "https://hara-lang.org", base: "/benchmarks", output: "static", outDir: "../target/benchmarks", integrations: [sitemap()] });
