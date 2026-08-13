# EV-0005: Adopt a Rust service with an event-sourced Postgres commitment graph

Evidence for [ADR-0005](../adr.d/0005-adopt-rust-event-sourced-postgres-commitment-graph.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0005-adopt-rust-event-sourced-postgres-commitment-graph"

[[check]]
id = "vision-names-storage-decision"
invariant = "The vision document names ADR-0005 as addressing the backend/storage architecture direction."
type = "present"
pattern = 'ADR-0005'
paths = ["docs/VISION.md"]

[[check]]
id = "backend-is-rust"
invariant = "The backend service is implemented in Rust."
type = "present"
pattern = 'name = "ringmaster-backend"'
paths = ["backend/Cargo.toml"]

[[check]]
id = "events-are-immutable"
invariant = "Commitment events are appended immutably; the database rejects mutation or deletion of an existing event row."
type = "present"
pattern = 'reject_obligation_event_mutation'
paths = ["backend/migrations/0001_obligation_events.sql"]

[[check]]
id = "projections-are-derived"
invariant = "rebuild_projection always truncates and rewrites the projection from the full event log, never patching it in place."
type = "present"
pattern = 'TRUNCATE obligation_projection'
paths = ["backend/src/obligation.rs"]
```

## Notes

[ADR-0007](../adr.d/0007-generalize-obligation-and-require-pgvector.md)
renamed the aggregate and its schema from Commitment to Obligation; the
checks below point at the renamed files, which continue to satisfy this
ADR's event-sourcing and derived-projection guarantees unchanged.

All four checks are automated and verified directly against the migration
and crate that implement them. Two `cargo test` cases exercise the actual
invariants against a live Postgres instance and both pass:
`event_rows_cannot_be_mutated_or_deleted` (the append-only trigger rejects
`UPDATE`/`DELETE`) and `projection_is_rebuilt_from_the_event_log_alone` (the
projection reflects a later `status_changed` event only after
`rebuild_projection` reruns against the full log). `events-are-immutable`
was additionally confirmed by hand earlier: inserting a row and then
attempting `UPDATE`/`DELETE` against it both raised the trigger's exception
inside the running container.

Remaining open work under this ADR: an MCP-based ingestion adapter for
MindLeak facts ([ADR-0003](../adr.d/0003-ringmaster-ingests-mindleak-as-an-mcp-source.md))
is currently blocked — the installed MindLeak binaries are macOS arm64 and
cannot run inside this service's Linux container. That blocker is recorded
as durable knowledge linked to ADR-0003 and `backend/Cargo.toml`, not hidden
here.
