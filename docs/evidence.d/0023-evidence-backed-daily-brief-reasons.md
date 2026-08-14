# EV-0023: Evidence-backed Daily Brief reasons — source-fragment traceability on Obligation

Evidence for [ADR-0023](../adr.d/0023-evidence-backed-daily-brief-reasons.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0023-evidence-backed-daily-brief-reasons"

[[check]]
id = "obligation-source-fragment-id-preserved"
invariant = "obligation_projection carries a nullable source_fragment_id, preserved by rebuild_projection across events that don't name it."
type = "manual"

[[check]]
id = "obligations-route-includes-source-fields"
invariant = "GET /api/obligations includes source_fragment_id and source_text for each row."
type = "manual"

[[check]]
id = "daily-brief-reason-cites-evidence"
invariant = "GET /api/daily-brief's reason states the linked source evidence, or that none is recorded."
type = "manual"
```

## Notes

All three checks are `manual` and unverified (`ASSERTED`) because ADR-0023
is **Proposed**, not yet accepted or implemented. Once accepted, replace
each with a `present` pattern check against the implementing migration and
route module, mirroring EV-0015/EV-0020's shape.
