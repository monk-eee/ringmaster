# ADR-0042: Surface `occurred_at` on nodes, with date-range retrieval and a second MCP tool

- **Status:** Proposed
- **Date:** 2026-08-17
- **Decider:** monk-eee
- **Approval:** Pending — awaiting monk-eee's decision
- **Depends on:** [ADR-0009](0009-add-graph-nodes-edges-and-source-fragments.md), [ADR-0019](0019-semantic-search-over-source-fragments.md), [ADR-0025](0025-node-edge-write-api-and-traversal.md), [ADR-0040](0040-dated-source-ingestion.md)
- **Tags:** architecture, api, mcp, data-model

## Context

[ADR-0040](0040-dated-source-ingestion.md) added a required `occurred_at`
to every ingested source and three surfaces (API, CLI, MCP) to write it —
but nothing reads it back. `graph::Node` has no `occurred_at` field;
`GET /api/nodes` and `GET /api/nodes/:id` don't return it; `list_nodes`
can't filter or order by it. Since ADR-0040 shipped, `occurred_at` is
write-only: set at ingestion, invisible everywhere after.

Two ADRs already named this exact follow-up as deferred, not resolved:
[ADR-0019](0019-semantic-search-over-source-fragments.md)'s own scope says
*"No keyword fusion, metadata filters, or graph expansion -- each
deferred, per this ADR's scope."* [ADR-0040](0040-dated-source-ingestion.md)'s
own scope says reading data back out over MCP is *"the 'clever ways of
getting out' monk-eee already deferred"* when prioritizing ingestion
first, in monk-eee's own words: *"getting data in and storing it is
pretty important, we can work on clever ways of getting out [later]."*
Data is now in. This is the first "getting out" slice: given a date range,
retrieve the dated sources in it — no natural-language parsing, no new
scoring model, no embedding model required.

## Decision

- **`Node` gains `pub occurred_at: Option<DateTime<Utc>>`**, selected in
  `get_node` and `list_nodes`. Every existing response that already
  serializes a `Node` — `GET /api/nodes`, `GET /api/nodes/:id` — surfaces
  it automatically. No new route for this part.
- **`list_nodes` gains optional `occurred_from`/`occurred_to`
  (`DateTime<Utc>`) range params**, additive to its existing `node_type`
  filter. When either bound is given, results order by `occurred_at DESC
  NULLS LAST` instead of today's `updated_at DESC`. Omitting both keeps
  today's exact behavior and order, unchanged.
- **`GET /api/nodes` gains optional `?occurred_from=`/`?occurred_to=`
  query params** (RFC3339; `400` on an unparseable value), passed straight
  through to `list_nodes`.
- **The `ringmaster-ingest` MCP binary ([ADR-0040](0040-dated-source-ingestion.md))
  gains a second tool, `recall_sources`**, with the same three optional
  filters (`node_type`, `occurred_from`, `occurred_to`), calling the same
  extended `list_nodes`. No embedding model required — it always works,
  the same posture ingestion itself already has. This is the direct
  answer to pointing an agent at "what happened between date X and Y" and
  getting real, dated nodes back, the same way `ingest_source` let an
  agent put them in.

## Scope

**In scope:** `Node.occurred_at` and its serialization; `list_nodes`'s new
range filter/order; `GET /api/nodes`'s new query params; the
`recall_sources` MCP tool.

**Out of scope, named honestly:**

- **Semantic/similarity search changes.** `/api/search` and
  `search_source_fragments` ([ADR-0019](0019-semantic-search-over-source-fragments.md))
  are untouched; adding date-range filtering there, if wanted, is a
  separate, later slice.
- **Natural-language date parsing** ("last week", "this month"). Callers
  supply RFC3339 bounds directly — the same convention
  [ADR-0040](0040-dated-source-ingestion.md) already requires for
  `occurred_at` on ingestion, kept consistent rather than inventing a
  second one.
- **Person/participant filtering.** `participants` still lives in
  unstructured `attributes` JSONB, not a queryable column — resolving that
  is the same deferred owner/counterparty-linking work
  [ADR-0040](0040-dated-source-ingestion.md) already named out of scope.
- **Teaching Timeline/Time Horizon to weigh a linked source's
  `occurred_at`.** Those views still rank by Obligation due dates
  ([ADR-0029](0029-time-horizon-view.md)); this ADR makes the data
  retrievable, it does not change those views.
- **The "What am I forgetting?" one-button experience**
  ([VISION.md](../VISION.md#one-button-what-am-i-forgetting)) — a much
  larger, later product decision. This ADR only makes dated sources
  retrievable by range; it is not that experience.

## Options considered

- **Extend the existing `GET /api/nodes` + `list_nodes` (chosen):** reuses
  [ADR-0025](0025-node-edge-write-api-and-traversal.md)'s existing route,
  function, and tests; adds two optional params; no new route surface.
- **A new dedicated `GET /api/sources` route:** rejected — `nodes` already
  is the entity carrying `occurred_at`; a second route over the same
  table would just be a renamed duplicate of `/api/nodes`.
- **Add date-range filtering to `/api/search` instead:** rejected for this
  slice — search requires a configured embedding model (`503` otherwise)
  and is similarity-ranked, not chronological; a plain date-range read
  should work with zero configuration, matching how ingestion itself
  needs no model configured either.
- **Build natural-language date-range parsing now:** rejected as
  premature — no evidence yet of what phrasing/timezone handling is
  actually needed; RFC3339 bounds keep this consistent with what
  [ADR-0040](0040-dated-source-ingestion.md) already requires callers to
  supply on the way in.

## Consequences

- **Positive:** `occurred_at`, write-only since
  [ADR-0040](0040-dated-source-ingestion.md), becomes readable through
  every existing node-reading path plus one new MCP tool, without a new
  route or a configured embedding model.
- **Positive:** a second tool on the same already-accepted MCP binary
  directly answers "point an agent at a date range, get dated evidence
  back" — the explicitly-deferred half of monk-eee's own stated priority.
- **Negative / trade-off:** none identified — purely additive (a new
  optional struct field, two new optional query params, one new MCP
  tool); no existing behavior changes when the new params are omitted.
- **Risk:** low. No schema change (the column already exists from
  [ADR-0040](0040-dated-source-ingestion.md)); no new dependency; one new
  tool added to an existing binary.

## Exit criteria and evidence

Evidence: [EV-0042](../evidence.d/0042-occurred-at-retrieval-and-recall-sources-mcp-tool.md)

| Exit criterion | Evidence |
|---|---|
| `Node`, `GET /api/nodes`, and `GET /api/nodes/:id` responses include `occurred_at` | `node-responses-include-occurred-at` |
| `GET /api/nodes?occurred_from=&occurred_to=` filters by range, rejecting an unparseable bound with `400` | `nodes-route-filters-by-occurred-at-range` |
| Omitting `occurred_from`/`occurred_to` preserves today's exact response and order | `nodes-route-unchanged-without-date-params` |
| The `ringmaster-ingest` MCP server exposes a second tool, `recall_sources`, filtering by the same range/type without requiring an embedding model | `mcp-exposes-recall-sources-tool` |
