import { defineConfig } from "vite";
import { resolve } from "node:path";

export default defineConfig({
  build: {
    target: "node18",
    outDir: "dist-node",
    emptyOutDir: true,
    lib: {
      entry: resolve(import.meta.dirname, "entries/noir-node.mjs"),
      formats: ["es"],
      fileName: () => "worker.mjs"
    }
  }
});
