# EV-0064: A semantic `search` MCP tool

Evidence for [ADR-0064](../adr.d/0064-semantic-search-mcp-tool.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0064-semantic-search-mcp-tool"

[[check]]
id = "search-mcp-tool-exists"
invariant = "A search MCP tool delegates to graph::search_source_fragments."
type = "present"
pattern = "search_source_fragments"
paths = ["backend/src/bin/ringmaster-ingest/mcp.rs"]

[[check]]
id = "search-tool-handles-unconfigured-model"
invariant = "The search tool errors legibly (not empty) when no embedding model is configured."
type = "present"
pattern = "EmbeddingConfig::from_env"
paths = ["backend/src/bin/ringmaster-ingest/mcp.rs"]

[[check]]
id = "search-tool-listed-and-callable-over-mcp"
invariant = "The search tool is listed and callable over a live MCP stdio handshake."
type = "manual"
last_verified = "2026-08-17"
rationale = "Not a file-content regex. Verified directly: built the ringmaster-ingest binary, ran it as `mcp-serve` with DATABASE_URL and the embedding model configured, and drove a raw JSON-RPC stdio handshake (initialize -> notifications/initialized -> tools/list -> tools/call). tools/list included `search`; tools/call with a query returned ranked source-fragment hits with similarity scores. The backend suite also passes (graph::search_source_fragments is already covered by ADR-0019's tests)."
```

## Notes

This tool adds no search logic: it surfaces
[ADR-0019](../adr.d/0019-semantic-search-over-source-fragments.md)'s existing
`graph::search_source_fragments` over the MCP server
([ADR-0040](../adr.d/0040-dated-source-ingestion.md)), now that
[ADR-0062](../adr.d/0062-auto-embed-fragments-on-ingest.md)/[ADR-0063](../adr.d/0063-reindex-backfill-embeddings.md)
make embeddings exist for the corpus.
