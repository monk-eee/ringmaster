# EV-0066: Non-destructive graph management over MCP

Evidence for
[ADR-0066](../adr.d/0066-non-destructive-graph-management-over-mcp.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0066-non-destructive-graph-management-over-mcp"

[[check]]
id = "entity-management-tools-exposed"
invariant = "MCP exposes generic entity list, get, create, update, and batch-upsert tools."
type = "present"
pattern = "(?=[\\s\\S]*async fn list_entities)(?=[\\s\\S]*async fn get_entity)(?=[\\s\\S]*async fn create_entity)(?=[\\s\\S]*async fn update_entity)(?=[\\s\\S]*async fn upsert_entities)"
paths = ["backend/src/bin/ringmaster-ingest/mcp.rs"]

[[check]]
id = "atomic-exact-entity-upsert"
invariant = "Batch upsert is exact-match, shallow-merge, ambiguity-safe, and atomic."
type = "manual"
last_verified = "2026-08-17"
rationale = "Two database-backed integration tests passed through Unit Test MCP against ringmaster_test. One proves whitespace-trimmed exact-match shallow merge, retaining existing attributes while updating one entity and creating another. The other proves two exact matches produce an ambiguity and roll back an earlier valid create in the same batch. The full backend suite passed in the same run."

[[check]]
id = "relationship-management-tools-exposed"
invariant = "MCP exposes relationship list and create tools with temporal options."
type = "present"
pattern = "(?=[\\s\\S]*async fn list_relationships)(?=[\\s\\S]*async fn create_relationship)"
paths = ["backend/src/bin/ringmaster-ingest/mcp.rs"]

[[check]]
id = "graph-tools-live-mcp"
invariant = "All seven graph tools are listed and representative calls work over a live MCP stdio handshake."
type = "manual"
last_verified = "2026-08-17"
rationale = "A live stdio handshake negotiated MCP protocol 2024-11-05 and listed 10 tools total: the three existing source/search tools plus all seven graph tools. Representative calls for every graph tool succeeded against ringmaster_test; update and upsert retained pre-existing attributes, upsert returned updated/created, and relationship listing returned the created edge. Cleanup removed the one marker edge and three marker nodes."
```
