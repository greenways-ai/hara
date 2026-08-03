import { defineConfig } from "astro/config";
import sitemap from "@astrojs/sitemap";
export default defineConfig({ site: "https://benchmarks.hara-lang.org", output: "static", outDir: "../target/benchmarks", integrations: [sitemap()] });
