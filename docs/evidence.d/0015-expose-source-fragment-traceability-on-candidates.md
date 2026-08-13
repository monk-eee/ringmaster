# EV-0015: Expose source-fragment traceability on candidates

Evidence for [ADR-0015](../adr.d/0015-expose-source-fragment-traceability-on-candidates.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0015-expose-source-fragment-traceability-on-candidates"

[[check]]
id = "source-fragment-id-column-exists"
invariant = "candidate_projection carries source_fragment_id, populated by the projection rebuild from the candidate's extracted event payload."
type = "manual"

[[check]]
id = "candidates-route-includes-source-fields"
invariant = "GET /api/candidates includes source_fragment_id, source_text, and speaker for each row via a read-only join against source_fragments."
type = "manual"
```

## Notes

Both checks are `manual` and unverified (`ASSERTED`) because ADR-0015 is
**Proposed**, not yet accepted or implemented. Once accepted, replace both
with `present` pattern checks against the migration and route module that
implement them, mirroring EV-0013's shape.
