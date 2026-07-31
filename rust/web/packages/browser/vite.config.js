import { defineConfig } from "vite";

export default defineConfig({
  build: {
    emptyOutDir: true,
    outDir: "packages/browser/dist",
    assetsInlineLimit: 10_000_000,
    lib: {
      entry: "packages/browser/src/index.js",
      name: "Hara",
      formats: ["es", "iife"],
      fileName: (format) => (format === "es" ? "hara.mjs" : "hara.js")
    },
    rollupOptions: {
      output: {
        inlineDynamicImports: true,
        assetFileNames: "hara.wasm"
      }
    }
  }
});
