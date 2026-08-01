import { defineConfig } from "vite";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const web = dirname(fileURLToPath(import.meta.url));
const sqlite = resolve(
  web,
  "../extensions/std-db-sqlite/node_modules/@sqlite.org/sqlite-wasm/index.mjs"
);

export default defineConfig({
  resolve: {
    alias: {
      "@sqlite.org/sqlite-wasm": sqlite
    }
  },
  build: {
    target: "es2022",
    outDir: "dist-sqlite-browser",
    emptyOutDir: true,
    assetsInlineLimit: 0,
    lib: {
      entry: resolve(web, "entries/sqlite-browser.mjs"),
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
