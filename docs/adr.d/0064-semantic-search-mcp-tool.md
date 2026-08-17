# ADR-0064: A semantic `search` MCP tool, so an agent can recall the corpus by meaning

- **Status:** Accepted
- **Date:** 2026-08-17
- **Decider:** monk-eee
- **Depends on:** [ADR-0019](0019-semantic-search-over-source-fragments.md), [ADR-0040](0040-dated-source-ingestion.md), [ADR-0062](0062-auto-embed-fragments-on-ingest.md)
- **Tags:** mcp, search, embeddings, retrieval

## Context

Ringmaster is MCP-first — the whole point of the stdio MCP server
([ADR-0040](0040-dated-source-ingestion.md)) is to let an agent put sources in
and get them back out. It exposes two tools today: `ingest_source` (write) and
`recall_sources` (read by `node_type`/`occurred_at` range only —
[ADR-0042](0042-occurred-at-retrieval-and-recall-sources-mcp-tool.md), which
"requires no embedding model"). So an agent can list *which* sources exist by
type and date, but cannot ask *"what did we say about the billing migration?"* —
there is no similarity retrieval over the MCP surface, even though
[ADR-0019](0019-semantic-search-over-source-fragments.md) already implements
exactly that for the HTTP API (`graph::search_source_fragments`) and
[ADR-0062](0062-auto-embed-fragments-on-ingest.md)/[ADR-0063](0063-reindex-backfill-embeddings.md)
have now made embeddings actually exist for the corpus.

The capability is built and the data is present; it is simply not reachable
from the one interface the product is designed around. That is the gap.

## Decision

Add a third MCP tool, `search`, to the `ringmaster-ingest` stdio server that
exposes the existing semantic search over source fragments to an agent.

- A `search` tool takes a natural-language `query` and an optional `limit`
  (default 5), and returns the ranked fragments — reusing
  [`graph::search_source_fragments`](../../backend/src/graph.rs)
  ([ADR-0019](0019-semantic-search-over-source-fragments.md)) verbatim, the
  same function the HTTP `GET /api/search` route calls. No new search logic.
- It reads the embedding model from the environment
  (`EmbeddingConfig::from_env()`); when unset it returns a clear tool error
  ("embedding is disabled") rather than empty results, mirroring the HTTP
  route's `503` posture ([ADR-0019](0019-semantic-search-over-source-fragments.md))
  so an agent is told *why* it got nothing.
- It lives entirely in the existing
  [`mcp.rs`](../../backend/src/bin/ringmaster-ingest/mcp.rs) alongside the
  other two tools, following the identical `#[tool]` / `Parameters<..>` /
  `CallToolResult` pattern — read-only, no resources/prompts/sampling.

## Scope

**In scope:** the `search` MCP tool and its parameter struct; delegating to the
existing search function; the not-configured error posture.

**Out of scope, named honestly:**

- **Any change to search ranking, fusion, or filters** — this only surfaces
  [ADR-0019](0019-semantic-search-over-source-fragments.md)'s existing behavior
  over MCP; keyword fusion / metadata filters remain its named future work.
- **Embedding-on-read or auto-reindex from this tool** — search reads existing
  embeddings; populating them stays [ADR-0062](0062-auto-embed-fragments-on-ingest.md)
  (ingest) / [ADR-0063](0063-reindex-backfill-embeddings.md) (backfill).
- **Returning graph neighbours / obligations alongside a hit** — a richer
  agent-recall shape is a separate decision; this returns the same fragment
  rows the HTTP route does.

## Options considered

- **A thin `search` MCP tool delegating to the existing function (chosen):**
  smallest change that closes the "can't recall by meaning over MCP" gap, zero
  new search logic, one file, follows the established tool pattern.
- **Extend `recall_sources` with an optional `query`:** overloads one tool with
  two very different retrieval modes (date/type list vs. similarity ranking)
  and a hard dependency on an embedding model that `recall_sources` explicitly
  does not need ([ADR-0042](0042-occurred-at-retrieval-and-recall-sources-mcp-tool.md));
  rejected as muddying a deliberately model-free tool.
- **No MCP search, tell agents to use the HTTP route:** defeats the MCP-first
  design and forces an agent to speak two protocols to the same backend;
  rejected.

## Consequences

- **Positive:** an agent can now retrieve evidence by meaning over MCP —
  "what was said about X" — turning the embeddings ADR-0062/0063 populate into
  something the product's primary interface can actually use.
- **Positive:** no new search logic or infrastructure; one cold file, one tool,
  consistent with the existing two.
- **Negative / trade-off:** the MCP server process now needs an embedding model
  configured to serve `search` (the other two tools still don't) — surfaced as
  a clear tool error when absent, not a silent empty result.
- **Risk:** low — read-only, delegates to already-tested code
  ([ADR-0019](0019-semantic-search-over-source-fragments.md)), and degrades to a
  legible error when unconfigured.

## Exit criteria and evidence

Evidence: [EV-0064](../evidence.d/0064-semantic-search-mcp-tool.md)

| Exit criterion | Evidence |
|---|---|
| A `search` MCP tool delegates to the existing semantic search | `search-mcp-tool-exists` |
| It errors legibly when no embedding model is configured | `search-tool-handles-unconfigured-model` |
| The tool is reachable over a live MCP handshake | `search-tool-listed-and-callable-over-mcp` |
