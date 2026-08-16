# EV-0046: Unowned-obligation risk signal via existing `owns` edges

Evidence for [ADR-0046](../adr.d/0046-unowned-obligation-risk-signal.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0046-unowned-obligation-risk-signal"

[[check]]
id = "unowned-signal-is-a-pure-function-of-has-owner"
invariant = "risk_signals pushes unowned when has_owner is false, and never when true."
type = "present"
pattern = 'fn risk_signals\('
paths = ["backend/src/api.rs"]

[[check]]
id = "daily-brief-computes-has-owner-via-owns-edge"
invariant = "GET /api/daily-brief flags an Obligation with no owns edge as unowned, and does not flag one that has one."
type = "present"
pattern = 'daily_brief_flags_an_obligation_with_no_owns_edge_as_unowned'
paths = ["backend/src/api.rs"]

[[check]]
id = "time-horizon-computes-has-owner-via-owns-edge"
invariant = "GET /api/time-horizon does the same."
type = "present"
pattern = 'time_horizon_flags_an_obligation_with_no_owns_edge_as_unowned'
paths = ["backend/src/api.rs"]

[[check]]
id = "no-frontend-change-required"
invariant = "No frontend change was needed or made -- both views already render risk_signals generically."
type = "absent"
pattern = 'unowned'
paths = ["frontend/src/components/DailyBrief.tsx", "frontend/src/components/TimeHorizon.tsx"]
```

## Notes

All four checks are automated. The first three are `present`-type against
the implementing function/tests; the fourth is deliberately `absent` --
proof that neither frontend component needed to name the new signal at
all, since both already iterate `risk_signals` generically.
