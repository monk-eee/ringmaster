# EV-0047: Obligation detail page — a first-class read view over existing data

Evidence for [ADR-0047](../adr.d/0047-obligation-detail-page.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0047-obligation-detail-page"

[[check]]
id = "obligation-detail-route-returns-risk-signals"
invariant = "GET /api/obligations/:id returns the same fields Daily Brief returns, plus risk_signals."
type = "present"
pattern = 'async fn get_obligation_detail'
paths = ["backend/src/api/obligations.rs"]

[[check]]
id = "obligation-detail-route-resolves-linked-nodes"
invariant = "The route resolves linked edges against nodes, honestly reporting null for an unresolvable neighbor."
type = "present"
pattern = 'linked_nodes'
paths = ["backend/src/api/obligations.rs"]

[[check]]
id = "obligation-detail-route-404s-for-unknown-id"
invariant = "The route 404s for an unknown id."
type = "present"
pattern = 'obligation_detail_route_404s_for_an_unknown_id'
paths = ["backend/src/api/obligations.rs"]

[[check]]
id = "title-and-due-phrase-are-exported-and-reused"
invariant = "itemTitle/duePhrase are exported and reused, not duplicated."
type = "present"
pattern = 'export function itemTitle'
paths = ["frontend/src/components/DailyBrief.tsx"]

[[check]]
id = "rows-are-selectable-into-shared-detail-view"
invariant = "Selecting a Today row or an Obligations-table row opens the shared detail view with a Back control."
type = "present"
pattern = 'ObligationDetail'
paths = ["frontend/src/App.tsx"]

[[check]]
id = "playwright-proves-obligation-detail-flow"
invariant = "Focused browser coverage proves opening an Obligation's detail and returning."
type = "present"
pattern = 'obligation detail:'
paths = ["frontend/tests/obligations.spec.ts"]
```

## Notes

All six checks are automated against the implementing route/component/test.
