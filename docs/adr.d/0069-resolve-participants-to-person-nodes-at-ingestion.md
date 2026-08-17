# ADR-0069: Resolve participant/speaker names to existing Person nodes at ingestion

- **Status:** Accepted
- **Date:** 2026-08-17
- **Decider:** monk-eee
- **Approval:** Direct instruction ("continue and accept"), 2026-08-17
- **Depends on:** [ADR-0009](0009-add-graph-nodes-edges-and-source-fragments.md), [ADR-0010](0010-transcript-ingestion-parsing-chunking-provenance.md), [ADR-0040](0040-dated-source-ingestion.md), [ADR-0060](0060-extract-candidate-owner-and-link-at-promotion.md)
- **Tags:** ingestion, graph, product

## Context

[ADR-0040](0040-dated-source-ingestion.md) explicitly named this gap and
deferred it: *"Owner/counterparty/Person-node linking at ingestion or
extraction time... `participants` stays a plain string list, matching
today's meeting behavior exactly — no dedup, no resolution, no new edge."*
[ADR-0051](0051-relationship-workspace.md) later built `last_interaction_at`
as a best-effort string match between a `source_fragments.speaker` value and
a Person node's `canonical_text`, and named the same limitation again: *"Still
the same deferred work ADR-0040 named; `last_interaction_at` is a
best-effort string match, explicitly not a guaranteed identity link."*
`docs/current-status.md`'s audit still lists *"Person/participant-to-
Person-node linking at ingestion time"* as unbuilt.

Since then, [ADR-0060](0060-extract-candidate-owner-and-link-at-promotion.md)
solved the structurally identical problem for a different raw string
(`owner_name`): resolve it against existing `person` nodes by exact,
case-insensitive `canonical_text` match; on a match, create an edge; on no
match, do nothing — no new Person node fabricated. That pattern is directly
reusable here, at ingestion time, for `participants` and per-fragment
`speaker` instead of at promotion time for `owner_name`.

## Decision

Apply ADR-0060's exact-match-only, never-fabricate resolution to
ingestion. After `ingest_source` (and the meeting-specific ingestion path
it shares with `POST /api/meetings/ingest`) creates the source node and its
fragments, in the same function:

- Collect the unique set of names from `metadata.participants` and every
  created fragment's `speaker` (meeting sources only; non-meeting sources
  carry no `speaker`, matching today's behavior).
- For each unique name, look up a `person` node whose `canonical_text`
  matches case-insensitively (`lower(canonical_text) = lower($1)`, the same
  query ADR-0060 already uses).
- **Exact match found:** create a `participated_in` edge (`graph::create_edge`)
  from that person node to the newly created source node, confidence `1.0`,
  in the same transaction as fragment creation. Multiple names resolving to
  the same person within one ingestion call create at most one edge for
  that person (dedup by resolved person id before inserting).
- **No match (including an empty participant/speaker list):** nothing
  extra happens — no new Person node is created, no edge, ingestion
  behaves exactly as it does today.

## Scope

**In scope:** exact-match-only participant/speaker resolution inside
`ingest_source`'s existing transaction; a new `participated_in` edge type
created only on an exact match.

**Out of scope, named honestly:**

- **Fuzzy/partial name matching, or auto-creating a Person node when no
  match exists.** Same refusal ADR-0060 already made for `owner_name` and
  ADR-0040 already made for `participants` generally — a near-miss
  (nickname, typo, "Roopa" vs "Roopa Venkat") resolves to no link, not a
  guess. Auto-creating a Person node per unresolved transcript name would
  reintroduce the exact noisy-node-flood problem
  [ADR-0056](0056-local-test-database-isolation-and-dev-data-cleanup.md)/[ADR-0051](0051-relationship-workspace.md)
  already fixed.
- **Changing `last_interaction_at`'s computation** ([ADR-0051](0051-relationship-workspace.md))
  to prefer a `participated_in` edge over its existing string match. Real,
  separate follow-up — this ADR only adds the edge, it does not rewire an
  existing accepted read path.
- **Retroactively backfilling edges for already-ingested sources.** Mirrors
  [ADR-0040](0040-dated-source-ingestion.md)'s own precedent of not
  backfilling `occurred_at`; only newly ingested sources gain edges.
- **Resolving `counterparty` or any field beyond participant/speaker
  names.** Unrelated to this gap.

## Options considered

- **Exact case-insensitive match only, no auto-create (chosen):** directly
  reuses [ADR-0060](0060-extract-candidate-owner-and-link-at-promotion.md)'s
  already-accepted pattern; smallest change that closes the named gap
  without fabricating identity from ambiguous transcript names.
- **Fuzzy/nickname-aware matching:** rejected for the same reason
  ADR-0060 rejected it — a near-miss should resolve to no link, not a
  guess, and no validated matching model exists in this repo.
- **Auto-create a Person node for every unmatched participant/speaker
  name:** rejected — would recreate the exact noisy, low-signal
  Person-node volume ADR-0056/ADR-0051 already fixed, working directly
  against this repo's own no-fabrication posture.
- **Retroactive backfill across existing sources:** rejected as separate,
  larger follow-up work, not needed to close the specific gap named in
  `docs/current-status.md`.

## Consequences

- **Positive:** participant/speaker names that already have a matching
  Person node become a real, traversable `participated_in` edge instead of
  only a same-string coincidence — the first real step toward closing the
  "Person/participant-to-Person-node linking" gap this repo's own audit has
  named twice.
- **Positive:** additive only — no schema change (edges already support an
  arbitrary `edge_type` string per [ADR-0009](0009-add-graph-nodes-edges-and-source-fragments.md)),
  no behavior change for ingestion calls where no participant/speaker
  matches an existing person.
- **Negative / trade-off:** only resolves names that already exist as a
  Person node with an exact case-insensitive match. Most transcripts
  ingested before a person has been created manually still produce zero
  edges — an honest, named limitation, not a regression.
- **Risk:** low — no migration; a small additive step inside a transaction
  that already writes fragments, following an already-accepted pattern
  ([ADR-0060](0060-extract-candidate-owner-and-link-at-promotion.md)).

## Exit criteria and evidence

Evidence: [EV-0069](../evidence.d/0069-resolve-participants-to-person-nodes-at-ingestion.md)

| Exit criterion | Evidence |
|---|---|
| Ingestion resolves each unique participant/speaker name against existing Person nodes by exact case-insensitive match | `ingestion-resolves-participants-by-exact-match` |
| A matched name creates a `participated_in` edge from the person to the new source node, in the same transaction | `ingestion-creates-participated-in-edge-on-exact-match` |
| An unmatched name creates no new Person node and no edge — ingestion behavior is otherwise unchanged | `ingestion-unchanged-without-a-match` |
| The full backend suite passes against `ringmaster_test` with participant resolution in place | `backend-suite-passes-with-participant-linking` |
