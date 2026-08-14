# EV-0029: Time Horizon view — Obligations bucketed by due-date window

Evidence for [ADR-0029](../adr.d/0029-time-horizon-view.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0029-time-horizon-view"

[[check]]
id = "time-horizon-route-buckets-by-due-date"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once GET /api/time-horizon groups non-closed Obligations into Overdue/7/30/90/Beyond buckets."

[[check]]
id = "at-risk-no-date-lands-in-overdue"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once an at_risk Obligation with no due date is bucketed under Overdue, matching the Daily Brief's own precedent."

[[check]]
id = "closed-excluded-from-time-horizon"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once a closed Obligation is confirmed absent from every bucket."

[[check]]
id = "time-horizon-tab-exists"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once a Time Horizon tab renders the bucketed Obligations in the frontend SPA."
```

## Notes

Pre-implementation: all four checks are deliberately `manual`/unproven,
per this repo's own convention (evidence stays honest about intent vs.
proof until the ADR is accepted and implemented). Do not implement before
[ADR-0029](../adr.d/0029-time-horizon-view.md)'s Status flips to Accepted.
