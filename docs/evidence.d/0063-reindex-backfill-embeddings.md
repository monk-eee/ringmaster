# EV-0063: A reindex command to backfill embeddings for existing fragments

Evidence for [ADR-0063](../adr.d/0063-reindex-backfill-embeddings.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0063-reindex-backfill-embeddings"

[[check]]
id = "reindex-function-exists"
invariant = "A library function backfills embeddings for fragments with no embedding yet."
type = "present"
pattern = "reindex_unembedded_fragments"
paths = ["backend/src/transcript.rs"]

[[check]]
id = "cli-exposes-reindex-embeddings"
invariant = "The ringmaster-ingest CLI exposes a reindex-embeddings subcommand."
type = "present"
pattern = "reindex-embeddings"
paths = ["backend/src/bin/ringmaster-ingest/main.rs"]

[[check]]
id = "reindex-is-tested-and-suite-passes"
invariant = "A test proves an unembedded fragment becomes embedded, and the full suite passes."
type = "manual"
last_verified = "2026-08-17"
rationale = "Not a file-content regex. Verified directly: ran the full backend suite via podman against ringmaster_test with the embedding model reachable and --test-threads=1; all tests passed, including reindex_embeds_previously_unembedded_fragments_when_a_model_is_configured, which creates a fragment with no embedding, runs reindex_unembedded_fragments, and asserts that fragment then has an embedding row. Also ran the CLI reindex-embeddings subcommand live against the dev database and observed the unembedded count drop to zero with search then returning results for previously-unsearchable fragments."
```

## Notes

This ADR completes [ADR-0062](../adr.d/0062-auto-embed-fragments-on-ingest.md),
which auto-embeds new ingests but deferred back-filling fragments stored before
that change. The backfill only appends to the `embeddings` table and never
touches an immutable `source_fragment`
([ADR-0010](../adr.d/0010-transcript-ingestion-parsing-chunking-provenance.md)).
