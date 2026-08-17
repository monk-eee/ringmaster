# EV-0056: Local test-database isolation, plus a reviewable (not auto-run) dev-data cleanup

Evidence for [ADR-0056](../adr.d/0056-local-test-database-isolation-and-dev-data-cleanup.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0056-local-test-database-isolation-and-dev-data-cleanup"

[[check]]
id = "test-database-exists-and-migrated"
invariant = "ringmaster_test exists and every migration is applied to it."
type = "manual"
rationale = "Proposed ADR; requires a one-time manual database-creation step against this specific environment's existing Postgres volume, not something a file-content regex can prove."

[[check]]
id = "test-convention-uses-isolated-database"
invariant = "The documented test-run convention points at ringmaster_test, not ringmaster."
type = "manual"
rationale = "Proposed ADR; replace with a present-type check against the updated repo-memory/CONTRIBUTING.md text once implemented."

[[check]]
id = "dev-data-report-is-read-only-and-reports-both-sides"
invariant = "dev-data-report.sql runs read-only and reports both matching and non-matching counts."
type = "manual"
rationale = "Proposed ADR; replace with a present-type check once scripts/dev-data-report.sql exists."

[[check]]
id = "cleanup-script-exists-and-is-not-automated"
invariant = "dev-data-cleanup.sql exists, targets the same disclosed criteria, and is not invoked by any automated process."
type = "manual"
rationale = "Proposed ADR; deliberately stays manual/asserted even after implementation -- proving a script is never auto-invoked is a repo-wide absence claim, not a single-file regex match."
```

## Notes

All four checks are intentionally manual/unverified while this ADR remains
Proposed. Acceptance authorizes implementing the prevention half (the
isolated test database, the report script); it does not by itself
authorize running `dev-data-cleanup.sql` against this environment's
existing data.
