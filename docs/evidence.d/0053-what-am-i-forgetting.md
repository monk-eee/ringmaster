# EV-0053: "What am I forgetting?" — compose existing risk signals into one capped, prominent list

Evidence for [ADR-0053](../adr.d/0053-what-am-i-forgetting.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0053-what-am-i-forgetting"

[[check]]
id = "forgetting-section-capped-and-signal-filtered"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once Today shows a What am I forgetting section listing at most 5 Obligations, each with at least one risk signal."

[[check]]
id = "forgetting-section-ranked-by-signal-count"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once rows are ordered by risk_signals.length descending, then existing Daily Brief order."

[[check]]
id = "forgetting-section-honest-empty-state"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once an honest empty state renders when zero Obligations carry a risk signal."
```

## Notes

Pre-implementation: all three checks are deliberately `manual`/unproven,
per this repo's own convention. Do not implement before
[ADR-0053](../adr.d/0053-what-am-i-forgetting.md)'s Status flips to
Accepted.
