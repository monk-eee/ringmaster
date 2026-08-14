# EV-0023: Evidence-backed Daily Brief reasons — source-fragment traceability on Obligation

Evidence for [ADR-0023](../adr.d/0023-evidence-backed-daily-brief-reasons.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0023-evidence-backed-daily-brief-reasons"

[[check]]
id = "obligation-source-fragment-id-preserved"
invariant = "obligation_projection carries a nullable source_fragment_id, preserved by rebuild_projection across events that don't name it."
type = "present"
pattern = 'fn payload_uuid\('
paths = ["backend/src/obligation.rs"]

[[check]]
id = "obligations-route-includes-source-fields"
invariant = "GET /api/obligations includes source_fragment_id and source_text for each row."
type = "present"
pattern = '"source_text": source_text'
paths = ["backend/src/api.rs"]

[[check]]
id = "daily-brief-reason-cites-evidence"
invariant = "GET /api/daily-brief's reason states the linked source evidence, or that none is recorded."
type = "present"
pattern = 'No evidence recorded\.'
paths = ["backend/src/api.rs"]
```

## Notes

All three checks are automated and verified directly against the
implementing migration/route files. `cargo test` covers both branches:
`obligations_route_includes_source_fragment_evidence` (a linked fragment's
`source_fragment_id`/`source_text` surface on `GET /api/obligations`) and
`daily_brief_reason_cites_evidence_when_linked_and_states_none_when_not`
(the Daily Brief `reason` cites the linked quote for one obligation and
states "No evidence recorded." for an otherwise-identical one with no
link).
