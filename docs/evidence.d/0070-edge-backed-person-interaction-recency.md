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
paths = ["backend/src/api.rs"]

[[check]]
id = "person-list-batches-edge-backed-interactions"
invariant = "The People list derives interaction dates for all returned people in one batched edge-plus-fallback query."
type = "present"
pattern = "edge_interactions: Vec<EdgeInteractionRow>"
paths = ["backend/src/api.rs"]

[[check]]
id = "legacy-speaker-fallback-preserved"
invariant = "A source with only an exact legacy speaker match still contributes to Person interaction recency."
type = "present"
pattern = 'fn person_list_uses_legacy_speaker_fallback_with_no_participation_edge'
paths = ["backend/src/api.rs"]

[[check]]
id = "newest-interaction-wins-across-paths"
invariant = "The newest occurred_at value wins when both edge-backed and legacy speaker evidence exist."
type = "present"
pattern = 'fn newest_interaction_wins_between_edge_and_legacy_paths'
paths = ["backend/src/api.rs"]

[[check]]
id = "backend-suite-passes-with-edge-backed-recency"
invariant = "The backend suite passes with edge-backed Person interaction recency."
type = "manual"
last_verified = "2026-08-17"
rationale = "A live test run is not a file-content regex. Ran the full backend suite via the Unit Test MCP custom command (cargo test -- --test-threads=1) against ringmaster_test with the edge-backed derivation in place, including person_detail_uses_participation_edge_for_last_interaction_at, person_list_uses_participation_edge_for_last_interaction_at, person_list_uses_legacy_speaker_fallback_with_no_participation_edge, and newest_interaction_wins_between_edge_and_legacy_paths; two consecutive runs reported PASSED with zero failures."
```

## Notes

Implemented in `backend/src/api.rs`: `list_nodes_route` (the People list) adds
a third batched query, `edge_interactions`, joining `participated_in` edges
to their target source nodes' `occurred_at` and grouping by person id; the
exposed `last_interaction_at` is the max of that edge-backed value and the
existing legacy speaker-match value. `get_node_detail` mirrors this with a
single `GREATEST(edge_path.last_interaction_at, legacy_path.last_interaction_at)`
query over two subqueries. Both routes keep exactly one query per evidence
path (no per-row queries). Four new tests cover: edge-only derivation on
detail and on the list route, legacy-only fallback on the list route
(reusing the existing detail-side coverage), and that the newer of the two
paths wins on both routes when both exist.

