import { defineConfig } from "astro/config";
import sitemap from "@astrojs/sitemap";
const base = process.env.HARA_BENCHMARK_BASE || "/benchmarks";
const site = "https://www.hara-lang.org";
const outDir = process.env.HARA_BENCHMARK_OUT_DIR || "../core/target/benchmarks";
export default defineConfig({ site, base, output: "static", outDir, integrations: [sitemap()] });
