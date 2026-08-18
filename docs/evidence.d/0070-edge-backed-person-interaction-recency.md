# EV-0070: Edge-backed Person interaction recency

Evidence for
[ADR-0070](../adr.d/0070-edge-backed-person-interaction-recency.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0070-edge-backed-person-interaction-recency"

[[check]]
id = "person-detail-uses-participation-edge"
invariant = "Person detail derives last_interaction_at from a participated_in source even when no speaker string matches."
type = "present"
pattern = 'fn person_detail_uses_participation_edge_for_last_interaction_at'
paths = ["backend/src/api/nodes.rs"]

[[check]]
id = "person-list-batches-edge-backed-interactions"
invariant = "The People list derives interaction dates for all returned people in one batched edge-plus-fallback query."
type = "present"
pattern = 'let interactions: Vec<InteractionRow> = sqlx::query_as\([\s\S]*?UNION ALL[\s\S]*?GROUP BY evidence\.person_id'
paths = ["backend/src/api/nodes.rs"]

[[check]]
id = "person-list-has-no-second-interaction-query"
invariant = "The People list does not retain a second edge_interactions query alongside the combined batched query."
type = "absent"
pattern = "EdgeInteractionRow|edge_interactions"
paths = ["backend/src/api/nodes.rs"]

[[check]]
id = "legacy-speaker-fallback-preserved"
invariant = "A source with only an exact legacy speaker match still contributes to Person interaction recency."
type = "present"
pattern = 'fn person_list_uses_legacy_speaker_fallback_with_no_participation_edge'
paths = ["backend/src/api/nodes.rs"]

[[check]]
id = "newest-interaction-wins-across-paths"
invariant = "The newest occurred_at value wins when both edge-backed and legacy speaker evidence exist."
type = "present"
pattern = 'fn newest_interaction_wins_between_edge_and_legacy_paths[\s\S]*?newer legacy-path date must win[\s\S]*?newer edge-path date must win'
paths = ["backend/src/api/nodes.rs"]

[[check]]
id = "backend-suite-passes-with-edge-backed-recency"
invariant = "The backend suite passes with edge-backed Person interaction recency."
type = "manual"
last_verified = "2026-08-17"
rationale = "After the single-query conformance refactor, the full backend suite passed through Unit Test MCP against ringmaster_test, and the focused newest_interaction_wins_between_edge_and_legacy_paths run also passed. The custom Rust runner reports process status but does not emit parsed test counts or a coverage artifact."
```

## Notes

Implemented in `backend/src/api.rs`: `list_nodes_route` (the People list) uses
one batched `UNION ALL` aggregate query for both `participated_in` edge-backed
dates and legacy speaker-match dates, grouped by person id; no per-row or
second interaction query remains. `get_node_detail` uses one bounded
`GREATEST(edge_path.last_interaction_at, legacy_path.last_interaction_at)`
query over two subqueries. Four tests cover edge-only derivation on detail and
on the list route, legacy-only fallback on the list route (reusing existing
detail-side coverage), and both newest-wins orderings on both routes.

