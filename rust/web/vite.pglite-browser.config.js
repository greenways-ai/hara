import { defineConfig } from "vite";
import { resolve } from "node:path";

const pglite = resolve(
  import.meta.dirname,
  "../extensions/std-db-pglite/node_modules/@electric-sql/pglite/dist/index.js"
);

export default defineConfig({
  resolve: {
    alias: {
      "@electric-sql/pglite": pglite
    }
  },
  build: {
    target: "es2022",
    outDir: "dist-pglite-browser",
    emptyOutDir: true,
    assetsInlineLimit: 0,
    lib: {
      entry: resolve(import.meta.dirname, "entries/pglite-browser.mjs"),
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
