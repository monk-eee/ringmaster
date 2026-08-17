# EV-0055: Dev-database hygiene — run tests against a dedicated database

Evidence for [ADR-0055](../adr.d/0055-dev-database-hygiene-separate-test-db.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0055-dev-database-hygiene-separate-test-db"

[[check]]
id = "test-database-provisioned"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once the local stack provisions a ringmaster_test database distinct from the dev ringmaster database, with migrations applied."

[[check]]
id = "dev-db-reset-exists"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once a bounded, documented TRUNCATE-based reset for the polluted dev database exists."

[[check]]
id = "docs-direct-tests-to-test-db"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once AGENTS.md / the contributor guide direct local test runs at ringmaster_test rather than the dev ringmaster database."
```

## Notes

Pre-implementation: all three checks are deliberately `manual`/unproven, per
this repo's own convention. Do not implement before
[ADR-0055](../adr.d/0055-dev-database-hygiene-separate-test-db.md)'s Status
flips to Accepted. This ADR is docs-level intent; the implementation
(compose init, reset script, doc updates) follows acceptance.
