# ADR-0056: Local test-database isolation, plus a reviewable (not auto-run) dev-data cleanup

- **Status:** Proposed
- **Date:** 2026-08-17
- **Decider:** monk-eee
- **Depends on:** [ADR-0006](0006-local-development-stack-runs-via-podman-compose.md), [ADR-0040](0040-dated-source-ingestion.md)
- **Tags:** architecture, infrastructure, testing, data-hygiene

## Context

[docs/current-status.md](../current-status.md)'s live audit found the
local dev Postgres holds roughly 2,025 Obligations, 1,007 person nodes,
505 meeting nodes, and 1,008 candidates — almost entirely test-fixture
residue ("Filter Test Person", "Marked at risk. No evidence recorded.",
"Due in 1232 day(s)...") from months of `cargo test` runs, not monk-eee's
real work. Repo memory independently confirms two concrete symptoms
already traced to the same root cause before this audit connected them:
recurring embedding-search flakiness (near-duplicate embeddings from the
same fixture text crowding out real results) and audit-row count-delta
test races. This is why: CI's Postgres is an ephemeral GitHub Actions
service container, fresh every run, so it never accumulates anything —
but every local `cargo test` invocation (this session's own established
pattern, `podman run ... -e DATABASE_URL=postgres://...@postgres:5432/ringmaster
... cargo test`) points at the **exact same** long-lived `ringmaster`
database and volume the running dev stack — and a person opening
`localhost:3000` — actually reads. Nothing before now needed to
distinguish "a test wrote this" from "a real event happened," because
until ingestion ([ADR-0040](0040-dated-source-ingestion.md)) existed there
was no path for real data to exist at all.

This is two different problems needing two different postures:

1. **Prevention (future pollution).** Purely additive, safe, reversible —
   give tests a separate database so no future `cargo test` run ever
   touches what a person actually looks at.
2. **Cleanup (the ~4,500 rows already there).** A bulk deletion against a
   shared database that other concurrent sessions actively read and write
   right now. This repo's own operational-safety posture treats deleting
   data on shared infrastructure as needing explicit confirmation before
   it runs, not just before a final irreversible step — this ADR proposes
   the criteria and a read-only report; it does not authorize running any
   `DELETE` as part of accepting it.

## Decision

### Prevention: `ringmaster_test`, a second database on the same Postgres

- `compose.yaml`'s `postgres` service gains an init script
  (`docker-entrypoint-initdb.d/create-test-db.sql`, run automatically by
  the postgres image on a **fresh** volume only) that creates a second,
  empty database, `ringmaster_test`, alongside `ringmaster` on the same
  server/container — no new service, no new container, no port change.
