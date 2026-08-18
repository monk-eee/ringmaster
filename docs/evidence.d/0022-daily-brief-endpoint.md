# EV-0022: A read-only Daily Brief endpoint — Obligations ranked by urgency

Evidence for [ADR-0022](../adr.d/0022-daily-brief-endpoint.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0022-daily-brief-endpoint"

[[check]]
id = "daily-brief-route-exists"
invariant = "GET /api/daily-brief ranks non-closed obligations by urgency: at-risk status, then soonest hard_due_at, then soonest soft_due_at, then most-recently-updated."
type = "present"
pattern = 'fn daily_brief\('
paths = ["backend/src/api/obligations.rs"]

[[check]]
id = "daily-brief-includes-reason"
invariant = "Each ranked item includes a deterministic, evidence-free reason string derived only from its own status/due-date fields."
type = "present"
pattern = 'fn daily_brief_reason\('
paths = ["backend/src/api/obligations.rs"]
```

## Notes

Both checks are automated and verified directly against the implementing
route/function in `backend/src/api.rs`. `cargo test` exercises the route
end to end (`daily_brief_ranks_at_risk_first_and_excludes_closed`): an
at-risk obligation outranks an open one even with a distant hard due
date, a closed obligation never appears, and the `reason` string matches
("Marked at risk."). Live-verified against the running backend container
as well.
