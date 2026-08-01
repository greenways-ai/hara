import { defineConfig } from "vite";
import { resolve } from "node:path";

const sqlite = resolve(
  import.meta.dirname,
  "../extensions/std-db-sqlite/node_modules/@sqlite.org/sqlite-wasm/node.mjs"
);

export default defineConfig({
  resolve: {
    alias: {
      "@sqlite.org/sqlite-wasm": sqlite
    }
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