- For the **already-initialized** volume this environment already has
  (where the init script won't retroactively run), `ringmaster_test` is
  created once by hand with a plain `CREATE DATABASE ringmaster_test;` —
  additive and non-destructive; it does not read, lock, or modify
  `ringmaster` in any way.
- Every backend migration is applied to `ringmaster_test` the same way
  CI already applies them (`psql "$DATABASE_URL" -f <migration>`, in
  order) — `ringmaster_test` needs its own schema since
  `sqlx::migrate!` only runs inside the `ringmaster-backend` binary's own
  startup ([`main.rs`](../../backend/src/main.rs)), never automatically
  for a test process.
- **No Rust code changes.** Every `test_pool()` helper across the six
  files that define one already reads `DATABASE_URL` from the
  environment; pointing that variable at `ringmaster_test` instead of
  `ringmaster` is the entire fix. Repo memory's own documented
  "how to run backend tests" convention is updated to the new URL, and
  becomes the enforcement mechanism this repo already relies on for
  cross-session conventions.

### Cleanup: a report first, criteria in the open, no automated delete

- `scripts/dev-data-report.sql`: read-only `SELECT`s reporting, per table
  (`nodes` grouped by `node_type`, `obligation_projection`,
  `candidate_projection`), a count matching a disclosed, conservative
  test-fixture heuristic — `canonical_text`/`statement` containing the
  case-insensitive substring `test`, or an Obligation whose
  `source_fragment_id IS NULL` (today, the *only* path that creates a
  real Obligation is promoting an accepted candidate,
  [ADR-0027](0027-promote-accepted-candidate-to-obligation.md), which
  always carries a `source_fragment_id` forward — a null one has never
  been through that real flow) — alongside the count that does **not**
  match, so a reader sees both sides, not just a number to trust.
- `scripts/dev-data-cleanup.sql` (drafted, reviewable, **not executed by
  accepting this ADR**): the matching `DELETE` statements for the same
  criteria, run manually, once, only after a human reviews
  `dev-data-report.sql`'s actual output against this specific database —
  this ADR's exit criteria do not include having run it.
- Both scripts operate on `obligation_events`/`candidate_events` (the
  source of truth) plus `nodes`/`edges`/`source_fragments`, then require
  the existing `rebuild_projection`/`rebuild_candidate_projection`
  functions to be re-run (already-existing, already-tested code, not new
  logic) so the projections reflect the trimmed event log — never
  patched in place.

## Scope

**In scope:** the `ringmaster_test` init script and one-time manual
creation; applying migrations to it; updating the documented test-run
convention; `dev-data-report.sql` (read-only, safe to run immediately);
drafting (not running) `dev-data-cleanup.sql`.

**Out of scope, named honestly:**

- **Running any cleanup `DELETE` against the current `ringmaster`
  database.** Requires its own explicit go-ahead at execution time,
  separate from accepting this ADR's criteria and scripts.
- **A generalized "is this row a test fixture" flag or schema change.**
  The heuristic here is pattern-matching against existing data, not a
  new, permanent classification mechanism — a real, different, larger
  decision if ever wanted.
- **Rewriting the six duplicated `test_pool()` functions into one shared
  helper.** A legitimate cleanup, but an unrelated refactor to the
  database-isolation problem this ADR solves.
- **Changing CI.** CI's Postgres is already fresh per run and already
  unaffected by this problem.
- **Any change to `main.rs`'s own migration-on-boot behavior**, or to
  which database the running `backend`/`frontend` containers use — only
  test invocations change database.

## Options considered

- **A second database on the same Postgres server (chosen):** no new
  service/container/port, minimal `compose.yaml` change, zero Rust code
  change — the smallest change that stops tests from writing into what a
  person looks at.
- **A separate ephemeral Postgres container for local tests, mirroring
  CI exactly:** would isolate perfectly and match CI's own posture, but
  adds a second container to manage locally and a slower per-run startup
  cost CI doesn't pay (its service container's lifetime spans the whole
  job); rejected as more machinery than the problem needs.
- **Wrap each test in a transaction that always rolls back:** would need
  every `test_pool()` call site changed to hand out a shared transaction
  instead of a pool connection, and doesn't compose cleanly with code
  under test that opens its own nested transactions (e.g.
  [ADR-0034](0034-http-meeting-transcript-ingestion.md)'s atomic
  ingestion) — a materially larger refactor for the same outcome a second
  database gets for free.
- **Automatically delete matching rows as part of this ADR's
  implementation:** rejected — a bulk, heuristic-based deletion against a
  database other concurrent sessions are actively using is exactly the
  kind of hard-to-reverse, shared-infrastructure action this repo's own
  operational posture requires explicit confirmation for, not an
  ADR-acceptance-implies-execute action.

## Consequences

- **Positive:** every future local `cargo test` run stops adding to the
  pile a person actually looks at; closes the root cause behind two
  already-observed classes of test flakiness (embedding crowding,
  audit-count races), not just their symptoms.
- **Positive:** the cleanup criteria and report are disclosed and
  reviewable before anything is deleted, not asserted as already-decided.
- **Negative / trade-off:** the ~4,500 existing rows remain until someone
  explicitly runs the cleanup script after reviewing the report — this
  ADR does not make Today/People instantly demo-clean.
- **Risk:** low for the prevention half (purely additive: a new empty
  database, a documented convention change). The cleanup half's risk is
  explicitly deferred to whoever reviews and runs it, not incurred by
  accepting this record.

## Exit criteria and evidence

Evidence: [EV-0056](../evidence.d/0056-local-test-database-isolation-and-dev-data-cleanup.md)

| Exit criterion | Evidence |
|---|---|
| `ringmaster_test` exists and every migration is applied to it | `test-database-exists-and-migrated` |
| The documented test-run convention points at `ringmaster_test`, not `ringmaster` | `test-convention-uses-isolated-database` |
| `dev-data-report.sql` runs read-only and reports both matching and non-matching counts | `dev-data-report-is-read-only-and-reports-both-sides` |
| `dev-data-cleanup.sql` exists, targets the same disclosed criteria, and is not invoked by any automated process | `cleanup-script-exists-and-is-not-automated` |
