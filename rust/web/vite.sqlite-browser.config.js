import { defineConfig } from "vite";
import { resolve } from "node:path";

export default defineConfig({
  optimizeDeps: {
    exclude: ["@sqlite.org/sqlite-wasm"]
  },
  build: {
    target: "es2022",
    outDir: "dist-sqlite-browser",
    emptyOutDir: true,
    assetsInlineLimit: 0,
    lib: {
      entry: resolve(import.meta.dirname, "entries/sqlite-browser.mjs"),
      formats: ["es"],
      fileName: () => "worker.mjs"
    },
    rollupOptions: {
      output: {
        assetFileNames: "assets/[name][extname]"
      }
    }
  }
});
