import { defineConfig } from "vite";
import { resolve } from "node:path";

export default defineConfig({
  base: "./",
  build: {
    target: "es2022",
    outDir: "dist",
    emptyOutDir: true,
    rollupOptions: {
      external: ["./noir-wasm.mjs"],
      output: {
        entryFileNames: "[name].js",
        chunkFileNames: "[name].js",
        assetFileNames: "assets/[name][extname]"
      }
    },
    lib: {
      entry: resolve(import.meta.dirname, "noir-loader.js"),
      formats: ["es"],
      fileName: "noir-loader"
    }
  }
});
