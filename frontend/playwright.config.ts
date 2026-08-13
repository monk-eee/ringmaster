import { defineConfig } from "@playwright/test";

// ADR-0012: assumes `podman compose up -d` already has Postgres + the Rust
// backend running and reachable at BACKEND_URL; this only launches the
// front end's own Express server for the test run.
export default defineConfig({
  testDir: "./tests",
  fullyParallel: true,
  webServer: {
    command: "node server.mjs",
    url: "http://localhost:3000",
    reuseExistingServer: !process.env.CI,
    env: {
      PORT: "3000",
      BACKEND_URL: process.env.BACKEND_URL || "http://localhost:8080",
    },
  },
  use: {
    baseURL: "http://localhost:3000",
  },
  projects: [{ name: "chromium", use: { browserName: "chromium" } }],
});
