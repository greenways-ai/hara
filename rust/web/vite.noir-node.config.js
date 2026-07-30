import { defineConfig } from "vite";
import { resolve } from "node:path";

const noirSource = process.env.HARA_NOIR_SOURCE
  ? resolve(process.env.HARA_NOIR_SOURCE)
  : resolve(import.meta.dirname, "../extensions/ledger-noir");

export default defineConfig({
  resolve: {
    alias: {
      "../assets/hta.js": resolve(import.meta.dirname, "hta.js"),
      "fake-indexeddb/auto": resolve(import.meta.dirname, "node_modules/fake-indexeddb/auto/index.mjs")
    }
  },
  build: {
    target: "node18",
    outDir: "dist-node",
    emptyOutDir: true,
    lib: {
      entry: resolve(noirSource, "node/worker.mjs"),
      formats: ["es"],
      fileName: () => "worker.mjs"
    },
    rollupOptions: {
      output: { inlineDynamicImports: true }
    }
  }
});
