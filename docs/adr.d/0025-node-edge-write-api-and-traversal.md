# ADR-0025: Node/edge write API and neighborhood traversal

- **Status:** Accepted
- **Date:** 2026-08-14
- **Decider:** monk-eee
- **Approval:** Explicitly accepted by monk-eee on 2026-08-14
- **Depends on:** [ADR-0009](0009-add-graph-nodes-edges-and-source-fragments.md)
- **Tags:** architecture, api, data-model, graph

## Context

[ADR-0009](0009-add-graph-nodes-edges-and-source-fragments.md) built the
generic `nodes`/`edges` tables for the 12 non-Obligation
[docs/PRODUCT-SPEC.md §5.2](../PRODUCT-SPEC.md#52-core-node-types) node
types, but deliberately scoped out "any real extraction or ingestion
feature" and "any real Person/Meeting/Risk/etc. content" — it proved the
schema with a Rust-only round-trip test, nothing more. Today, nothing in
`nodes`/`edges` is reachable outside `cargo test`: no HTTP route creates,
lists, updates, or reads a node or edge. monk-eee has now asked directly
to be able to "enter data," "drill into info," see "visual
relationships," and "traverse the social graph," with nodes "enriched" —
none of which is possible while the graph substrate has no API surface at
all.

This ADR is the data-layer prerequisite, the same sequencing this
repository already used for Epic E6 (embeddings, [ADR-0018](0018-generate-and-store-source-fragment-embeddings.md),
before search, [ADR-0019](0019-semantic-search-over-source-fragments.md))
and Epic E7 (due dates, [ADR-0020](0020-obligation-due-date-fields.md),
before the Daily Brief, [ADR-0022](0022-daily-brief-endpoint.md)): build
and prove the API before any frontend visualization depends on it.

## Decision

- `POST /api/nodes` creates a node (`node_type`, `canonical_text`,
  optional `attributes` JSON object). `201` with the created node.
- `PATCH /api/nodes/:id` enriches an existing node: any of
  `canonical_text`, `lifecycle_state`, or `attributes` may be supplied.
  `attributes` is shallow-merged into the existing JSONB object (Postgres
  `||`), so enriching one attribute never clobbers others already
  recorded. `updated_at` is set to the current time. `200` with the
  updated node, `404` for an unknown id.
- `GET /api/nodes` lists nodes, optionally filtered by `?node_type=`.
  Read-only.
- `GET /api/nodes/:id` returns one node plus its immediate neighborhood:
  every edge where `from_id` or `to_id` equals this node's id, each
  paired with a summary (`id`, `node_type`, `canonical_text`) of the
  node on the *other* end when that other end is itself a `nodes` row
  (an edge terminating at an Obligation id, per
  [ADR-0009](0009-add-graph-nodes-edges-and-source-fragments.md)'s
  polymorphic design, is included with a `null` neighbor summary rather
  than a failed join). `404` for an unknown id. This is the one-hop
  traversal primitive; it does not walk beyond direct neighbors.
- `POST /api/edges` creates an edge (`from_id`, `to_id`, `edge_type`,
  optional `confidence`). `201` with the created edge. No existence check
  is performed on `from_id`/`to_id` beyond what
  [ADR-0009](0009-add-graph-nodes-edges-and-source-fragments.md) already
  established (polymorphic, unenforced at the database level).
- `nodes`/`edges` remain ordinary mutable rows, not event-sourced — this
  ADR does not revisit [ADR-0009](0009-add-graph-nodes-edges-and-source-fragments.md)'s
  deliberate choice to exempt them from the event-sourcing guarantee
  Obligation and audit events carry.
- No authentication or authorization is added; consistent with the
  already-accepted single-operator scope
  ([ADR-0004](0004-defer-multi-user-access-control-single-user-v1.md)).

## Scope

**In scope:** `POST`/`GET`/`PATCH /api/nodes`, `GET /api/nodes/:id` with
one-hop neighbor edges, `POST /api/edges`.

**Out of scope:** entity resolution/deduplication against existing nodes
(still explicitly deferred by
[ADR-0009](0009-add-graph-nodes-edges-and-source-fragments.md)); editing
or deleting edges; multi-hop graph traversal beyond one hop; bulk/batch
import; any node-type-specific validation of `attributes`' shape (every
node type shares the same generic JSON bag today); the frontend
data-entry, drill-down, or visualization UI that consumes this API (a
separate, following record).

## Options considered

- **Direct CRUD-style routes over the existing `nodes`/`edges` tables
  (chosen):** the smallest addition that makes the already-built,
  already-tested schema reachable at all; matches the plain mutable-row
  design [ADR-0009](0009-add-graph-nodes-edges-and-source-fragments.md)
  already chose for these two tables specifically.
- **Event-source node/edge mutations too, matching Obligation:**
  rejected — [ADR-0009](0009-add-graph-nodes-edges-and-source-fragments.md)
  already reasoned through this and deliberately chose otherwise for
  these comparatively static, descriptive entities; revisiting that
  choice is not what this record is about.
- **A generic graph-query language (e.g. arbitrary multi-hop traversal)
  instead of a fixed one-hop neighborhood read:** rejected as premature —
  no real data volume or traversal need exists yet to justify the design
  work; a fixed one-hop read directly serves the frontend drill-down this
  ADR is a prerequisite for, and a richer query shape can be a later,
  separately-justified decision.

## Consequences

- **Positive:** the graph substrate [ADR-0009](0009-add-graph-nodes-edges-and-source-fragments.md)
  built over a year of ADRs ago (in this repository's own compressed
  timeline) finally becomes something a person or a future frontend can
  actually create, read, enrich, and traverse.
- **Negative / trade-off:** this is the API surface's first write routes
  with no entity-resolution safeguard, so nothing stops a caller from
  creating duplicate nodes for the same real-world person or meeting;
  that risk is inherited from, not introduced by, ADR-0009's own already-
  accepted deferral.
- **Risk:** low — no new storage engine, no schema change, no auth model
  change; the only genuinely new exposure is a write path to tables that
  previously had none.

## Exit criteria and evidence

Evidence: [EV-0025](../evidence.d/0025-node-edge-write-api-and-traversal.md)

| Exit criterion | Evidence |
|---|---|
| A route creates a node and a route creates an edge | `nodes-create-route-exists`, `edges-create-route-exists` |
| A route lists nodes and a route reads one node with its one-hop neighborhood | `nodes-list-route-exists`, `nodes-detail-route-includes-neighbors` |
| A route enriches a node's attributes without clobbering previously-recorded ones | `nodes-patch-route-merges-attributes` |
