# ADR-0009: Add a generic node/edge graph substrate and source fragments table

- **Status:** Accepted
- **Date:** 2026-08-13
- **Decider:** monk-eee
- **Approval:** Continuation of accepted [docs/PRODUCT-SPEC.md](../PRODUCT-SPEC.md) Epic E2 under "pick next logical thing", 2026-08-13
- **Depends on:** [ADR-0007](0007-generalize-obligation-and-require-pgvector.md)
- **Tags:** architecture, data-model, graph

## Context

[docs/PRODUCT-SPEC.md § 5.2](../PRODUCT-SPEC.md#52-core-node-types) names 15
node types; [ADR-0007](0007-generalize-obligation-and-require-pgvector.md)
already gives Obligation (with Commitment as its promise subtype) its own
specialized, event-sourced tables. §9.2 lists `nodes`, `edges`, and
`source_fragments` as separate physical tables from `obligations`, and Epic
E2 in §16 asks for "Nodes, edges, source fragments, obligations, evidence and
temporal fields" as the Foundation's graph model.

The remaining 12 non-Obligation node types (Person, Meeting, Request,
Follow-up, Risk, Decision, Expectation, Date/Event, Customer Problem,
Outcome, Service/Feature/Work Item, Evidence) do not need event-sourcing in
the same way Obligation does — they are comparatively static, descriptive
entities that obligations link to, not the primary object whose full history
must be reconstructable. A generic, mutable `nodes`/`edges` substrate matches
how the spec itself describes them (`nodes`: "lifecycle state, created/updated
timestamps" — implying ordinary mutation, not an immutable log).

## Decision

- A generic `nodes` table represents the 12 non-Obligation, non-Source-Fragment
  node types from §5.2, discriminated by a free-text `node_type` column
  (matching how `obligation_events.event_type` is already free text rather
  than a frozen enum). Nodes are ordinary mutable rows with `created_at` /
  `updated_at`; they do **not** carry the event-sourcing immutability
  guarantee [ADR-0005](0005-adopt-rust-event-sourced-postgres-commitment-graph.md)
  gives Obligation and [ADR-0008](0008-add-append-only-audit-events-table.md)
  gives audit events. That is a deliberate distinction, not an oversight.
- A generic `edges` table connects any two entities by id — `from_id` /
  `to_id` may reference either a `nodes.id` or an Obligation's
  `obligation_id`. No foreign-key constraint enforces this polymorphism at
  the database level; correctness is an application-layer responsibility
  for now. Confidence, validity window, and provenance are carried per edge.
- A `source_fragments` table stores bounded source passages (transcript
  spans or document excerpts) with speaker, timing, classification, and a
  content hash, matching §9.2 and the transcript span shape already used in
  the §6.3 extraction object example (`start_ms` / `end_ms`).
- This ADR does not populate real data, does not build any Rust code that
  writes real Person/Meeting/Risk/etc. content, and does not add
  `evidence_events` or `attention_items` — those are coupled to extraction
  (Epic E4) and the attention engine (Epic E7), which remain undecided.
  A minimal Rust module proves the schema is usable: create a node, create
  an edge, create a source fragment, read them back.

## Scope

**In scope:** the `nodes`, `edges`, and `source_fragments` tables; a minimal
Rust module exercising create/read for each.

**Out of scope:** a frozen `node_type` / `edge_type` vocabulary,
`evidence_events`, `attention_items`, deduplication or entity-resolution
logic (§6.2), any real extraction or ingestion feature, and enforcing the
polymorphic edge endpoints at the database level.

## Options considered

- **Generic nodes/edges plus a dedicated Obligation aggregate (chosen):**
  matches §9.2's own table list; keeps the stronger event-sourcing guarantee
  only where the spec's core "was this completed, is it still relevant"
  requirement actually needs it, rather than forcing every entity through
  the heavier pattern.
- **Fold Obligation into the generic `nodes`/`edges` schema too:** would be
  more uniform, but discards the append-only, fully-derived-projection
  guarantee [ADR-0005](0005-adopt-rust-event-sourced-postgres-commitment-graph.md)
  already established and tested for the one entity that most needs it.
- **One dedicated table per node type (Person, Meeting, Risk, ...):**
  closer to a traditional relational schema, but contradicts §9.2's own
  single `nodes` table design and would need a schema migration for every
  new node type instead of a new `node_type` value.

## Consequences

- **Positive:** the graph substrate the spec's UX and agent-interaction
  sections depend on (entity views, semantic search, relationship traversal)
  now exists and is tested, without inventing extraction or attention logic
  prematurely.
- **Negative / trade-off:** no database-level guarantee prevents a bad
  `from_id`/`to_id` from pointing at nothing; that must be validated in
  application code once real writers exist.
- **Risk:** an unconstrained `node_type` vocabulary can drift into
  inconsistent naming across features. Mitigated by treating vocabulary
  choices as ordinary, ADR-0001-governed changes once real features define
  them, the same way `obligation_events.event_type` is handled today.

## Exit criteria and evidence

Evidence: [EV-0009](../evidence.d/0009-add-graph-nodes-edges-and-source-fragments.md)

| Exit criterion | Evidence |
|---|---|
| `nodes`, `edges`, and `source_fragments` tables exist per §9.2's shape | `nodes-table-exists`, `edges-table-exists`, `source-fragments-table-exists` |
| A Rust module can create and read nodes, edges, and source fragments | `graph-module-exists` |
