import { defineConfig } from "astro/config";
import sitemap from "@astrojs/sitemap";
const base = process.env.HARA_BENCHMARK_BASE || "/";
const site = base === "/" ? "https://benchmarks.hara-lang.org" : "https://hara-lang.org";
const outDir = process.env.HARA_BENCHMARK_OUT_DIR || "../target/benchmarks";
export default defineConfig({ site, base, output: "static", outDir, integrations: [sitemap()] });
