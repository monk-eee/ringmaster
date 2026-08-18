# EV-0051: Relationship workspace — People shows who needs something from you, not every person node

Evidence for [ADR-0051](../adr.d/0051-relationship-workspace.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0051-relationship-workspace"

[[check]]
id = "person-list-filters-by-needs-attention"
invariant = "GET /api/nodes?node_type=person&needs_attention=true returns only people with a linked open/at-risk Obligation."
type = "present"
pattern = "needs_attention"
paths = ["backend/src/graph/node.rs"]

[[check]]
id = "person-list-unchanged-without-filter"
invariant = "Omitting needs_attention preserves the person list route's prior default behavior."
type = "present"
pattern = 'needs_attention: Option<bool>'
paths = ["backend/src/api/nodes.rs"]

[[check]]
id = "person-detail-includes-last-interaction-at"
invariant = "GET /api/nodes/:id (person) includes last_interaction_at derived from matching source_fragments.occurred_at."
type = "present"
pattern = "last_interaction_at"
paths = ["backend/src/api/nodes.rs"]

[[check]]
id = "person-detail-relationship-includes-risk-signals"
invariant = "Each Obligation in a person's relationship grouping includes risk_signals."
type = "present"
pattern = '"risk_signals": risk_signals\('
paths = ["backend/src/api/nodes.rs"]

[[check]]
id = "people-tab-defaults-to-needing-attention"
invariant = "People.tsx defaults to the needs_attention filter with a Show everyone toggle."
type = "present"
pattern = "needsAttentionOnly"
paths = ["frontend/src/components/People.tsx"]

[[check]]
id = "playwright-proves-needs-attention-default-and-toggle"
invariant = "Focused browser coverage proves a bare person is excluded by default and revealed by Show everyone."
type = "present"
pattern = 'ADR-0051'
paths = ["frontend/tests/obligations.spec.ts"]
```

## Notes

Backend: `graph::list_nodes`'s `needs_attention` param adds one `EXISTS`
subquery, a no-op when `false` (verified: a person with only a closed-only
linked Obligation, or none at all, is excluded only when the filter is
on). `GET /api/nodes?node_type=person` is additionally enriched with
batched (not per-row) `open_count`/`at_risk_count`/`last_interaction_at`.
`GET /api/nodes/:id` for a person includes `last_interaction_at` (best-
effort `speaker` string match against `source_fragments`, `null` when
none) and each relationship Obligation carries `risk_signals`, reusing
ADR-0041/0046's function verbatim. Frontend: `People.tsx` defaults to
`needsAttentionOnly = true`, with a "Show everyone" toggle; list cards
show at-risk/open counts and a relative last-interaction phrase; the
person detail view shows the same phrase. `tsc --noEmit` and `vite build`
pass. All backend tests use the unique-node-id-lookup pattern, never an
aggregate count.
