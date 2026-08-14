# EV-0033: Progressive graph traversal trail over one-hop neighborhoods

Evidence for [ADR-0033](../adr.d/0033-progressive-graph-traversal-trail.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0033-progressive-graph-traversal-trail"

[[check]]
id = "trail-starts-and-appends"
invariant = "Selecting a list node starts a trail and clicking a neighbour appends its node and connecting relationship."
type = "present"
pattern = 'function selectRootNode|function visitNeighbor'
paths = ["frontend/src/components/GraphExplorer.tsx"]

[[check]]
id = "trail-renders-readable-path"
invariant = "The trail visibly renders human-readable nodes and relationship labels."
type = "present"
pattern = 'graph-trail-path'
paths = ["frontend/src/components/GraphExplorer.tsx"]

[[check]]
id = "trail-navigation-is-reversible"
invariant = "Back and earlier-step selection truncate the trail and restore that node as current focus."
type = "present"
pattern = 'function jumpToTrailStep|function goBack'
paths = ["frontend/src/components/GraphExplorer.tsx"]

[[check]]
id = "focused-node-explains-why-here"
invariant = "A non-root focus explains its immediate path through a deterministic Why here line, including the traversed edge's historical/suggested trust state as text, not colour alone."
type = "present"
pattern = 'Why here: connected to'
paths = ["frontend/src/components/GraphExplorer.tsx"]

[[check]]
id = "one-hop-api-remains-boundary"
invariant = "Traversal composes repeated calls to the existing one-hop GET /api/nodes/:id; no backend route, schema, or frontend dependency was added."
type = "manual"
rationale = "A negative claim (no new route/dependency) is not reliably provable by a positive regex match; verified by direct review of this ADR's diff (frontend-only, no new imports in package.json) and of backend/src/api.rs (no new route registered)."

[[check]]
id = "playwright-proves-multi-step-traversal"
invariant = "Focused browser coverage proves a user can traverse at least two edges and return to the root."
type = "present"
pattern = 'graph trail: traversing two edges and returning to the root'
paths = ["frontend/tests/obligations.spec.ts"]
```

## Notes

All checks but one are automated against the implementing component/test.
Verified live: created three fresh nodes and two edges (one confidence-
bearing and later superseded, to exercise both trust states at once),
traversed A → B in the browser, and confirmed the "Why here" line read
exactly `connected to Trust Test A via "maybe_attended" (historical, until
3/1/2026; suggested, 55% confidence)` with the verb rendered in the
at-risk color with a dotted underline -- text and visual treatment
together, never color alone. `one-hop-api-remains-boundary` stays a
reasoned `manual` check (a negative claim about absent new routes/
dependencies is not soundly provable by a positive regex), matching
EV-0021's own precedent for the same kind of claim.

