# EV-0022: A read-only Daily Brief endpoint — Obligations ranked by urgency

Evidence for [ADR-0022](../adr.d/0022-daily-brief-endpoint.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0022-daily-brief-endpoint"

[[check]]
id = "daily-brief-route-exists"
invariant = "GET /api/daily-brief ranks non-closed obligations by urgency: at-risk status, then soonest hard_due_at, then soonest soft_due_at, then most-recently-updated."
type = "manual"

[[check]]
id = "daily-brief-includes-reason"
invariant = "Each ranked item includes a deterministic, evidence-free reason string derived only from its own status/due-date fields."
type = "manual"
```

## Notes

Both checks are `manual` and unverified (`ASSERTED`) because ADR-0022 is
**Proposed**, not yet accepted or implemented. Once accepted, replace both
with `present` pattern checks against the implementing route module,
mirroring EV-0019/EV-0020's shape.
