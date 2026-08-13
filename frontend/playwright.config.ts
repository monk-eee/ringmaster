import { defineConfig } from "@playwright/test";

// ADR-0014: assumes `podman compose up -d` already has Postgres + the Rust
// backend running and reachable at BACKEND_URL; this only launches Vite's
// own dev server for the test run.
export default defineConfig({
  testDir: "./tests",
  fullyParallel: true,
  webServer: {
    command: "npx vite",
    url: "http://localhost:3000",
    reuseExistingServer: !process.env.CI,
    env: {
      BACKEND_URL: process.env.BACKEND_URL || "http://localhost:8080",
    },
  },
  use: {
    baseURL: "http://localhost:3000",
  },
  projects: [{ name: "chromium", use: { browserName: "chromium" } }],
});
