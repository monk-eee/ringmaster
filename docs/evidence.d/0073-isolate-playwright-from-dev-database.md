# EV-0073: Isolate Playwright from the development database

Evidence for [ADR-0073](../adr.d/0073-isolate-playwright-from-dev-database.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0073-isolate-playwright-from-dev-database"

[[check]]
id = "playwright-uses-dedicated-processes-and-ports"
invariant = "Playwright manages a backend on 18080 and Vite on 13001 without reusing the development servers on 8080/3001."
type = "present"
pattern = '127\.0\.0\.1:18080'
paths = ["frontend/playwright.config.ts"]

[[check]]
id = "playwright-backend-enforces-test-database"
invariant = "Playwright backend startup refuses a DATABASE_URL whose database name is not ringmaster_test before any connection or migration."
type = "present"
pattern = "RINGMASTER_REQUIRE_TEST_DATABASE"
paths = ["backend/src/lib.rs"]

[[check]]
id = "browser-test-does-not-write-dev-database"
invariant = "A focused Playwright test passes without changing the development database fixture count."
type = "manual"
last_verified = "2026-08-18"
rationale = "Measured the real ringmaster database's Person-node count directly before and after a Playwright run against the new isolated config: 392 before, 392 after (unchanged). Ran `npx playwright test --grep \"People tab\"` (4 tests, all in frontend/tests/obligations.spec.ts, including the real-ingestion and Load-more fixture-creating tests): 4 passed, 0 failed, in 55.8s. The isolated backend's own startup log confirmed it connected to ringmaster_test, not ringmaster (`projection rebuilt (51 obligation(s))`, far below the real database's obligation count), and printed `listening on 127.0.0.1:18080`. ringmaster_test's own Person count rose from 96 to 204 across this run, showing the fixtures landed in the isolated database instead. The normal ringmaster-backend-1 (8080) and ringmaster-frontend-1 (3001) containers remained Up and untouched throughout. After the run, ports 18080/13001 returned to TIME_WAIT (process exited), confirming Playwright's teardown released both dedicated servers cleanly."

[[check]]
id = "normal-app-ports-remain-unchanged"
invariant = "Without Playwright test configuration, the backend listens on 8080 and Vite on 3001 as before."
type = "present"
pattern = '"0\.0\.0\.0:8080"'
paths = ["backend/src/lib.rs"]

[[check]]
id = "playwright-pollution-report-is-read-only"
invariant = "The dev-data report counts known Playwright fixture prefixes and contains no write statement."
type = "absent"
pattern = '(?<!--[^\n]*)\b(INSERT INTO|UPDATE |DELETE FROM|DROP )\b'
paths = ["scripts/dev-data-report.sql"]
```

## Notes

Implemented: `frontend/playwright.config.ts` runs a dedicated
`ringmaster-backend` (bound to `127.0.0.1:18080`, `RINGMASTER_REQUIRE_TEST_DATABASE=true`,
its own `CARGO_TARGET_DIR`) and a dedicated Vite instance
(`127.0.0.1:13001`) pointed at it, both with `reuseExistingServer: false`.
`backend/src/lib.rs` adds `RINGMASTER_BIND_ADDR` (defaulting to the
unchanged `0.0.0.0:8080`) and `enforce_test_database_if_required`, which
`main.rs` calls before connecting or migrating; it is a no-op unless
`RINGMASTER_REQUIRE_TEST_DATABASE=true`, and then refuses to start against
any database other than `ringmaster_test`. `scripts/dev-data-report.sql`
gained a read-only section counting the four known Playwright fixture-name
prefixes (`Pagination Test Person`, `Needs Attention Filter Bare`,
`Recent Interaction Person`, `Capped Recent Interactions Person`) against
the real database, disclosed alongside a non-matching count. Verified with
`cargo check`, the full backend test suite (152 lib tests + 9 integration
tests, all passing), and a live isolated Playwright run (see the manual
check above for exact measurements).