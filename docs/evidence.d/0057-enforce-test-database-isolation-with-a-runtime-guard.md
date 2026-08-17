# EV-0057: Enforce test-database isolation with a runtime guard

Evidence for [ADR-0057](../adr.d/0057-enforce-test-database-isolation-with-a-runtime-guard.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0057-enforce-test-database-isolation-with-a-runtime-guard"

[[check]]
id = "guard-requires-isolated-database"
invariant = "A test-only guard function in lib.rs requires the isolated ringmaster_test database."
type = "present"
pattern = "guard_test_database"
paths = ["backend/src/lib.rs"]

[[check]]
id = "every-test-pool-invokes-the-guard"
invariant = "Every backend test_pool() helper invokes guard_test_database before connecting."
type = "present"
pattern = "guard_test_database"
paths = ["backend/src/api.rs", "backend/src/audit.rs", "backend/src/extraction.rs", "backend/src/graph.rs", "backend/src/obligation.rs", "backend/src/transcript.rs"]

[[check]]
id = "guard-behavior-is-unit-tested"
invariant = "The guard's reject-dev-database behavior is unit-tested."
type = "present"
pattern = "guard_rejects_dev_database"
paths = ["backend/src/lib.rs"]

[[check]]
id = "ci-tests-target-the-isolated-database"
invariant = "CI runs backend tests against ringmaster_test, matching the guard."
type = "present"
pattern = "ringmaster_test"
paths = [".github/workflows/ci.yml"]

[[check]]
id = "backend-suite-passes-under-the-guard"
invariant = "The full backend suite passes against ringmaster_test with the guard active."
type = "manual"
last_verified = "2026-08-17"
rationale = "Proving a live test run passes is not a file-content regex. Verified directly: ran the full backend suite via podman against ringmaster_test with --test-threads=1; all tests passed, and a control run against DATABASE_URL=...ringmaster panicked in guard_test_database before any connection, confirming the guard blocks the dev database."
```

## Notes

This ADR amends [ADR-0056](../adr.d/0056-local-test-database-isolation-and-dev-data-cleanup.md):
0056 chose a documented convention as the enforcement mechanism and "No Rust
code changes"; this record adds a runtime guard because the convention proved
advisory-only under concurrent sessions. The cleanup half of 0056 is unchanged
and still awaits a human-reviewed run.
