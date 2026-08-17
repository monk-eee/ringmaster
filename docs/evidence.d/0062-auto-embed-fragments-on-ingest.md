# EV-0062: Auto-embed fragments on ingest (best-effort)

Evidence for [ADR-0062](../adr.d/0062-auto-embed-fragments-on-ingest.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0062-auto-embed-fragments-on-ingest"

[[check]]
id = "ingest-triggers-best-effort-embedding"
invariant = "Ingest calls a best-effort embedding helper after the commit."
type = "present"
pattern = "embed_fragments_best_effort"
paths = ["backend/src/transcript.rs"]

[[check]]
id = "auto-embed-is-tested"
invariant = "A test covers auto-embedding on ingest."
type = "present"
pattern = "ingest_auto_embeds_fragments_when_a_model_is_configured"
paths = ["backend/src/transcript.rs"]

[[check]]
id = "backend-suite-passes-and-ingest-stays-non-blocking"
invariant = "The full backend suite passes; ingest stays non-blocking when embedding is unconfigured or fails."
type = "manual"
last_verified = "2026-08-17"
rationale = "Not a file-content regex. Verified directly: ran the full backend suite via podman against ringmaster_test with --test-threads=1; all tests passed, including ingest_auto_embeds_fragments_when_a_model_is_configured. Confirmed non-blocking both ways: with RINGMASTER_EMBEDDING_URL unset the ingest tests still pass (helper is a no-op and ingest returns Ok), and against the live dev model a fresh ingest produced embeddings rows for its fragments (search populated) while ingest never failed."
```

## Notes

This ADR amends [ADR-0010](../adr.d/0010-transcript-ingestion-parsing-chunking-provenance.md)
and [ADR-0018](../adr.d/0018-generate-and-store-source-fragment-embeddings.md):
0010/0040 stated ingestion never generates embeddings and 0018 made embedding a
manual-only step. This record makes embedding automatic on ingest but keeps it
best-effort and outside the ingest transaction, so
[ADR-0018](../adr.d/0018-generate-and-store-source-fragment-embeddings.md)'s
"never block ingestion when unconfigured" guarantee is preserved.
