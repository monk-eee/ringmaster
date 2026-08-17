# ADR-0055: Dev-database hygiene — run tests against a dedicated database, not the dev app's

- **Status:** Proposed
- **Date:** 2026-08-17
- **Decider:** monk-eee
- **Approval:** Pending — awaiting monk-eee's decision
- **Depends on:** [ADR-0005](0005-adopt-rust-event-sourced-postgres-commitment-graph.md), [ADR-0006](0006-local-development-stack-runs-via-podman-compose.md), [ADR-0017](0017-add-github-actions-ci-pipeline.md)
- **Tags:** infrastructure, testing, operations

## Context

[docs/current-status.md](../current-status.md)'s audit named this the
headline operational problem: the dev database is **~99% test-fixture
noise** — ~2,025 obligations (1,698 flagged "needs attention"), 1,007 person
nodes ("Filter Test Person", "Node Route Test Person"), 505 meetings, 1,008
candidates — none of it real, all the residue of months of `cargo test`
runs. It makes the flagship Today page read "1,656 things need your
attention" and unusable as a demo of the actual product.

The root cause is precise and confirmed, not vague: **locally, `cargo test`
connects to the exact same database the running dev app reads.** Every
test's `test_pool()` uses `DATABASE_URL`, and the way tests are run in this
environment points that at
`postgres://ringmaster:ringmaster-dev@postgres:5432/ringmaster` — the same
`ringmaster` database `compose.yaml` serves to the backend
([ADR-0006](0006-local-development-stack-runs-via-podman-compose.md), line
27). Tests create real nodes/obligations/candidates and, by design
([ADR-0005](0005-adopt-rust-event-sourced-postgres-commitment-graph.md)),
never delete them — the event log is append-only. CI is unaffected: it
provisions its own ephemeral `ringmaster-ci` Postgres service
([ADR-0017](0017-add-github-actions-ci-pipeline.md)), thrown away each run.
So this is strictly a **local** hygiene gap: nothing separates "a test wrote
this" from "a real event happened" in the one database a person looks at.

This isn't a code bug — the event-sourcing is correct and does exactly what
it's told. It's that until ingestion existed
([ADR-0040](0040-dated-source-ingestion.md)), there was never a path for
real data to enter, so test data sharing the dev DB never mattered. Now it
does.

## Decision

- **Tests connect to a dedicated `ringmaster_test` database, never the dev
  app's `ringmaster` database.** The local test invocation sets
  `DATABASE_URL=…/ringmaster_test`; `compose.yaml`'s Postgres service gains
  an init step (or a documented one-liner) that creates `ringmaster_test`
  alongside `ringmaster`, and the test database has the same migrations
  applied. The dev `ringmaster` database is thereafter only ever written by
  the running app and real ingestion.
- **A one-time reset for the already-polluted dev `ringmaster` database.**
  Because the append-only triggers are `BEFORE UPDATE`/`BEFORE DELETE`
  (row-level), they reject `DELETE` but do **not** fire on `TRUNCATE` — so a
  bounded `TRUNCATE`-based reset script (all event, projection, node, edge,
  fragment, embedding, candidate, audit tables) is permitted and is the
  honest way to clear accumulated fixtures without weakening the
  immutability guarantee for normal operation. The app rebuilds its
  projection empty on next boot.
- **The contributor guide and `AGENTS.md`'s validation section document
  running tests against `ringmaster_test`**, so no future run repollutes the
  dev database.

## Scope

**In scope:** pointing local test runs at a dedicated `ringmaster_test`
database; provisioning that database in the local stack; a one-time
`TRUNCATE`-based reset for the existing polluted dev database; documenting
the split.

**Out of scope, named honestly:**

- **Per-test transactional isolation** (wrap each test in a transaction that
  rolls back). It's the more rigorous long-term answer and would also fix
  the intermittent full-suite projection-rebuild flake noted in memory, but
  it touches every `test_pool()` call site and every test — a materially
  larger change than separating the database, and a reasonable *later* ADR
  once the database split proves out.
- **Seed fixtures for a curated demo dataset.** A clean empty dev DB plus
  real ingestion ([ADR-0040](0040-dated-source-ingestion.md)) is enough to
  demo the real product; a canned demo seed is separate, optional work.
- **CI changes.** CI already uses a throwaway database and is not affected.

## Options considered

- **Dedicated `ringmaster_test` database + one-time reset (chosen):** the
  smallest change that removes the actual cause (shared database) rather
  than repeatedly cleaning up after it; keeps the dev DB pristine for the
  app/demo; no code-logic change, no weakening of the append-only guarantee.
- **Per-test transactional rollback isolation:** more rigorous but a much
  larger, riskier refactor of every test; deferred to a later ADR rather
  than bundled here.
- **Just add a reset script, keep sharing the database:** rejected — treats
  the symptom, not the cause; the dev DB would be repolluted the next time
  anyone runs the suite.
- **Do nothing / clean the DB manually when demoing:** rejected — the audit
  shows this is exactly what has happened for months; it doesn't scale and
  makes the flagship surface untrustworthy.

## Consequences

- **Positive:** the dev database stays a faithful mirror of real
  (ingested/app-created) data, so Today/People/Timeline actually demo the
  product; removes the single biggest thing making the flagship surface
  look broken; no change to event-sourcing correctness or the append-only
  guarantee.
- **Negative / trade-off:** contributors and agents must remember to target
  `ringmaster_test` (mitigated by documenting it in `AGENTS.md`/the
  contributor guide and the standard test invocation); the one-time reset is
  destructive and must be run deliberately, once.
- **Risk:** low-to-moderate. The database split is configuration, not logic;
  the reset is a bounded, documented, one-time `TRUNCATE`. The main residual
  risk is a stray future run that forgets the `ringmaster_test` target —
  which documentation and a shared default address but cannot fully prevent
  until per-test isolation (the deferred option) exists.

## Exit criteria and evidence

Evidence: [EV-0055](../evidence.d/0055-dev-database-hygiene-separate-test-db.md)

| Exit criterion | Evidence |
|---|---|
| The local stack provisions a `ringmaster_test` database distinct from `ringmaster` | `test-database-provisioned` |
| A bounded reset for the polluted dev database exists and is documented | `dev-db-reset-exists` |
| Contributor docs direct test runs at `ringmaster_test`, not the dev DB | `docs-direct-tests-to-test-db` |
