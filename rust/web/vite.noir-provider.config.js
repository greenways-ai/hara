import { defineConfig } from "vite";
import { resolve } from "node:path";

export default defineConfig({
  build: {
    target: "es2022",
    outDir: "dist-provider",
    emptyOutDir: true,
    lib: {
      entry: resolve(import.meta.dirname, "entries/noir-browser.mjs"),
      formats: ["es"],
      fileName: () => "worker.mjs"
    }
  }
});
