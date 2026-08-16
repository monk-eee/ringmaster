# EV-0042: Surface `occurred_at` on nodes, with date-range retrieval and a second MCP tool

Evidence for [ADR-0042](../adr.d/0042-occurred-at-retrieval-and-recall-sources-mcp-tool.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0042-occurred-at-retrieval-and-recall-sources-mcp-tool"

[[check]]
id = "node-responses-include-occurred-at"
invariant = "Node carries occurred_at, selected by get_node and list_nodes, so every existing response serializing a Node includes it."
type = "present"
pattern = 'pub occurred_at: Option<chrono::DateTime<chrono::Utc>>,'
paths = ["backend/src/graph.rs"]

[[check]]
id = "nodes-route-filters-by-occurred-at-range"
invariant = "GET /api/nodes filters by occurred_from/occurred_to and rejects an unparseable bound with 400."
type = "present"
pattern = 'nodes_route_filters_by_occurred_at_range_and_rejects_an_unparseable_bound'
paths = ["backend/src/api.rs"]

[[check]]
id = "nodes-route-unchanged-without-date-params"
invariant = "Omitting occurred_from/occurred_to preserves GET /api/nodes's prior response and order exactly."
type = "manual"
rationale = "A negative/unchanged-behavior claim is not reliably provable by a positive regex match; verified by direct test run: node_create_list_enrich_and_detail_round_trip (GET /api/nodes?node_type=person, no date params) passes unchanged, matching EV-0039's own precedent for this kind of claim."
last_verified = "2026-08-17"

[[check]]
id = "mcp-exposes-recall-sources-tool"
invariant = "The ringmaster-ingest MCP server exposes a second tool, recall_sources, filtering by the same range/type without requiring an embedding model."
type = "present"
pattern = 'async fn recall_sources\(&self, Parameters\(params\): Parameters<RecallSourcesParams>\)'
paths = ["backend/src/bin/ringmaster-ingest/mcp.rs"]
```

## Notes

Implemented: `Node.occurred_at` selected in `get_node`/`list_nodes`/
`update_node`'s `RETURNING`, so `GET /api/nodes`, `GET /api/nodes/:id`, and
`PATCH /api/nodes/:id` all surface it with no route changes needed for
those beyond `/api/nodes`. `list_nodes` gained `occurred_from`/
`occurred_to` (`DateTime<Utc>`) params, filtered via a NULL-coalescing
`WHERE` (one query, not a branch per combination), ordering by
`occurred_at DESC NULLS LAST` when either bound is given. `GET /api/nodes`
gained the matching `?occurred_from=`/`?occurred_to=` query params,
RFC3339, `400` on an unparseable value. The `ringmaster-ingest` MCP server
(`mcp.rs`) gained `recall_sources`, sharing the same `list_nodes` call.

Directly verified end-to-end (not just via the automated checks above): a
raw `initialize` + `tools/list` MCP handshake against the built
`ringmaster-ingest mcp-serve` binary confirmed both `ingest_source` and
`recall_sources` are registered with correct auto-derived JSON schemas;
a `tools/call` for `recall_sources` against the real dev database returned
actual node rows with a populated top-level `occurred_at` field.
