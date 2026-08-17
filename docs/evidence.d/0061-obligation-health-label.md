# EV-0061: A derived Obligation health label — composing existing status and signals, not a new score

Evidence for [ADR-0061](../adr.d/0061-obligation-health-label.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0061-obligation-health-label"

[[check]]
id = "obligation-health-returns-fixed-five-values"
invariant = "obligation_health returns exactly one of Completed/At Risk/Broken/Stalled/Healthy, never a sixth value or a number."
type = "present"
pattern = 'fn obligation_health\('
paths = ["backend/src/api.rs"]

[[check]]
id = "obligation-health-attached-at-existing-call-sites"
invariant = "health is attached alongside risk_signals on Daily Brief, Time Horizon, and Obligation detail."
type = "manual"
last_verified = "2026-08-17"
rationale = "Attachment at three specific routes in one file isn't provable by a single file-content regex (it would only prove 'at least once', not 'at all three'). Verified directly instead, by both reading each of the three response-construction sites and by three passing test assertions, each against a different route: daily_brief_ranks_at_risk_first_and_excludes_closed asserts health on GET /api/daily-brief, time_horizon_route_attaches_risk_signals asserts health on GET /api/time-horizon, and promotion_creates_owns_edge_on_exact_owner_match asserts health on GET /api/obligations/:id."

[[check]]
id = "obligation-health-distinguishes-broken-from-stalled"
invariant = "An overdue, still-open Obligation with no stale signal returns Broken, not Stalled."
type = "present"
pattern = "obligation_health_distinguishes_broken_from_stalled"
paths = ["backend/src/api.rs"]
```

## Notes

Implemented as a pure function reusing `status`/`hard_due_at`/the already-
computed `risk_signals` slice -- no new signal, no schema change, no new
route. Attached at exactly the three call sites this ADR named: `daily_brief`,
`time_horizon`, and `get_obligation_detail`. Deliberately not attached to the
fourth existing `risk_signals` call site (the Person relationship view,
`get_node_detail`) -- outside this ADR's own stated scope. Full backend
suite: 136 passed, 0 failed, against `ringmaster_test`.
