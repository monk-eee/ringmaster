# EV-0042: Surface `occurred_at` on nodes, with date-range retrieval and a second MCP tool

Evidence for [ADR-0042](../adr.d/0042-occurred-at-retrieval-and-recall-sources-mcp-tool.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0042-occurred-at-retrieval-and-recall-sources-mcp-tool"

[[check]]
id = "node-responses-include-occurred-at"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once graph::Node has an occurred_at field, selected by get_node and list_nodes."

[[check]]
id = "nodes-route-filters-by-occurred-at-range"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once GET /api/nodes accepts occurred_from/occurred_to and rejects an unparseable bound with 400."

[[check]]
id = "nodes-route-unchanged-without-date-params"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "A negative/unchanged-behavior claim; verified by direct review/test once implemented, matching EV-0039's own precedent for this kind of claim."

[[check]]
id = "mcp-exposes-recall-sources-tool"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once the ringmaster-ingest MCP server (mcp.rs) exposes a second tool, recall_sources, calling the extended list_nodes function."
```

## Notes

Pre-implementation: all four checks are deliberately `manual`/unproven, per
this repo's own convention (evidence stays honest about intent vs. proof
until the ADR is accepted and implemented). Do not implement before
[ADR-0042](../adr.d/0042-occurred-at-retrieval-and-recall-sources-mcp-tool.md)'s
Status flips to Accepted.
