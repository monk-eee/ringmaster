# EV-0028: Person relationship view — resolve linked Obligations into a per-person page

Evidence for [ADR-0028](../adr.d/0028-person-relationship-view.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0028-person-relationship-view"

[[check]]
id = "obligation-neighbor-resolves"
invariant = "GET /api/nodes/:id resolves an Obligation-typed edge target's status/dates/reason instead of returning a null neighbor."
type = "present"
pattern = '"type": "obligation"'
paths = ["backend/src/api.rs"]

[[check]]
id = "unknown-neighbor-still-null"
invariant = "A genuinely unknown neighbor id (neither nodes nor obligation_projection) still reports null."
type = "present"
pattern = 'node_detail_includes_neighbor_summary_and_handles_a_non_node_edge_target'
paths = ["backend/src/api.rs"]

[[check]]
id = "person-relationship-grouping"
invariant = "A person node's response includes an at_risk/open grouped relationship object; other node types omit it."
type = "present"
pattern = 'node_detail_omits_relationship_grouping_for_non_person_nodes'
paths = ["backend/src/api.rs"]

[[check]]
id = "relationship-view-component-exists"
invariant = "The Graph Explorer's detail panel renders a Relationship view for person nodes."
type = "present"
pattern = 'renderRelationshipGroup'
paths = ["frontend/src/components/GraphExplorer.tsx"]
```

## Notes

All four checks are automated against the implementing route/component.
`cargo test` covers: an edge into a real, linked Obligation resolving with
its real status/dates/reason (`node_detail_resolves_a_real_linked_obligation_with_status_and_reason`);
a genuinely unknown id still reporting null, unchanged from ADR-0025
(`node_detail_includes_neighbor_summary_and_handles_a_non_node_edge_target`);
and a non-person node never getting a `relationship` field
(`node_detail_omits_relationship_grouping_for_non_person_nodes`). Verified
live: created a real person node and linked it to a real at-risk Obligation
via `POST /api/edges`, confirmed `GET /api/nodes/:id` returned the resolved
neighbor and the `relationship.at_risk` group, and confirmed in the browser
that the Graph tab's detail panel renders the "Relationship" section with
the correct status badge and reason text above the existing SVG view.
