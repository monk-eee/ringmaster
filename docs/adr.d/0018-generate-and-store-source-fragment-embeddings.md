# ADR-0018: Generate and store embeddings for source fragments

- **Status:** Proposed
- **Date:** 2026-08-14
- **Decider:** monk-eee
- **Approval:** Pending — awaiting monk-eee's decision
- **Depends on:** [ADR-0007](0007-generalize-obligation-and-require-pgvector.md), [ADR-0010](0010-transcript-ingestion-parsing-chunking-provenance.md), [ADR-0011](0011-extraction-pipeline-candidate-schema-and-model-adapter.md)
- **Tags:** architecture, data-model, embeddings, semantic-retrieval

## Context

[docs/PRODUCT-SPEC.md](../PRODUCT-SPEC.md) names Epic E6 ("Semantic
retrieval") as the next backlog item after extraction: "Embedding pipeline,
hybrid search, metadata filters and graph expansion." [ADR-0007](0007-generalize-obligation-and-require-pgvector.md)
made pgvector a mandatory extension and created an `embeddings` table, but
left its `embedding` column dimension-unconstrained, explicitly deferring
that decision: "no embedding model has been chosen yet... a follow-up
ADR-governed migration can add a fixed dimension... once one is." That gap
is what this ADR closes — nothing else.

A real, locally-running embedding model is now available: `nomic-embed-text`
was pulled into the same local Ollama instance
[ADR-0011](0011-extraction-pipeline-candidate-schema-and-model-adapter.md)'s
model adapter already talks to, confirmed reachable at
`http://localhost:11434/v1/embeddings` and producing 768-dimensional
vectors. This ADR adopts the exact same optional, OpenAI-compatible,
environment-configured adapter posture ADR-0011 already established for
chat completion, applied to embeddings instead: it must degrade cleanly and
never block anything else when unconfigured.

## Decision

- `embeddings.embedding` becomes `vector(768)` (migration
  `0008_fix_embedding_dimension.sql`), matching `nomic-embed-text`'s output
  size. The table is empty in every environment today, so this is a plain
  `ALTER COLUMN`, not a backfill.
- A new `embedding_adapter` module mirrors `model_adapter.rs` exactly:
  `EmbeddingConfig::from_env()` reads `RINGMASTER_EMBEDDING_URL` /
  `RINGMASTER_EMBEDDING_MODEL` (naming parallel to
  `RINGMASTER_LLM_URL`/`RINGMASTER_MODEL`), and `embed(config, text)` posts
  to `{url}/embeddings` and returns the vector or a typed error. It never
  panics and returns `None`/an error, never a placeholder vector, when
  unconfigured or unreachable.
- A new function, `embed_source_fragment(pool, config, source_fragment_id)`,
  reads one `source_fragments` row's immutable `text`, calls `embed`, and
  inserts one row into `embeddings` (`entity_type = "source_fragment"`,
  `entity_id = source_fragment_id`, `model_id`, `source_hash` from the
  fragment's own existing hash). Called explicitly per fragment, the same
  deliberate, non-automatic posture
  [ADR-0013](0013-http-endpoints-trigger-and-list-extraction-candidates.md)
  chose for extraction — ingestion remains fully functional with no
  embedding model configured.
- No HTTP route is added here. [ADR-0011](0011-extraction-pipeline-candidate-schema-and-model-adapter.md)
  shipped the extraction function alone before
  [ADR-0013](0013-http-endpoints-trigger-and-list-extraction-candidates.md)
  separately added its HTTP trigger; this ADR follows the same sequencing
  for embeddings.

## Scope

**In scope:** fixing `embeddings.embedding`'s dimension; the
`embedding_adapter` module; a function that embeds and stores one named
source fragment.

**Out of scope:** hybrid/semantic search or any retrieval query (the other
half of Epic E6); an HTTP trigger or automatic embedding on ingestion;
embedding any entity type other than `source_fragment` (obligations,
candidates, nodes remain unembedded for now); re-embedding on model change;
a vector similarity index (e.g. `ivfflat`/`hnsw`) — deferred until there is
enough real data for an index to matter.

## Options considered

- **Mirror `model_adapter.rs`'s optional, env-configured adapter pattern
  (chosen):** a proven, already-accepted design in this exact codebase;
  keeps embeddings and chat completion as two independently-configurable,
  independently-optional capabilities rather than conflating them.
- **Reuse `RINGMASTER_LLM_URL`/`RINGMASTER_MODEL` for embeddings too:**
  rejected — a chat-completion endpoint and an embeddings endpoint are
  usually different models (as here: `glm-4.7-flash` vs
  `nomic-embed-text`), so collapsing the config would force them to always
  match.
- **Leave the dimension unconstrained and store whatever comes back:**
  rejected — pgvector can index and compare fixed-dimension vectors far
  more usefully, and a real model is now chosen, so the deferral
  ADR-0007 named no longer needs to hold.

## Consequences

- **Positive:** closes ADR-0007's explicitly-named deferred gap; gives
  Epic E6 a real, tested foundation (a model adapter and a place to store
  its output) to build retrieval on next.
- **Negative / trade-off:** hard-coding `vector(768)` ties the schema to
  `nomic-embed-text`'s specific output size; switching embedding models
  later would need its own migration.
- **Risk:** none material — the `embeddings` table is empty everywhere
  today, and the adapter follows an already-proven non-blocking pattern.

## Exit criteria and evidence

Evidence: [EV-0018](../evidence.d/0018-generate-and-store-source-fragment-embeddings.md)

| Exit criterion | Evidence |
|---|---|
| `embeddings.embedding` is a fixed-dimension `vector(768)` column | `embedding-column-has-fixed-dimension` |
| An embedding adapter calls an OpenAI-compatible endpoint and returns a typed error, without panicking, when unconfigured | `embedding-adapter-function-exists` |
| A function embeds and stores one named source fragment's embedding | `embed-source-fragment-function-exists` |
