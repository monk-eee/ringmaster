# ADR-0062: Auto-embed fragments on ingest (best-effort), so search has data

- **Status:** Accepted
- **Date:** 2026-08-17
- **Decider:** monk-eee
- **Amends:** [ADR-0010](0010-transcript-ingestion-parsing-chunking-provenance.md), [ADR-0018](0018-generate-and-store-source-fragment-embeddings.md)
- **Depends on:** [ADR-0018](0018-generate-and-store-source-fragment-embeddings.md), [ADR-0019](0019-semantic-search-over-source-fragments.md), [ADR-0040](0040-dated-source-ingestion.md)
- **Tags:** ingestion, search, embeddings

## Context

Semantic search ([ADR-0019](0019-semantic-search-over-source-fragments.md)) is
infrastructurally complete — an embedding adapter
([ADR-0018](0018-generate-and-store-source-fragment-embeddings.md)), a
`vector(768)` column, `GET /api/search` returning `200` — but the live audit
([docs/current-status.md](../current-status.md)) found it returns **nothing**,
because **zero embeddings exist**: embedding was deliberately left a manual,
never-automatic step ([ADR-0018](0018-generate-and-store-source-fragment-embeddings.md)),
and ingestion explicitly "never generates embeddings"
([ADR-0010](0010-transcript-ingestion-parsing-chunking-provenance.md),
[ADR-0040](0040-dated-source-ingestion.md)). So every fragment ingested via
any of the three surfaces (HTTP, CLI, MCP) lands unsearchable, and nothing in
normal use ever calls the embed function. The one dev-DB reset this session
wiped the 25 embeddings that a manual run had once created, leaving search
dead.

The manual-only posture existed for a good reason:
[ADR-0018](0018-generate-and-store-source-fragment-embeddings.md) required
embedding to "degrade cleanly and never block ingestion, extraction, or
storage when unconfigured." That guarantee must survive any change here — a
missing or slow embedding model must never fail or delay an ingest.

## Decision

Make embedding happen automatically on ingest, **best-effort and after the
write commits**, so search has data in normal use without weakening
[ADR-0018](0018-generate-and-store-source-fragment-embeddings.md)'s
non-blocking guarantee.

- A private `embed_fragments_best_effort(pool, &fragment_ids)` in
  [`backend/src/transcript.rs`](../../backend/src/transcript.rs) runs **after**
  the ingest transaction commits — never inside it — so a slow or failing
  external embedding call can neither hold the ingest transaction open nor
  roll a stored meeting back.
- It embeds each just-written fragment by delegating to the existing
  [`graph::embed_source_fragment`](../../backend/src/graph.rs)
  ([ADR-0018](0018-generate-and-store-source-fragment-embeddings.md)) — no new
  embedding logic.
- It is **best-effort**: when `EmbeddingConfig::from_env()` is `None` (no model
  configured, e.g. CI), it returns immediately; when an individual embed call
  fails, it logs and continues. Ingestion always returns `Ok` with its
  fragments regardless — the fragment simply stays unembedded, exactly as
  today.
- Both ingest entry points call it after commit: `ingest_transcript`
  (`/api/meetings`, [ADR-0034](0034-http-meeting-transcript-ingestion.md)) and
  `ingest_source` (the one function API/CLI/MCP share,
  [ADR-0040](0040-dated-source-ingestion.md)) — so all surfaces benefit from a
  single helper.

## Scope

**In scope:** the best-effort post-commit helper; calling it from both ingest
functions; a tolerant test proving auto-embed occurs when a model is
configured and that ingest still succeeds when it is not.

**Out of scope, named honestly:**

- **Idempotency / dedup of embeddings.** Re-embedding a fragment can still
  create a duplicate `embeddings` row — a pre-existing property of
  [ADR-0018](0018-generate-and-store-source-fragment-embeddings.md)'s manual
  path, not introduced here; ingest embeds each new immutable fragment exactly
  once, so no duplication arises from this change alone.
- **Back-filling embeddings for fragments ingested before this change.** A
  separate one-off; this decision only covers new ingests.
- **Retry/queue for a transient embedding failure.** Best-effort means a
  failed fragment stays unembedded until a future manual/again-ingest path;
  no background retry is introduced.
- **Embedding candidates, obligations, or nodes** — only source fragments, as
  [ADR-0018](0018-generate-and-store-source-fragment-embeddings.md)/[ADR-0019](0019-semantic-search-over-source-fragments.md)
  scope.

## Options considered

- **Best-effort embed after commit, in the shared ingest layer (chosen):**
  one helper, both surfaces, search populated in normal use, and
  [ADR-0018](0018-generate-and-store-source-fragment-embeddings.md)'s
  non-blocking guarantee preserved because the embed runs outside the
  transaction and never propagates an error to the caller.
- **Embed inside the ingest transaction:** would make an external HTTP call
  hold a DB transaction open and let an embedding failure roll back a stored
  meeting — directly violates
  [ADR-0018](0018-generate-and-store-source-fragment-embeddings.md); rejected.
- **A background worker / queue:** the robust long-term answer for retries and
  throughput, but real new infrastructure (a queue, a worker loop, delivery
  semantics) for a single-user local-first app; deferred as over-engineering
  for now.
- **Keep embedding manual, add a "reindex all" button instead:** leaves the
  default demo path returning empty search results and pushes a manual step
  onto every use; rejected as not fixing the actual gap.

## Consequences

- **Positive:** search returns real results in normal use — ingesting a source
  makes it findable without a separate manual step — closing the "0 embeddings,
  search dead" gap the audit surfaced.
- **Positive:** one shared helper; all three ingestion surfaces benefit; no new
  infrastructure.
- **Negative / trade-off:** ingest now issues N external embedding calls after
  committing (one per fragment) when a model is configured; these are outside
  the transaction and non-blocking to correctness, but do add wall-clock time
  to an ingest against a live model. Acceptable for local-first single-user
  scale; a queue is the named future answer if it ever isn't.
- **Risk:** low — when unconfigured (CI) the helper is a no-op; when a call
  fails, the fragment stays unembedded exactly as it is today.

## Exit criteria and evidence

Evidence: [EV-0062](../evidence.d/0062-auto-embed-fragments-on-ingest.md)

| Exit criterion | Evidence |
|---|---|
| Ingest triggers best-effort embedding after commit | `ingest-triggers-best-effort-embedding` |
| The behavior is covered by a test | `auto-embed-is-tested` |
| The full backend suite passes; ingest stays non-blocking when embedding is unconfigured or fails | `backend-suite-passes-and-ingest-stays-non-blocking` |
