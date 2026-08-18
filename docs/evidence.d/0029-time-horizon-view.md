# EV-0029: Time Horizon view — Obligations bucketed by due-date window

Evidence for [ADR-0029](../adr.d/0029-time-horizon-view.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0029-time-horizon-view"

[[check]]
id = "time-horizon-route-buckets-by-due-date"
invariant = "GET /api/time-horizon groups non-closed Obligations into Overdue/7/30/90/Beyond buckets."
type = "present"
pattern = 'fn time_horizon_bucket\('
paths = ["backend/src/api/obligations.rs"]

[[check]]
id = "at-risk-no-date-lands-in-overdue"
invariant = "An at_risk Obligation with no due date is bucketed under Overdue, matching the Daily Brief's own precedent."
type = "present"
pattern = 'status == "at_risk"[\s\S]*?"overdue"'
paths = ["backend/src/api/obligations.rs"]

[[check]]
id = "closed-excluded-from-time-horizon"
invariant = "A closed Obligation is confirmed absent from every bucket."
type = "present"
pattern = "op.status <> 'closed'"
paths = ["backend/src/api/obligations.rs"]

[[check]]
id = "time-horizon-tab-exists"
invariant = "A tab renders the bucketed Obligations in the frontend SPA (the Timeline tab, renamed from Time Horizon by ADR-0039; same component and route)."
type = "present"
pattern = '"timeline"'
paths = ["frontend/src/App.tsx"]
```

## Notes

All four checks are automated and verified directly against the
implementing route and frontend files. `cargo test` covers:
`time_horizon_buckets_by_due_date_with_the_at_risk_no_date_exception` —
a past-due Obligation lands in `overdue`, an `at_risk` Obligation with no
date also lands in `overdue` (not `beyond`), a near-term Obligation lands
in `next_7_days`, and a closed Obligation never appears in any bucket.
56/56 backend tests pass; `tsc --noEmit` and `vite build` both pass.
