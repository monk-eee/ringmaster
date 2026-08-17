import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// ADR-0014: dev server only (this stack is local dev tooling per ADR-0006).
// BACKEND_URL is read here, server-side, and never exposed to client code.
const backendUrl = process.env.BACKEND_URL || "http://localhost:8080";

export default defineConfig({
  plugins: [react()],
  server: {
    host: true,
    port: 3001,
    strictPort: true,
    proxy: {
      "/api": { target: backendUrl, changeOrigin: true },
    },
  },
});
