# EV-0081: Add an Actions lens to Graph Explorer's neighbourhood view

Evidence for [ADR-0081](../adr.d/0081-graph-explorer-actions-lens.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0081-graph-explorer-actions-lens"

[[check]]
id = "actions-lens-control-exists"
invariant = "Graph Explorer's node-detail panel offers an All/Actions lens control."
type = "present"
pattern = 'className="lens-select"'
paths = ["frontend/src/components/GraphExplorer.tsx"]

[[check]]
id = "actions-lens-filter-predicate-checks-obligation-and-risk"
invariant = "The lens predicate includes the polymorphic Obligation neighbour shape and node_type === \"risk\" neighbours, nothing else."
type = "present"
pattern = 'target\.type === "obligation"\)[\s\S]*?target\.node_type === "risk"'
paths = ["frontend/src/components/GraphExplorer.tsx"]

[[check]]
id = "shown-filtered-count-is-honest"
invariant = "The relationship-count line states both the shown and filtered-out neighbour counts while the Actions lens is active, never silently dropping neighbours without saying so."
type = "present"
pattern = 'filtered by Actions lens'
paths = ["frontend/src/components/GraphExplorer.tsx"]

[[check]]
id = "playwright-proves-lens-filters-live"
invariant = "Focused browser coverage proves the Actions lens includes a risk neighbour, excludes an unrelated one, states the count honestly, and survives a pivot."
type = "present"
pattern = 'the Actions lens filters neighbours to what needs doing \(ADR-0081\)'
paths = ["frontend/tests/obligations.spec.ts"]

[[check]]
id = "lens-does-not-disturb-trail-or-why-here"
invariant = "Switching the lens does not reset, truncate, or otherwise alter the current traversal trail; pivoting into a neighbour behaves identically regardless of the active lens."
type = "manual"
rationale = "Proven live: ran the Playwright test directly (npx playwright test tests/obligations.spec.ts) and confirmed the trail stayed at 2 items across a pivot into a lens-filtered-visible neighbour and a subsequent lens switch back to All -- a cross-cutting behavioral claim spanning both ADR-0033's pivot function and this ADR's filter, better proven live than by a fragile multi-line regex."
last_verified = "2026-08-18"

[[check]]
id = "no-new-backend-route-or-query"
invariant = "This lens is a pure client-side filter over the existing GET /api/nodes/:id response; no new backend route, query parameter, or schema change is introduced."
type = "manual"
rationale = "A negative claim (no new backend surface) is not reliably provable by a positive regex match; verified by direct review of the implementing diff (GraphExplorer.tsx and the test file only; no changes to backend/src/api/mod.rs or frontend/src/api.ts), matching EV-0033's own precedent for the same kind of claim."
last_verified = "2026-08-18"
```
