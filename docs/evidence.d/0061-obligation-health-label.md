# EV-0061: A derived Obligation health label — composing existing status and signals, not a new score

Evidence for [ADR-0061](../adr.d/0061-obligation-health-label.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0061-obligation-health-label"

[[check]]
id = "obligation-health-returns-fixed-five-values"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once obligation_health returns exactly one of Completed/At Risk/Broken/Stalled/Healthy, never a sixth value or a number."

[[check]]
id = "obligation-health-attached-at-existing-call-sites"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once health is attached alongside risk_signals on Daily Brief, Time Horizon, and Obligation detail."

[[check]]
id = "obligation-health-distinguishes-broken-from-stalled"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once an overdue, still-open Obligation with no stale signal returns Broken, not Stalled."
```

## Notes

Pre-implementation: all three checks are deliberately `manual`/unproven,
per this repo's own convention. Do not implement before
[ADR-0061](../adr.d/0061-obligation-health-label.md)'s Status flips to
Accepted.
