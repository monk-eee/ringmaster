# ADR-0019: Semantic search over embedded source fragments

- **Status:** Proposed
- **Date:** 2026-08-14
- **Decider:** monk-eee
- **Approval:** Pending — awaiting monk-eee's decision
- **Depends on:** [ADR-0012](0012-minimal-http-api-and-node-web-front-end.md), [ADR-0013](0013-http-endpoints-trigger-and-list-extraction-candidates.md), [ADR-0018](0018-generate-and-store-source-fragment-embeddings.md)
- **Tags:** architecture, api, embeddings, semantic-retrieval

## Context

[docs/PRODUCT-SPEC.md §12](../PRODUCT-SPEC.md#12-mvp-user-stories-and-acceptance-criteria)
names "Search semantically" as an MVP acceptance criterion: "a natural-
language query retrieves relevant meetings, obligations and evidence even
when wording differs." Epic E6 names the full deliverable as "Embedding
pipeline, hybrid search, metadata filters and graph expansion";
[ADR-0018](0018-generate-and-store-source-fragment-embeddings.md) built
the embedding pipeline half and left "hybrid/semantic search or any
retrieval query" explicitly out of scope. Nothing today can answer a
search query — `embeddings` rows exist but nothing reads them back by
similarity. This ADR builds the plain semantic-search slice of E6, the
smallest piece that already satisfies the named acceptance criterion for
the one entity type currently embedded (`source_fragment`).

"Hybrid" search in [docs/PRODUCT-SPEC.md](../PRODUCT-SPEC.md) means
combining vector similarity with keyword/full-text matching. Building that
that fusion, per-entity-type metadata filters, and graph expansion (walking
`edges` from a matched result) are each their own, larger, undecided design
questions this ADR does not open.

## Decision

- A new function, `search_source_fragments(pool, config, query, limit)`,
  embeds the query text with the same
  [ADR-0018](0018-generate-and-store-source-fragment-embeddings.md)
  `embedding_adapter`, then ranks `embeddings` rows where
  `entity_type = 'source_fragment'` by pgvector cosine distance (`<=>`)
  against the query vector, joins back to `source_fragments` for the
  underlying text, and returns the top `limit` matches with a similarity
  score.
- `GET /api/search?q=<query>&limit=<n>` (limit optional, default 10) calls
  it and returns ranked results as JSON
  (`source_fragment_id`, `text`, `speaker`, `similarity`). Responses:
  - `200` with the ranked array (possibly empty).
  - `400` when `q` is missing or empty.
  - `503` with a typed error body when no embedding model is configured or
    it is unreachable — mirrors
    [ADR-0013](0013-http-endpoints-trigger-and-list-extraction-candidates.md)'s
    extraction-route posture exactly; search never panics or 500s just
    because the embedding model isn't configured.
- No keyword/full-text fusion, no metadata filters (speaker, date, meeting),
  and no graph expansion are added. Only `source_fragment` is searchable,
  because it is the only entity type
  [ADR-0018](0018-generate-and-store-source-fragment-embeddings.md)
  embeds today.

## Scope

**In scope:** the ranking function; the one read-only `GET /api/search`
route; typed, non-panicking handling of every embedding-adapter outcome.

**Out of scope:** true hybrid (keyword + vector) fusion; metadata filters;
graph expansion from a matched result; searching obligations, candidates,
or nodes (none are embedded yet); a vector index (`ivfflat`/`hnsw`) —
deferred until real data volume makes a sequential scan too slow to
justify one; surfacing search in the frontend (a future, separate UI
decision).

## Options considered

- **pgvector cosine distance over the existing `embeddings` table
  (chosen):** the smallest addition that directly answers the named
  acceptance criterion, reusing
  [ADR-0018](0018-generate-and-store-source-fragment-embeddings.md)'s
  schema and adapter as-is.
- **Build full hybrid (keyword + vector) fusion now:** rejected as this
  ADR's scope — a real design question (fusion/ranking strategy) that
  deserves its own bounded decision once plain semantic search is proven,
  rather than bundling an undiscussed design into this one.
- **Add a vector index now:** rejected — the table holds a handful of rows
  in every environment today; an index tuned against no real data would be
  guesswork, not a decision.

## Consequences

- **Positive:** closes the "Search semantically" MVP acceptance criterion
  for the one entity type currently embedded; gives Epic E6 a real,
  testable end-to-end slice (embed -> store -> query) instead of a
  write-only pipeline.
- **Negative / trade-off:** search quality is bounded by plain vector
  similarity alone (no keyword fallback yet), and only covers source
  fragments, not obligations or candidates.
- **Risk:** none material — read-only, additive, and degrades to a typed
  `503` rather than blocking anything when unconfigured, the same posture
  already proven for extraction and embedding.

## Exit criteria and evidence

Evidence: [EV-0019](../evidence.d/0019-semantic-search-over-source-fragments.md)

| Exit criterion | Evidence |
|---|---|
| A function ranks source fragments by embedding similarity to a query | `search-function-exists` |
| A read-only route returns ranked search results, or a typed error for every embedding-adapter/validation outcome | `search-route-exists` |
