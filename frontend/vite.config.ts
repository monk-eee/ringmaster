import { execFileSync } from "node:child_process";
import { defineConfig, type Plugin } from "vite";
import react from "@vitejs/plugin-react";

// ADR-0014: dev server only (this stack is local dev tooling per ADR-0006).
// BACKEND_URL is read here, server-side, and never exposed to client code.
const backendUrl = process.env.BACKEND_URL || "http://localhost:8080";
// ADR-0073: Playwright's isolated Vite instance overrides this to 13001 so
// it never competes with the developer's own dev server pinned to 3001
// (ADR-0067). Absent, behavior is unchanged.
const port = Number(process.env.VITE_PORT) || 3001;

// ADR-0078: build provenance, so a stale container is visible in
// `podman compose logs` instead of requiring a manual comparison. Missing
// git (e.g. a build context with no .git) degrades to "unknown" rather
// than failing the dev server.
function gitInfo(args: string[]): string {
  try {
    return execFileSync("git", args, { encoding: "utf8" }).trim() || "unknown";
  } catch {
    return "unknown";
  }
}
const gitSha = gitInfo(["rev-parse", "--short=12", "HEAD"]);
const gitCommitTime = gitInfo(["log", "-1", "--format=%cI"]);

function logBuildProvenance(): Plugin {
  return {
    name: "ringmaster-log-build-provenance",
    configureServer() {
      console.log(`ringmaster-frontend: built from ${gitSha} (${gitCommitTime})`);
    },
  };
}

export default defineConfig({
  plugins: [react(), logBuildProvenance()],
  server: {
    host: true,
    port,
    strictPort: true,
    proxy: {
      "/api": { target: backendUrl, changeOrigin: true },
    },
  },
});
