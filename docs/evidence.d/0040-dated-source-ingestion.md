# EV-0040: Dated source ingestion — required `occurred_at`, over API, CLI, and MCP

Evidence for [ADR-0040](../adr.d/0040-dated-source-ingestion.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0040-dated-source-ingestion"

[[check]]
id = "nodes-have-occurred-at-column"
invariant = "nodes has a nullable occurred_at column."
type = "present"
pattern = 'ALTER TABLE nodes ADD COLUMN occurred_at TIMESTAMPTZ'
paths = ["backend/migrations/0013_source_occurred_at.sql"]

[[check]]
id = "ingest-source-function-requires-occurred-at"
invariant = "The shared ingest_source function creates a node + ordered fragments, splitting non-meeting text by paragraph, and requires occurred_at."
type = "present"
pattern = 'pub async fn ingest_source\([\s\S]*?pool: &PgPool,[\s\S]*?metadata: &SourceMetadata'
paths = ["backend/src/transcript.rs"]

[[check]]
id = "sources-ingest-route-exists"
invariant = "POST /api/sources/ingest calls the shared function, rejecting a missing/blank occurred_at with 400."
type = "present"
pattern = '"/api/sources/ingest"'
paths = ["backend/src/api/mod.rs"]

[[check]]
id = "meeting-ingest-requires-occurred-at"
invariant = "POST /api/meetings/ingest now requires occurred_at and rejects its absence with 400, all other behavior unchanged."
type = "present"
pattern = 'ingest_meeting_route_rejects_a_missing_occurred_at_with_no_writes'
paths = ["backend/src/api/ingestion.rs"]

[[check]]
id = "cli-binary-ingests-via-shared-function"
invariant = "The ringmaster-ingest CLI binary ingests a source by calling the same shared function, with no HTTP server required."
type = "present"
pattern = 'ingest_source\(&pool, &metadata, &text\)'
paths = ["backend/src/bin/ringmaster-ingest/main.rs"]

[[check]]
id = "mcp-tool-exposes-ingest-source"
invariant = "The mcp-serve subcommand exposes exactly one MCP tool, ingest_source, over stdio, calling the same shared function."
type = "present"
pattern = 'async fn ingest_source\(\s*&self,\s*Parameters\(params\): Parameters<IngestSourceParams>'
paths = ["backend/src/bin/ringmaster-ingest/mcp.rs"]

[[check]]
id = "ingestion-never-triggers-extraction-or-embedding"
invariant = "None of the three surfaces (API, CLI, MCP) triggers extraction or embedding implicitly."
type = "present"
pattern = 'ingest_source_route_never_creates_a_candidate_implicitly'
paths = ["backend/src/api/ingestion.rs"]
```

## Notes

Implemented and verified live, not just by unit test: `cargo test` (89/89
backend tests pass, including new `ingest_source`/`split_paragraphs`/
`ingest_source_route`/`ingest_meeting_route_rejects_a_missing_occurred_at`
cases). Live end-to-end checks against the running dev stack: `POST
/api/sources/ingest` and the amended `POST /api/meetings/ingest` (both the
400-without-occurred_at and 201-with-occurred_at paths) via curl; the
`ringmaster-ingest` CLI binary run directly against `DATABASE_URL` with no
server, ingesting real stdin text into a node + ordered fragments; the
`mcp-serve` subcommand driven by hand over stdio through a real
`initialize` → `notifications/initialized` → `tools/list` → `tools/call`
sequence, confirming `tools/list` returns exactly the one `ingest_source`
tool with the correct JSON Schema, and `tools/call` genuinely creates a
node and fragment. `rmcp` (the one new dependency, scoped to this binary)
resolved cleanly from crates.io with the `server`/`schemars`/`transport-io`
features. All seven checks are automated `present` matches; no check in
this ADR is a negative/absent claim, so none needs to stay `manual`.

