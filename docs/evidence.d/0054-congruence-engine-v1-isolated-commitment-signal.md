# EV-0054: Congruence Engine v1 — flag a commitment with no linked node at all

Evidence for [ADR-0054](../adr.d/0054-congruence-engine-v1-isolated-commitment-signal.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0054-congruence-engine-v1-isolated-commitment-signal"

[[check]]
id = "isolated-signal-flags-a-zero-edge-commitment"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once a commitment-type Obligation with zero edges is flagged with an isolated risk signal."

[[check]]
id = "isolated-signal-does-not-flag-a-linked-commitment"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once a commitment-type Obligation with at least one edge is not flagged isolated."

[[check]]
id = "isolated-signal-attached-like-existing-signals"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once isolated appears in risk_signals on Daily Brief and Time Horizon, reusing the existing signal attachment pattern."
```

## Notes

Pre-implementation: all three checks are deliberately `manual`/unproven,
per this repo's own convention. Do not implement before
[ADR-0054](../adr.d/0054-congruence-engine-v1-isolated-commitment-signal.md)'s
Status flips to Accepted.
