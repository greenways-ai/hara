import { defineConfig } from "vite";
import { resolve } from "node:path";

export default defineConfig({
  optimizeDeps: {
    exclude: ["@sqlite.org/sqlite-wasm"]
  },
  build: {
    target: "node18",
    outDir: "dist-sqlite-node",
    emptyOutDir: true,
    assetsInlineLimit: 0,
    lib: {
      entry: resolve(import.meta.dirname, "entries/sqlite-node.mjs"),
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
