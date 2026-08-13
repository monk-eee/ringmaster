# EV-0008: Add an append-only audit_events table for security-relevant actions

Evidence for [ADR-0008](../adr.d/0008-add-append-only-audit-events-table.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0008-add-append-only-audit-events-table"

[[check]]
id = "audit-events-table-exists"
invariant = "The audit_events table exists with actor, action, previous_state, new_state, source, and policy_outcome columns."
type = "present"
pattern = 'CREATE TABLE audit_events'
paths = ["backend/migrations/0004_audit_events.sql"]

[[check]]
id = "audit-events-are-immutable"
invariant = "The database rejects mutation or deletion of an existing audit_events row."
type = "present"
pattern = 'reject_audit_event_mutation'
paths = ["backend/migrations/0004_audit_events.sql"]

[[check]]
id = "audit-record-function-exists"
invariant = "A Rust function appends one audit row."
type = "present"
pattern = 'pub async fn record'
paths = ["backend/src/audit.rs"]
```

## Notes

All three checks are automated and verified directly against the migration
and crate that implement them. A `cargo test` case exercises the actual
immutability invariant against a live Postgres instance, mirroring the
`obligation_events` test in [EV-0005](0005-adopt-rust-event-sourced-postgres-commitment-graph.md).

No application code calls `record()` yet — per ADR-0008's own scope, wiring
real call sites in is future work as extraction, validation, and other
audited features are built.
