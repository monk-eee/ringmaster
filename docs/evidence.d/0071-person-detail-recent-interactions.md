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
paths = ["backend/src/api.rs"]

[[check]]
id = "recent-interactions-deduplicate-with-edge-precedence"
invariant = "When both evidence paths identify one source, Person detail returns one interaction with participated_in evidence precedence."
type = "present"
pattern = 'fn recent_interactions_deduplicate_a_source_with_edge_precedence'
paths = ["backend/src/api.rs"]

[[check]]
id = "recent-interactions-cap-and-total"
invariant = "Person detail returns at most 10 recent interactions and the honest total deduplicated count."
type = "present"
pattern = 'fn recent_interactions_are_capped_at_ten_with_an_honest_total'
paths = ["backend/src/api.rs"]

[[check]]
id = "people-ui-renders-recent-interactions"
invariant = "People detail renders source title, type, and date with honest empty and capped states, never raw ids or generated summaries."
type = "present"
pattern = 'recent-interactions-heading'
paths = ["frontend/src/components/People.tsx"]

[[check]]
id = "recent-interactions-tests-pass"
invariant = "Backend and focused browser tests pass for recent Person interactions."
type = "manual"
last_verified = "2026-08-18"
rationale = "Backend: ran the full backend suite via the Unit Test MCP custom command (cargo test -- --test-threads=1) against ringmaster_test with recent_interactions in place, including person_detail_returns_recent_interactions_newest_first_across_both_paths, recent_interactions_deduplicate_a_source_with_edge_precedence, recent_interactions_are_capped_at_ten_with_an_honest_total, and recent_interactions_are_empty_for_non_person_nodes; PASSED with zero failures. Frontend: the ringmaster-backend-1 container was 14 hours stale and still returned pre-ADR-0071 responses (no recent_interactions field), which failed the Playwright suite; rebuilt via `docker compose build backend && docker compose up -d --force-recreate backend`, then `npx playwright test -g \"People tab\"` (chromium) passed 3/3, proving the heading, empty state, and detail flow render against a live, current backend."
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
existing "People tab" Playwright test, extended to assert the "Recent
interactions" heading and empty state render.