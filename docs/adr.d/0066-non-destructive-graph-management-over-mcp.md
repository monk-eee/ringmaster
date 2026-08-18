# ADR-0066: Expose non-destructive graph management over MCP

- **Status:** Accepted
- **Date:** 2026-08-17
- **Decider:** monk-eee
- **Approval:** Direct instruction ("add the right surface area - stop going small it s ... me over"), 2026-08-17
- **Depends on:** [ADR-0025](0025-node-edge-write-api-and-traversal.md), [ADR-0032](0032-temporal-edge-validity-supersede-on-write.md), [ADR-0040](0040-dated-source-ingestion.md), [ADR-0042](0042-occurred-at-retrieval-and-recall-sources-mcp-tool.md)
- **Amends:** [ADR-0040](0040-dated-source-ingestion.md)'s deliberately ingestion-only MCP scope and [ADR-0025](0025-node-edge-write-api-and-traversal.md)'s deferral of entity resolution, narrowly for exact-match upsert.
- **Tags:** mcp, graph, entity, relationship, api

## Context

Ringmaster's HTTP API can already create and enrich generic graph nodes and
create relationships ([ADR-0025](0025-node-edge-write-api-and-traversal.md)),
but its primary agent interface exposes only `ingest_source`,
`recall_sources`, and semantic `search`. An agent can therefore add evidence
and find graph data, but cannot manage that graph: it cannot create a Person,
set a Person's email/title/manager/team attributes, or create a relationship.

This is not a missing Person-specific convenience. It is a missing product
surface. Adding one narrow `create_person` tool would repeat the mistake for
Team, Project, Decision, Risk, and every other generic node type. The MCP
server needs the same non-destructive graph capabilities the storage layer
already supports, plus a bounded batch upsert suitable for agent-driven
enrichment.

## Decision

Add seven generic tools to the existing `ringmaster-ingest mcp-serve` server:

- `list_entities`: list nodes with optional `node_type`, exact
  `canonical_text`, occurred-at range, `limit`, and `offset` filters.
- `get_entity`: read one node by id together with every relationship touching
  it.
- `create_entity`: create one generic node from `node_type`,
  `canonical_text`, and an optional attributes object.
- `update_entity`: update an existing node by id. `canonical_text` and
  `lifecycle_state` are optional; attributes shallow-merge, matching
  [ADR-0025](0025-node-edge-write-api-and-traversal.md).
- `upsert_entities`: atomically enrich or create 1-100 entities in one call.
- `list_relationships`: list relationships touching one entity, optionally
  filtered by `edge_type` or to current relationships only.
- `create_relationship`: create an edge with the existing confidence,
  `valid_from`, and supersede-on-write semantics from
  [ADR-0032](0032-temporal-edge-validity-supersede-on-write.md).

`upsert_entities` uses an intentionally narrow identity rule:

1. Trim surrounding whitespace, then match case-sensitively on the exact
   `(node_type, canonical_text)` pair.
2. Zero matches creates a node; one match shallow-merges attributes and
   optionally updates lifecycle state; more than one match returns an
   ambiguity error rather than guessing.
3. The whole batch runs in one transaction. Any invalid item, ambiguity, or
   database failure rolls back every item.
4. Transaction-scoped advisory locks serialize concurrent upserts of the same
   exact identity. No fuzzy, alias, email, or model-driven matching occurs.
5. Each result reports `created` or `updated` and returns the resulting node.

All tool adapters remain thin and delegate to `graph.rs`; they do not call the
HTTP server or duplicate SQL. UUIDs and RFC3339 timestamps are validated into
clear tool errors. Attributes must be JSON objects so Postgres JSONB merge
semantics cannot produce surprising scalar replacement.

## Scope

**In scope:** the seven MCP tools; shared graph read helpers; exact-match,
atomic batch upsert; argument validation and actionable errors; live MCP
handshake verification.

**Out of scope, named honestly:**

- **Delete operations.** Source nodes own append-only evidence fragments, so
  generic deletion needs retention and orphan semantics before exposure.
- **Fuzzy entity resolution or aliases.** Exact matching is deterministic;
  richer identity is a separate data-model decision.
- **Person-specific schemas.** Email, title, manager, and team remain keys in
  the generic attributes object, consistent with ADR-0025.
- **Bulk relationship creation.** Entity enrichment is the demonstrated batch
  need; relationship writes remain one explicit edge per call.
- **Authentication or multi-user authorization.** The accepted single-user
  posture remains unchanged.

## Options considered

- **A complete non-destructive generic graph surface plus batch upsert
  (chosen):** exposes the graph as a product capability, supports every node
  type, and makes multi-person enrichment one atomic call.
- **Only `create_person` and `update_person`:** rejected because it hard-codes
  one of the graph's node types and leaves the same gap everywhere else.
- **Only `update_entity`:** rejected because agents would still need a second
  protocol for creation, discovery, and relationship management, and 13
  updates would require 13 calls.
- **Full CRUD including delete:** rejected until evidence retention and orphan
  behavior are decided.
- **Fuzzy name-based upsert:** rejected because silently merging two people is
  worse than returning an ambiguity that a caller can resolve by id.

## Consequences

- **Positive:** an MCP agent can discover, create, enrich, and connect the
  entire Ringmaster graph without falling back to HTTP or asking a human to
  run database commands.
- **Positive:** a set such as 13 existing Person nodes can be enriched in one
  atomic call while preserving every pre-existing attribute.
- **Negative / trade-off:** exact matching still surfaces existing duplicate
  identities as an error requiring explicit cleanup or id-based updates.
- **Risk:** moderate. The tools expose existing mutable graph writes to agents;
  bounded batches, validation, transactions, and no delete keep the blast
  radius controlled.

## Exit criteria and evidence

Evidence: [EV-0066](../evidence.d/0066-non-destructive-graph-management-over-mcp.md)

| Exit criterion | Evidence |
|---|---|
| MCP exposes entity list/get/create/update and exact batch upsert | `entity-management-tools-exposed` |
| Batch upsert exact-matches, shallow-merges, creates missing entities, rejects ambiguity, and is atomic | `atomic-exact-entity-upsert` |
| MCP exposes relationship list/create with temporal supersede options | `relationship-management-tools-exposed` |
| All seven tools are listed and representative calls work over a live stdio MCP handshake | `graph-tools-live-mcp` |
