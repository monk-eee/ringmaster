# EV-0071: Surface recent interaction sources on Person detail

Evidence for [ADR-0071](../adr.d/0071-person-detail-recent-interactions.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0071-person-detail-recent-interactions"

[[check]]
id = "person-detail-returns-recent-interactions"
invariant = "Person detail returns deduplicated interaction sources newest-first across participated_in and legacy speaker paths."
type = "present"
pattern = 'fn person_detail_returns_recent_interactions_newest_first_across_both_paths'
paths = ["backend/src/api/nodes.rs"]

[[check]]
id = "recent-interactions-deduplicate-with-edge-precedence"
invariant = "When both evidence paths identify one source, Person detail returns one interaction with participated_in evidence precedence."
type = "present"
pattern = 'fn recent_interactions_deduplicate_a_source_with_edge_precedence'
paths = ["backend/src/api/nodes.rs"]

[[check]]
id = "recent-interactions-cap-and-total"
invariant = "Person detail returns at most 10 recent interactions and the honest total deduplicated count."
type = "present"
pattern = 'fn recent_interactions_are_capped_at_ten_with_an_honest_total'
paths = ["backend/src/api/nodes.rs"]

[[check]]
id = "people-ui-renders-recent-interactions"
invariant = "People detail renders source title, type, and date with honest empty and capped states, never raw ids or generated summaries."
type = "present"
pattern = 'recent-interactions-heading[\s\S]*No recorded interactions\.[\s\S]*interaction\.title[\s\S]*interaction\.source_type[\s\S]*toLocaleDateString\(\)[\s\S]*Showing the latest'
paths = ["frontend/src/components/People.tsx"]

[[check]]
id = "people-browser-covers-recent-interaction-states"
invariant = "Focused browser coverage owns deterministic fixtures for populated, empty, and capped recent-interaction states."
type = "present"
pattern = 'people tab opens a person into relationship and populated interaction detail[\s\S]*No recorded interactions\.[\s\S]*people detail reports a capped recent-interactions list honestly[\s\S]*toHaveCount\(10\)[\s\S]*Showing the latest 10 of 12\.'
paths = ["frontend/tests/obligations.spec.ts"]

[[check]]
id = "recent-interactions-align-with-person-detail"
invariant = "Recent interactions uses the same horizontal inset as the surrounding Person-detail content."
type = "present"
pattern = '\.recent-interactions\s*\{[\s\S]*?padding: 0 1\.25rem;'
paths = ["frontend/public/style.css"]

[[check]]
id = "recent-interactions-tests-pass"
invariant = "Backend and focused browser tests pass for recent Person interactions."
type = "manual"
last_verified = "2026-08-18"
rationale = "Backend: the full backend suite previously passed through Unit Test MCP against ringmaster_test with all four recent_interactions cases. Frontend: after the review fixes, `npx playwright test tests/obligations.spec.ts --grep \"People tab\" --reporter=list` passed 4/4. The browser group now creates and selects exact Person fixtures, proves a real ingested interaction's title/type/localized date while hiding raw provenance, proves the empty state on a known bare Person, and proves a 10-of-12 capped state."
```

## Notes

Implemented across `backend/src/api.rs` (a single CTE query on `get_node_detail`
unioning `participated_in` edges and legacy speaker matches, deduplicated by
source id with edge precedence, capped at 10 with an honest total; non-person
nodes get an empty collection) and `frontend/src/components/People.tsx` (a
"Recent interactions" section with title/type/date, an honest empty state,
and a "Showing the latest N of M" cap notice, never a raw id or evidence-mode
label). `frontend/src/api.ts` carries the additive `RecentInteraction`/
`NodeDetail` typing. Regression coverage: four backend tests plus the
four-test "People tab" Playwright group, which owns deterministic populated,
empty, and capped fixtures rather than depending on ambient database ordering.