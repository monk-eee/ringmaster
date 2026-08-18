import { defineConfig } from "@playwright/test";

// ADR-0073: Playwright never talks to the development backend/frontend
// (8080/3001) -- it starts its own dedicated pair against the isolated
// ringmaster_test database, so a browser test run can never write fixture
// data into the database the running app and real ingestion read from.
// Neither process is reused across runs (reuseExistingServer: false): if
// something is already on 18080/13001, that is a stale process from a prior
// run, not a server safe to attach to, and startup should fail loudly.
const testDatabaseUrl =
  process.env.PLAYWRIGHT_DATABASE_URL ||
  "postgres://ringmaster:ringmaster-dev@localhost:5432/ringmaster_test";

export default defineConfig({
  testDir: "./tests",
  fullyParallel: true,
  webServer: [
    {
      command:
        "cargo run --manifest-path ../backend/Cargo.toml --bin ringmaster-backend",
      url: "http://127.0.0.1:18080/health",
      reuseExistingServer: false,
      timeout: 180_000,
      name: "Backend",
      stdout: "pipe",
      stderr: "pipe",
      env: {
        DATABASE_URL: testDatabaseUrl,
        RINGMASTER_BIND_ADDR: "127.0.0.1:18080",
        RINGMASTER_REQUIRE_TEST_DATABASE: "true",
        CARGO_TARGET_DIR: "../target/playwright-backend",
      },
    },
    {
      command: "npx vite",
      url: "http://127.0.0.1:13001",
      reuseExistingServer: false,
      timeout: 120_000,
      name: "Frontend",
      stdout: "pipe",
      stderr: "pipe",
      env: {
        BACKEND_URL: "http://127.0.0.1:18080",
        VITE_PORT: "13001",
      },
    },
  ],
  use: {
    baseURL: "http://127.0.0.1:13001",
  },
  // ADR-0087: fullyParallel drives every worker against one shared
  // backend/Vite pair (ADR-0073), so legitimate concurrent load can push a
  // real round trip past the implicit 5000ms default under full-suite
  // concurrency even though any single flow is well under it alone.
  expect: {
    timeout: 10_000,
  },
  projects: [{ name: "chromium", use: { browserName: "chromium" } }],
});
