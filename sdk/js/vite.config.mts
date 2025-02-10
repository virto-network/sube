import { defineConfig } from "vite";
import { resolve } from "path";
import wasm from 'vite-plugin-wasm';

export default defineConfig({
  server: {
    port: 3000,
  },
  plugins: [wasm()],
  build: {
    outDir: "dist/umd",
    lib: {
      entry: resolve(__dirname, "src/auth.ts"),
      name: "Auth",
      fileName: "auth",
      formats: ["umd"]
    },
  }
});
