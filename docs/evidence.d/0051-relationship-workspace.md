# EV-0051: Relationship workspace — People shows who needs something from you, not every person node

Evidence for [ADR-0051](../adr.d/0051-relationship-workspace.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0051-relationship-workspace"

[[check]]
id = "person-list-filters-by-needs-attention"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once GET /api/nodes?node_type=person&needs_attention=true returns only people with a linked open/at-risk Obligation."

[[check]]
id = "person-list-unchanged-without-filter"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "A negative/unchanged-behavior claim; verified by direct test run once implemented, matching EV-0039's own precedent for this kind of claim."

[[check]]
id = "person-detail-includes-last-interaction-at"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once GET /api/nodes/:id (person) includes last_interaction_at derived from matching source_fragments.occurred_at."

[[check]]
id = "person-detail-relationship-includes-risk-signals"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once each Obligation in a person's relationship grouping includes risk_signals."

[[check]]
id = "people-tab-defaults-to-needing-attention"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once People.tsx defaults to the needs_attention filter with a Show everyone toggle."
```

## Notes

Pre-implementation: all five checks are deliberately `manual`/unproven, per
this repo's own convention. Do not implement before
[ADR-0051](../adr.d/0051-relationship-workspace.md)'s Status flips to
Accepted.
