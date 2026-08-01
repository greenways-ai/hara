import { defineConfig } from "vite";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const web = dirname(fileURLToPath(import.meta.url));
const pglite = resolve(
  web,
  "../extensions/std-db-pglite/node_modules/@electric-sql/pglite/dist/index.js"
);

export default defineConfig({
  resolve: {
    alias: {
      "@electric-sql/pglite": pglite
    }
  },
  build: {
    target: "node18",
    outDir: "dist-pglite-node",
    emptyOutDir: true,
    assetsInlineLimit: 0,
    lib: {
      entry: resolve(web, "entries/pglite-node.mjs"),
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
