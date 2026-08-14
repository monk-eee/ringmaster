# EV-0018: Generate and store embeddings for source fragments

Evidence for [ADR-0018](../adr.d/0018-generate-and-store-source-fragment-embeddings.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0018-generate-and-store-source-fragment-embeddings"

[[check]]
id = "embedding-column-has-fixed-dimension"
invariant = "embeddings.embedding is a fixed-dimension vector(768) column, matching nomic-embed-text's output size."
type = "present"
pattern = 'vector\(768\)'
paths = ["backend/migrations/0008_fix_embedding_dimension.sql"]

[[check]]
id = "embedding-adapter-function-exists"
invariant = "An embedding adapter calls an OpenAI-compatible endpoint and returns a typed error, without panicking, when unconfigured."
type = "present"
pattern = 'RINGMASTER_EMBEDDING_URL'
paths = ["backend/src/embedding_adapter.rs"]

[[check]]
id = "embed-source-fragment-function-exists"
invariant = "A function reads one source fragment's text, embeds it, and stores the result in the embeddings table."
type = "present"
pattern = 'fn embed_source_fragment'
paths = ["backend/src/graph.rs"]
```

## Notes

All three checks are automated and verified directly against the migration
and crate files that implement them. `cargo test` cases exercise, against a
live Postgres instance: the embedding adapter's "no `RINGMASTER_EMBEDDING_URL`
configured" path, and its typed-error behavior against an unreachable
endpoint. A live round-trip against a real running embedding model has
also been exercised and verified: with `RINGMASTER_EMBEDDING_URL` pointed
at a local Ollama instance (`nomic-embed-text`),
`embed_source_fragment_round_trips_against_a_live_endpoint_when_configured`
embeds a real source fragment and stores it; the resulting row was
inspected directly with `psql` (`entity_type = source_fragment`,
`model_id = nomic-embed-text`, `vector_dims(embedding) = 768`) — a real
model response, correctly stored, not a stub. Hybrid search/retrieval
over these embeddings remains out of scope, per this ADR.
