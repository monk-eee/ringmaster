# ADR-0063: A reindex command to backfill embeddings for existing fragments

- **Status:** Accepted
- **Date:** 2026-08-17
- **Decider:** monk-eee
- **Depends on:** [ADR-0018](0018-generate-and-store-source-fragment-embeddings.md), [ADR-0040](0040-dated-source-ingestion.md), [ADR-0062](0062-auto-embed-fragments-on-ingest.md)
- **Tags:** search, embeddings, cli, operations

## Context

[ADR-0062](0062-auto-embed-fragments-on-ingest.md) made ingestion auto-embed
fragments so search populates in normal use — but it explicitly deferred
**back-filling embeddings for fragments ingested before that change**. Right
now the live dev database holds source fragments that predate auto-embed (and
any ingested while no embedding model was configured), all with no embedding
row, so semantic search ([ADR-0019](0019-semantic-search-over-source-fragments.md))
silently covers only the sliver ingested after
[ADR-0062](0062-auto-embed-fragments-on-ingest.md) landed. There is no way to
make already-stored fragments searchable short of re-ingesting them, which
would duplicate meetings and fragments — exactly what
[ADR-0010](0010-transcript-ingestion-parsing-chunking-provenance.md)'s
immutable, hashed provenance is designed to avoid.

A one-shot backfill is the missing operational step: embed every fragment that
lacks an embedding, using the same adapter everything else uses, without
touching the immutable fragments themselves.

## Decision

Add a `reindex-embeddings` subcommand to the existing `ringmaster-ingest` CLI
([ADR-0040](0040-dated-source-ingestion.md)) that backfills embeddings for
every source fragment with no embedding yet.

- A new library function
  [`transcript::reindex_unembedded_fragments(pool, config)`](../../backend/src/transcript.rs)
  selects every `source_fragment` with no `entity_type = 'source_fragment'`
  embedding row (a `LEFT JOIN ... WHERE e.id IS NULL`), embeds each via the
  existing [`graph::embed_source_fragment`](../../backend/src/graph.rs)
  ([ADR-0018](0018-generate-and-store-source-fragment-embeddings.md)), and
  returns `(candidates, embedded)`.
- It is **best-effort per fragment**: a failed embed is logged and skipped;
  the run continues and reports how many of the candidates succeeded — same
  posture as [ADR-0062](0062-auto-embed-fragments-on-ingest.md)'s auto-embed.
- The CLI's `reindex-embeddings` subcommand connects to `DATABASE_URL`, reads
  the embedding model from the environment (erroring clearly if unset, since a
  backfill with no model would be a silent no-op the operator did not intend),
  runs the function, and prints a small JSON summary
  (`{"unembedded_before": N, "embedded": M}`).
- It reads and writes only the `embeddings` table (append), never mutates a
  `source_fragment` — immutability ([ADR-0010](0010-transcript-ingestion-parsing-chunking-provenance.md))
  is untouched.

## Scope

**In scope:** the `reindex_unembedded_fragments` function; the
`reindex-embeddings` CLI subcommand; a tolerant test proving a previously
unembedded fragment becomes embedded when a model is configured.

**Out of scope, named honestly:**

- **An HTTP route or UI button for reindex.** The CLI is the corpus-scripting
  surface ([ADR-0040](0040-dated-source-ingestion.md)); a route can come later
  if wanted, but is not needed to close the backfill gap.
- **Idempotency beyond "has any embedding row".** A fragment with an existing
  embedding is skipped by the `WHERE e.id IS NULL` filter; re-embedding after a
  model change (to refresh vectors) is a different, later decision — this only
  fills gaps, never re-embeds.
- **Progress streaming / batching / concurrency.** A simple sequential loop;
  fine for local-first single-user corpus sizes, matching
  [ADR-0062](0062-auto-embed-fragments-on-ingest.md)'s no-queue choice.
- **Embedding anything but source fragments** — unchanged from
  [ADR-0018](0018-generate-and-store-source-fragment-embeddings.md)/[ADR-0019](0019-semantic-search-over-source-fragments.md).

## Options considered

- **A CLI subcommand backfilling all unembedded fragments (chosen):** reuses
  the existing scripting surface and the existing embed function, touches only
  the append-only `embeddings` table, and is the smallest thing that makes the
  existing corpus searchable.
- **A `POST /api/embeddings/reindex` route:** more discoverable, but adds a
  route to a heavily-contended file for an operation that is fundamentally a
  one-shot operator action, not a per-request product feature; deferred.
- **Auto-backfill on backend boot:** would embed the whole corpus on every
  restart, an unbounded, surprising side effect of starting the server, and
  would fight [ADR-0018](0018-generate-and-store-source-fragment-embeddings.md)'s
  "never block startup" posture; rejected.
- **Re-ingest the corpus:** duplicates immutable meetings/fragments, defeating
  [ADR-0010](0010-transcript-ingestion-parsing-chunking-provenance.md)'s
  provenance; rejected.

## Consequences

- **Positive:** the entire already-ingested corpus becomes searchable with one
  command, completing [ADR-0062](0062-auto-embed-fragments-on-ingest.md)'s
  half-step; search stops silently covering only post-0062 data.
- **Positive:** no new infrastructure, no route on a contended file, no change
  to immutable fragments.
- **Negative / trade-off:** a large corpus backfill issues one embedding call
  per fragment sequentially — acceptable at local-first scale, and a queue is
  the named future answer if it ever isn't.
- **Risk:** low — errors are per-fragment and skipped; with no model configured
  the command refuses up front rather than silently doing nothing.

## Exit criteria and evidence

Evidence: [EV-0063](../evidence.d/0063-reindex-backfill-embeddings.md)

| Exit criterion | Evidence |
|---|---|
| A function backfills embeddings for unembedded fragments | `reindex-function-exists` |
| A `reindex-embeddings` CLI subcommand invokes it | `cli-exposes-reindex-embeddings` |
| The backfill is covered by a test and the suite passes | `reindex-is-tested-and-suite-passes` |
