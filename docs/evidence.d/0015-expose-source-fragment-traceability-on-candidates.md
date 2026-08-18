# EV-0015: Expose source-fragment traceability on candidates

Evidence for [ADR-0015](../adr.d/0015-expose-source-fragment-traceability-on-candidates.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0015-expose-source-fragment-traceability-on-candidates"

[[check]]
id = "source-fragment-id-column-exists"
invariant = "candidate_projection carries source_fragment_id, populated by the projection rebuild from the candidate's extracted event payload."
type = "present"
pattern = 'ALTER TABLE candidate_projection ADD COLUMN source_fragment_id'
paths = ["backend/migrations/0007_candidate_projection_source_fragment.sql"]

[[check]]
id = "candidates-route-includes-source-fields"
invariant = "GET /api/candidates includes source_fragment_id, source_text, and speaker for each row via a read-only join against source_fragments."
type = "present"
pattern = 'LEFT JOIN source_fragments'
paths = ["backend/src/api/candidates.rs"]
```

## Notes

Both checks are automated against the migration and API module that
implement them. `cargo test` cases exercise: the projection rebuild
populating `source_fragment_id` from the extracted event's own payload, and
`GET /api/candidates` returning `source_fragment_id`/`source_text`/`speaker`
for a candidate extracted against a real, ingested transcript fragment.
