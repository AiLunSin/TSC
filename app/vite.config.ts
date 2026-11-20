import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { fileURLToPath, URL } from "node:url";

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  build: {
    target: "chrome105",
    sourcemap: false,
    chunkSizeWarningLimit: 600,
  },
  server: {
    strictPort: true,
    port: 5173,
  },
});