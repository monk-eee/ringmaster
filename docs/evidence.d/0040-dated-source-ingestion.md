# EV-0040: Dated source ingestion — required `occurred_at`, over API, CLI, and MCP

Evidence for [ADR-0040](../adr.d/0040-dated-source-ingestion.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0040-dated-source-ingestion"

[[check]]
id = "nodes-have-occurred-at-column"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once a migration adds a nullable occurred_at TIMESTAMPTZ column to nodes."

[[check]]
id = "ingest-source-function-requires-occurred-at"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once the shared ingest_source function (transcript.rs) creates a node + ordered fragments, splits non-meeting text by paragraph, and rejects a missing occurred_at."

[[check]]
id = "sources-ingest-route-exists"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once POST /api/sources/ingest calls the shared ingest_source function and rejects a missing/blank occurred_at with 400."

[[check]]
id = "meeting-ingest-requires-occurred-at"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once POST /api/meetings/ingest requires occurred_at and rejects its absence with 400, with every other field/response shape unchanged."

[[check]]
id = "cli-binary-ingests-via-shared-function"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once the ringmaster-ingest CLI binary (backend/src/bin/) connects directly to DATABASE_URL and calls ingest_source, with no HTTP server required."

[[check]]
id = "mcp-tool-exposes-ingest-source"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once the mcp-serve subcommand starts a stdio MCP server (rmcp) exposing exactly one tool, ingest_source, which calls the shared ingest_source function."

[[check]]
id = "ingestion-never-triggers-extraction-or-embedding"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once none of the three surfaces (API, CLI, MCP) is confirmed to call extraction or embedding implicitly."
```

## Notes

Pre-implementation: all seven checks are deliberately `manual`/unproven,
per this repo's own convention (evidence stays honest about intent vs.
proof until the ADR is accepted and implemented). Do not implement before
[ADR-0040](../adr.d/0040-dated-source-ingestion.md)'s Status flips to
Accepted.

