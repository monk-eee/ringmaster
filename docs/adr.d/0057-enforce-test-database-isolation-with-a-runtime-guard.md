# ADR-0057: Enforce test-database isolation with a runtime guard

- **Status:** Accepted
- **Date:** 2026-08-17
- **Decider:** monk-eee
- **Amends:** [ADR-0056](0056-local-test-database-isolation-and-dev-data-cleanup.md)
- **Depends on:** [ADR-0017](0017-add-github-actions-ci-pipeline.md), [ADR-0056](0056-local-test-database-isolation-and-dev-data-cleanup.md)
- **Tags:** architecture, testing, data-hygiene, infrastructure

## Context

[ADR-0056](0056-local-test-database-isolation-and-dev-data-cleanup.md)
created an isolated `ringmaster_test` database and made pointing
`DATABASE_URL` at it the fix, explicitly deciding **"No Rust code
changes"** and naming the documented test-run convention as *"the
enforcement mechanism this repo already relies on for cross-session
conventions."* That posture was correct for stopping *future* pollution
cheaply — but a documented convention is advisory, and this environment
has now demonstrated, twice, that advisory is not enough:

- The dev `ringmaster` database was reset to a clean slate (0 rows) and,
  within the same working session, repolluted to 47 obligations / 53
  nodes / 24 candidates by a concurrent `cargo test` run that still
  carried `DATABASE_URL=...@postgres:5432/ringmaster` — the old habit,
  not the new convention.
- A real end-to-end demonstration (ingest a meeting → extract → promote →
  one clean item on Today) was buried again by the same mechanism.

The root cause is unchanged from ADR-0056: every local `cargo test` shares
one Postgres server with the running dev stack, and nothing *stops* a test
process from opening the exact database a person reads at `localhost:3000`.
ADR-0056 solved *where tests should write*; it left *what happens when a
run ignores that* to human discipline across concurrent AI sessions — the
one thing this repo has repeatedly shown it cannot rely on.

CI complicates the naive fix: CI's ephemeral service database is *also*
named `ringmaster` ([ci.yml](../../.github/workflows/ci.yml):
`postgres://ringmaster:ringmaster-ci@localhost:5432/ringmaster`), so a
guard keyed on the name `ringmaster` alone would fail CI. The database
*name* is the only stable, host-independent discriminator available to a
test process, so enforcement and CI must agree on one name.

## Decision

Make isolation **enforced at runtime**, not merely documented, by adding a
single test-only guard and unifying every test surface on the isolated
database name.

### A runtime guard every `test_pool()` calls

- A `#[cfg(test)]` function `guard_test_database(database_url)` in
  [`backend/src/lib.rs`](../../backend/src/lib.rs) parses the database name
  from `DATABASE_URL` (the path segment after the final `/`, minus any
  `?`/`#` query) and **panics unless it is exactly `ringmaster_test`**.
- Each of the six duplicated `test_pool()` helpers (in `api`, `audit`,
  `extraction`, `graph`, `obligation`, `transcript`) calls
  `crate::guard_test_database(&database_url)` immediately after reading the
  environment variable, before opening a connection. A run that still
  targets `ringmaster` (or anything else) fails loudly on the first test
  instead of silently writing fixtures into what a person looks at.
- The guard's own logic is unit-tested without a database
  (`guard_accepts_isolated_database`, `guard_rejects_dev_database`), so the
  enforcement itself has proof, not just an assertion.

### One database name across every test surface

- CI's service database and `DATABASE_URL` are renamed from `ringmaster`
  to `ringmaster_test` so CI passes the same guard local runs do — making
  `ringmaster_test` the single, uniform "tests run here" name everywhere,
  and closing the gap where CI and local disagreed on a name.
- The local convention already documented in
  [docs/CONTRIBUTING.md](../CONTRIBUTING.md) (ADR-0056) is unchanged and
  now backed by enforcement rather than trust.

## Scope

**In scope:** the `#[cfg(test)]` guard function and its unit tests; adding
the guard call to all six existing `test_pool()` helpers; renaming CI's
service database and `DATABASE_URL` to `ringmaster_test`.

**Out of scope, named honestly:**

- **Consolidating the six duplicated `test_pool()` functions into one
  shared helper.** Still a legitimate, still an unrelated refactor
  (ADR-0056 said the same); each helper gains one call line, nothing more.
- **Running any cleanup `DELETE` against the existing `ringmaster` data.**
  Unchanged from ADR-0056 — the guard prevents *new* pollution; the
  ~existing rows still await a human-reviewed `dev-data-cleanup.sql` run.
- **Changing the running `backend`/`frontend` containers' database.** Only
  test processes are guarded; the app still reads `ringmaster` by design.
- **A schema-level or connection-level isolation mechanism** (separate
  server, per-test transaction rollback) — ADR-0056 already weighed and
  rejected these as more machinery than the problem needs; this ADR only
  hardens the chosen approach.

## Options considered

- **A runtime guard requiring the `ringmaster_test` name, plus renaming
  CI to match (chosen):** one small test-only function, one call line per
  helper, one CI env change — turns the existing convention into an
  enforced invariant with a single, uniform database name and no
  host/password brittleness.
- **A guard that denies only the literal name `ringmaster`:** would need no
  CI change *if* CI used a different name, but CI's database is named
  `ringmaster` too, so this either fails CI or forces host/password
  sniffing to tell dev from CI — more fragile than agreeing on one name.
- **Leave enforcement to the documented convention (ADR-0056 status quo):**
  rejected — already demonstrated insufficient twice in one session under
  concurrent sessions that keep carrying the old `DATABASE_URL`.
- **A shared `connect_test_pool()` that reads env, guards, and builds the
  pool, replacing all six bodies:** cleaner long-term, but a larger diff
  across six perpetually-contended files for the same runtime guarantee
  one added line per helper already gives; deferred with the consolidation
  refactor above.

## Consequences

- **Positive:** a test run that ignores the isolation convention now fails
  immediately and legibly instead of silently repolluting the dev
  database — the exact failure mode observed twice this session is closed
  structurally, not by discipline.
- **Positive:** `ringmaster_test` becomes the one "tests run here" name
  across local and CI, removing the dev/CI name divergence.
- **Negative / trade-off:** a developer who deliberately wants tests to run
  against a differently-named throwaway database must rename it to
  `ringmaster_test` or adjust the guard — an intentional constraint, the
  point of enforcement.
- **Risk:** low — the change is test-only Rust plus a CI env rename against
  an ephemeral database; no production/runtime path and no change to the
  app's own database.

## Exit criteria and evidence

Evidence: [EV-0057](../evidence.d/0057-enforce-test-database-isolation-with-a-runtime-guard.md)

| Exit criterion | Evidence |
|---|---|
| A test-only guard requires the isolated `ringmaster_test` database | `guard-requires-isolated-database` |
| Every `test_pool()` helper invokes the guard before connecting | `every-test-pool-invokes-the-guard` |
| The guard's accept/reject behavior is unit-tested | `guard-behavior-is-unit-tested` |
| CI runs tests against `ringmaster_test`, matching the guard | `ci-tests-target-the-isolated-database` |
| The full backend suite passes against `ringmaster_test` with the guard active | `backend-suite-passes-under-the-guard` |
