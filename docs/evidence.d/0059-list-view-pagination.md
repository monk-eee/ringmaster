# EV-0059: List-view pagination for Obligations, Candidates, and People

Evidence for [ADR-0059](../adr.d/0059-list-view-pagination.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0059-list-view-pagination"

[[check]]
id = "obligations-route-accepts-limit-and-offset"
invariant = "GET /api/obligations accepts limit/offset and applies them as SQL LIMIT/OFFSET, clamped rather than rejected."
type = "present"
pattern = 'fn list_obligations\(State\(pool\): State<PgPool>, Query\(params\): Query<ListQuery>\)'
paths = ["backend/src/api.rs"]

[[check]]
id = "candidates-route-accepts-limit-and-offset"
invariant = "GET /api/candidates accepts limit/offset and applies them as SQL LIMIT/OFFSET, clamped rather than rejected."
type = "present"
pattern = 'fn list_candidates\(State\(pool\): State<PgPool>, Query\(params\): Query<ListQuery>\)'
paths = ["backend/src/api.rs"]

[[check]]
id = "nodes-route-limit-offset-is-additive"
invariant = "GET /api/nodes accepts limit/offset; omitting both preserves exact current behavior."
type = "present"
pattern = 'const MAX_LIST_LIMIT: i64 = 200;'
paths = ["backend/src/api.rs"]

[[check]]
id = "list-views-offer-load-more"
invariant = "Obligations, Candidates/Inbox, and People each show a Load more affordance."
type = "present"
pattern = 'onClick=\{(on)?[Ll]oadMore\}'
paths = ["frontend/src/components/ObligationsTable.tsx", "frontend/src/components/CandidatesTable.tsx", "frontend/src/components/People.tsx"]

[[check]]
id = "playwright-proves-load-more"
invariant = "A Playwright test proves Load more appends a further page of results."
type = "present"
pattern = 'test\("people tab: Load more appends a further page \(ADR-0059\)"'
paths = ["frontend/tests/obligations.spec.ts"]
```

## Notes

Implemented: all three list routes (`/api/obligations`, `/api/candidates`,
`/api/nodes`) share a `clamp_list_params` helper clamping `limit` to
`[1, MAX_LIST_LIMIT=200]` rather than rejecting an out-of-range value
(matching ADR-0049's own audit-limit precedent); omitting both params
preserves each route's exact prior behavior (no `LIMIT`/`OFFSET` clause).
`ObligationsTable`/`CandidatesTable`/`People` each render a "Load more"
button when `hasMore` is true, fetching and appending the next page.

