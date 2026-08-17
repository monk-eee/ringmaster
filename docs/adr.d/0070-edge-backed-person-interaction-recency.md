# ADR-0070: Derive Person interaction recency from identity edges with a legacy fallback

- **Status:** Accepted
- **Date:** 2026-08-17
- **Decider:** monk-eee
- **Approval:** Direct instruction ("accept and go"), 2026-08-17
- **Depends on:** [ADR-0051](0051-relationship-workspace.md), [ADR-0069](0069-resolve-participants-to-person-nodes-at-ingestion.md)
- **Amends:** [ADR-0051](0051-relationship-workspace.md)'s `last_interaction_at` derivation, which currently relies only on a source-fragment speaker string matching a Person node's `canonical_text`.
- **Tags:** api, graph, people, relationships, ingestion

## Context

[ADR-0051](0051-relationship-workspace.md) added `last_interaction_at` to
the People list and Person detail response before Ringmaster had resolved
participant identity. Both API paths therefore derive the date by matching
`source_fragments.speaker` to a Person node's `canonical_text`. That was an
honest bridge at the time, but it misses non-meeting sources, participants
who never speak, and any speaker spelling/casing that does not exactly match
the stored Person name.

[ADR-0069](0069-resolve-participants-to-person-nodes-at-ingestion.md) now
creates a real `participated_in` edge from an existing Person node to each
newly ingested source where an exact case-insensitive participant or speaker
name resolves. The identity-backed path exists, but the People read model
does not use it. ADR-0069 deliberately performs no historical backfill, so
replacing string matching outright would make older recorded interactions
disappear from the UI.

## Decision

- For a Person node, both `GET /api/nodes?node_type=person` and
  `GET /api/nodes/:id` derive `last_interaction_at` as the greatest
  `nodes.occurred_at` from either of two evidence paths:
  1. a source node reached by an edge where `from_id` is the Person id and
     `edge_type = 'participated_in'`; or
  2. the existing legacy path, where one of that source node's fragments has
     `speaker` exactly equal to the Person's `canonical_text`.
- The edge path is authoritative identity evidence and covers every source
  type, including non-meeting sources with no speaker. The legacy path remains
  a compatibility fallback for pre-ADR-0069 sources that were never
  backfilled. The exposed value is simply the maximum timestamp across both;
  no precedence rule can hide a newer historical interaction.
- The People list preserves ADR-0051's batched-query invariant: interaction
  dates for all returned Person ids/names are fetched in one query, never one
  query per card. Person detail uses one bounded aggregate query.
- Response shapes and frontend behavior do not change. Both routes still
  return the same nullable RFC3339 `last_interaction_at` field, and the People
  UI keeps its existing relative-date rendering.

## Scope

**In scope:** the two `last_interaction_at` SQL derivations in
`backend/src/api.rs`; database-backed coverage for edge-only, legacy-only,
and combined-date behavior; preserving batched list retrieval.

**Out of scope, named honestly:**

- **Backfilling `participated_in` edges for old sources.** The fallback keeps
  old evidence visible without manufacturing identity links retrospectively.
- **Removing the speaker-string fallback.** That is safe only after a verified
  backfill or an explicit decision to discard unresolved historical recency.
- **Returning a recent-interactions collection, interaction count, or source
  provenance in the API.** This decision corrects one existing derived field;
  richer relationship history is a separate product surface.
- **Changing participant resolution at ingestion.** Matching and edge creation
  remain governed by ADR-0069.
- **Frontend redesign.** The existing People cards and detail view already
  consume the field correctly.

## Options considered

- **Maximum across identity edges and the legacy speaker path (chosen):** uses
  the new durable identity relation for all future source types while retaining
  pre-ADR-0069 history and the existing response contract.
- **Use only `participated_in` edges:** cleaner, but silently erases historical
  recency because ADR-0069 explicitly declined a backfill.
- **Keep speaker matching only:** avoids a query change but leaves ADR-0069's
  identity evidence unused and still misses non-speaking participants and
  non-meeting sources.
- **Backfill first, then switch to edges only:** potentially valid later, but
  substantially larger and riskier than making current reads correct without
  mutating historical data.

## Consequences

- **Positive:** People recency is identity-backed for all newly ingested
  participant sources, not only transcript turns whose speaker text happens to
  match exactly.
- **Positive:** historical interactions remain visible without guessing or
  mutating old graph data.
- **Positive:** no client or schema migration; only the derivation becomes more
  complete.
- **Negative / trade-off:** the fallback preserves ADR-0051's known possibility
  of a false association from an exact same-name speaker in legacy data.
- **Risk:** low. Two aggregate reads change, with explicit regression coverage
  for old and new evidence paths.

## Exit criteria and evidence

Evidence: [EV-0070](../evidence.d/0070-edge-backed-person-interaction-recency.md)

| Exit criterion | Evidence |
|---|---|
| Person detail derives `last_interaction_at` from a `participated_in` source even when no speaker string matches | `person-detail-uses-participation-edge` |
| The People list derives interaction dates for all returned people in one batched edge-plus-fallback query | `person-list-batches-edge-backed-interactions` |
| A pre-ADR-0069 source with only an exact speaker match still contributes to recency | `legacy-speaker-fallback-preserved` |
| When edge and legacy paths both exist, the newest source occurrence wins | `newest-interaction-wins-across-paths` |
| The backend suite passes with the revised derivation | `backend-suite-passes-with-edge-backed-recency` |
