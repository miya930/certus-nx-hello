import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [react()],
  build: {
    outDir: "../build/ui",
    emptyOutDir: true,
    // ローカルのサーバーから開くだけなので、KaTeX を含めて 1 つのファイルにまとめたままにする。
    chunkSizeWarningLimit: 800,
  },
  server: {
    // 開発中は、シミュレーションの API をサーバーに転送する。
    proxy: { "/api": "http://127.0.0.1:8765" },
  },
});
