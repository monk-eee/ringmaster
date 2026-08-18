import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// ADR-0014: dev server only (this stack is local dev tooling per ADR-0006).
// BACKEND_URL is read here, server-side, and never exposed to client code.
const backendUrl = process.env.BACKEND_URL || "http://localhost:8080";
// ADR-0073: Playwright's isolated Vite instance overrides this to 13001 so
// it never competes with the developer's own dev server pinned to 3001
// (ADR-0067). Absent, behavior is unchanged.
const port = Number(process.env.VITE_PORT) || 3001;

export default defineConfig({
  plugins: [react()],
  server: {
    host: true,
    port,
    strictPort: true,
    proxy: {
      "/api": { target: backendUrl, changeOrigin: true },
    },
  },
});
