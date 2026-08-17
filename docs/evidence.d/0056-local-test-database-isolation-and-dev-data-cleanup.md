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
last_verified = "2026-08-17"
rationale = "Requires a one-time manual database-creation step against this environment's existing Postgres volume, not something a file-content regex can prove. Verified directly: CREATE DATABASE ringmaster_test, then every backend/migrations/*.sql file applied in order via psql, all succeeded."

[[check]]
id = "test-convention-uses-isolated-database"
invariant = "The documented test-run convention points at ringmaster_test, not ringmaster."
type = "present"
pattern = "ringmaster_test"
paths = ["docs/CONTRIBUTING.md"]

[[check]]
id = "dev-data-report-is-read-only-and-reports-both-sides"
invariant = "dev-data-report.sql runs read-only and reports both matching and non-matching counts."
type = "present"
pattern = "matching_heuristic"
paths = ["scripts/dev-data-report.sql"]

[[check]]
id = "cleanup-script-exists-and-is-not-automated"
invariant = "dev-data-cleanup.sql exists, targets the same disclosed criteria, and is not invoked by any automated process."
type = "manual"
last_verified = "2026-08-17"
rationale = "Deliberately stays manual even after implementation -- proving a script is never auto-invoked is a repo-wide absence claim, not a single-file regex match. Verified directly: grepped compose.yaml, .github/workflows/ci.yml, and package.json scripts for any reference to dev-data-cleanup -- none invoke it, only documentation and the script itself reference the filename."
```

## Notes

All four checks were intentionally manual/unverified while this ADR was
Proposed. Acceptance authorized implementing the prevention half (the
isolated test database, the report script); it does not by itself
authorize running `dev-data-cleanup.sql` against this environment's
existing data.
