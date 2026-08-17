# Contributing

## Before implementation

Every change to source, tests, configuration, infrastructure, or pipelines must
be covered by an accepted ADR in [`docs/adr.d/`](adr.d/README.md) before
implementation. Reuse an existing record only when its decision and scope apply.
Otherwise, use the repository ADR authoring skill to draft a bounded decision
and matching evidence record, then obtain explicit acceptance from its named
decider.

Purely editorial corrections that do not change behavior, constraints,
interfaces, or operating rules may use the `N/A - editorial` exemption.

## Pull requests

Use the GitHub pull request template and link the governing ADR or state the
editorial exemption. Keep a pull request focused on one reviewable outcome and
include the validation evidence needed to assess it.

Before requesting review, run:

```bash
node scripts/check-evidence.mjs
git diff --check
```

Add project-specific build, format, lint, and test commands here when the Rust
and Node project structures are established under accepted ADRs.

## Backend tests

Per [ADR-0056](adr.d/0056-local-test-database-isolation-and-dev-data-cleanup.md),
backend tests run against a dedicated `ringmaster_test` database, never the
`ringmaster` database the running dev app and real ingestion write to:

```bash
podman run --rm --network ringmaster_default \
  -e DATABASE_URL="postgres://ringmaster:ringmaster-dev@postgres:5432/ringmaster_test" \
  -v "$PWD":/app:Z -w /app docker.io/library/rust:1-slim \
  cargo test --manifest-path backend/Cargo.toml -- --test-threads=1
```

`ringmaster_test` is created automatically on a fresh `podman compose up`
volume (`docker-entrypoint-initdb.d/create-test-db.sql`); on an
already-initialized volume, create it once by hand
(`CREATE DATABASE ringmaster_test;`) and apply every file under
`backend/migrations/` in order with `psql`, the same way CI provisions its
own ephemeral database.