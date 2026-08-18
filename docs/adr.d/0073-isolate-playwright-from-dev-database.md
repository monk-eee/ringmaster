# ADR-0073: Isolate Playwright from the development database

- **Status:** Accepted
- **Date:** 2026-08-18
- **Decider:** monk-eee
- **Approval:** Direct instruction ("accept and continue"), 2026-08-18
- **Amends:** [ADR-0056](0056-local-test-database-isolation-and-dev-data-cleanup.md) and [ADR-0057](0057-enforce-test-database-isolation-with-a-runtime-guard.md) by extending their prevention invariant from backend unit tests to browser tests.
- **Depends on:** [ADR-0012](0012-minimal-http-api-and-node-web-front-end.md), [ADR-0014](0014-react-vite-single-page-app.md), [ADR-0056](0056-local-test-database-isolation-and-dev-data-cleanup.md), [ADR-0057](0057-enforce-test-database-isolation-with-a-runtime-guard.md)
- **Tags:** testing, playwright, data-hygiene, infrastructure

## Context

ADR-0056 created `ringmaster_test`, and ADR-0057 makes every backend
`test_pool()` panic unless `DATABASE_URL` names that database. The running
application is cleanly separated from `cargo test` as a result.

Playwright is outside that guard. `frontend/playwright.config.ts` currently
starts or reuses Vite on port 3001 and proxies to `BACKEND_URL`, defaulting to
the long-lived application backend on port 8080. Every browser fixture created
through `POST /api/nodes`, ingestion, candidate actions, and graph writes
therefore lands in the same `ringmaster` database a person reads in the app.

This is no longer theoretical. A read-only audit on 2026-08-18 found 392
Person nodes in the development database, including 357 named
`Pagination Test Person ...`, 8 `Needs Attention Filter Bare ...`, 5
`Recent Interaction Person ...`, and 4
`Capped Recent Interactions Person ...`: at least 374 known browser-test
fixtures, or more than 95% of all Person rows. The browser suite has recreated
the data-pollution failure ADR-0056/0057 fixed for backend tests.

## Decision

- The backend gains an optional `RINGMASTER_BIND_ADDR`; absent it keeps the
  exact current `0.0.0.0:8080` behavior. Playwright sets it to
  `127.0.0.1:18080` so its backend can run alongside the development backend.
- The backend also supports `RINGMASTER_REQUIRE_TEST_DATABASE=true`. When set,
  startup refuses any `DATABASE_URL` whose database name is not exactly
  `ringmaster_test`, before opening a connection or running migrations. This
  is the browser-test equivalent of ADR-0057's unit-test guard; a typo cannot
  silently point Playwright back at `ringmaster`.
- `frontend/playwright.config.ts` manages two web servers, neither reusable:
  1. `ringmaster-backend` on `127.0.0.1:18080`, with
     `DATABASE_URL` set from `PLAYWRIGHT_DATABASE_URL` or the local
     `ringmaster_test` default, `RINGMASTER_REQUIRE_TEST_DATABASE=true`, and an
     isolated Cargo target directory; and
  2. Vite on `127.0.0.1:13001`, proxying `/api` to the Playwright backend.
- Playwright's `baseURL` becomes `http://127.0.0.1:13001`. The normal app stays
  on port 3001 and its backend stays on 8080; browser tests never reuse either
  process.
- The test backend applies migrations at startup exactly as the normal backend
  already does. `ringmaster_test` remains the one database name for every test
  surface.
- `scripts/dev-data-report.sql` gains a separate, disclosed read-only section
  counting known Playwright fixture prefixes alongside non-matching Person
  rows. This makes existing pollution reviewable but performs no deletion.
- No cleanup runs as part of this ADR. Deleting the existing fixture rows still
  requires the explicit report/backup/no-active-user confirmation required by
  ADR-0056, plus a cleanup script updated to cover browser fixtures safely.

## Scope

**In scope:** configurable backend bind address; opt-in startup enforcement of
the `ringmaster_test` database; dedicated Playwright backend/frontend ports and
processes; a read-only Playwright-fixture count in the dev-data report;
documentation and focused verification that the development database count
does not change during a browser test.

**Out of scope, named honestly:**

- **Deleting existing browser fixtures.** The current database is shared and
  active; cleanup remains a separately confirmed operational action.
- **Per-test rollback or database reset.** The suite may accumulate inside
  `ringmaster_test`; isolation, not pristine-state enforcement, is the problem
  this ADR solves. Tests must continue owning unique fixtures.
- **Running Playwright in GitHub Actions.** The frontend CI job currently
  typechecks/builds only; adding browser CI and a browser matrix is a separate
  pipeline decision.
- **Changing production/development ports or database configuration.** Default
  backend and Vite behavior remains unchanged outside Playwright.
- **General multi-tenant database authorization.** This is a local test safety
  boundary, not ADR-0004's deferred access-control system.

## Options considered

- **Dedicated test backend and Vite ports over `ringmaster_test` (chosen):**
  mirrors the real application boundary, exercises HTTP end-to-end, cannot
  collide with the running app, and reuses the already accepted test database.
- **Keep using ports 3001/8080 but point the running backend at
  `ringmaster_test`:** rejected because it temporarily takes the actual app
  offline or makes it display test data, and concurrent sessions can race the
  database switch.
- **Delete fixtures after every browser test:** rejected because immutable
  source fragments and event logs make reliable cleanup non-trivial, crashes
  leave residue, and cleanup does not prevent one mistaken run against real
  data.
- **Mock every API response in Playwright:** rejected because these tests are
  explicitly end-to-end and must continue proving real backend behavior.
- **Rely on `BACKEND_URL` discipline:** rejected for the same reason ADR-0057
  rejected a documented backend-test convention: this workspace has now
  demonstrated that advisory configuration repollutes the real app.

## Consequences

- **Positive:** browser tests stop changing the data visible at port 3001.
- **Positive:** an incorrect Playwright database URL fails before any write,
  structurally extending ADR-0057's protection to E2E tests.
- **Positive:** tests no longer depend on or compete with whichever frontend
  and backend containers happen to be running.
- **Negative / trade-off:** the first browser run compiles/starts a separate
  backend and is slower than reusing the development stack.
- **Negative / trade-off:** `ringmaster_test` still needs to exist locally,
  as already required for backend tests by ADR-0056.
- **Risk:** low to moderate. The normal app's defaults are unchanged, but
  cross-platform Playwright process startup and shutdown require focused
  Windows and CI-compatible command verification.

## Exit criteria and evidence

Evidence: [EV-0073](../evidence.d/0073-isolate-playwright-from-dev-database.md)

| Exit criterion | Evidence |
|---|---|
| Playwright starts an isolated backend and Vite server on dedicated ports, never reusing 3001/8080 | `playwright-uses-dedicated-processes-and-ports` |
| Test-mode backend startup refuses every database except `ringmaster_test` | `playwright-backend-enforces-test-database` |
| A focused browser test passes while the development database fixture count remains unchanged | `browser-test-does-not-write-dev-database` |
| Normal backend/Vite defaults remain 8080/3001 | `normal-app-ports-remain-unchanged` |
| The report discloses known Playwright fixture counts without deleting data | `playwright-pollution-report-is-read-only` |