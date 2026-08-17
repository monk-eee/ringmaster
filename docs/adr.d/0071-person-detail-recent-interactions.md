# ADR-0071: Surface recent interaction sources on Person detail

- **Status:** Accepted
- **Date:** 2026-08-17
- **Decider:** monk-eee
- **Approval:** Direct instruction ("accept and continue"), 2026-08-18
- **Depends on:** [ADR-0051](0051-relationship-workspace.md), [ADR-0069](0069-resolve-participants-to-person-nodes-at-ingestion.md), [ADR-0070](0070-edge-backed-person-interaction-recency.md)
- **Tags:** api, frontend, people, relationships, provenance

## Context

The relationship design's primary projection is **Past / Now / Next**.
[ADR-0051](0051-relationship-workspace.md) shipped Now as grouped open and
at-risk Obligations. [ADR-0070](0070-edge-backed-person-interaction-recency.md)
made the header's scalar `last_interaction_at` correct by combining resolved
`participated_in` edges with the legacy speaker fallback. Person detail still
does not show Past: a manager can see *when* the latest interaction happened,
but not which recent sources contributed to that relationship context.

ADR-0070 explicitly left a recent-interactions collection and source
provenance for a separate product decision. The identity and compatibility
rules are now established, so this record can add that bounded read model
without changing ingestion or inventing generated summaries.

## Decision

- `GET /api/nodes/:id` adds `recent_interactions` and
  `recent_interactions_total` for Person nodes. Other node types return an
  empty collection and zero, preserving one stable response shape.
- An interaction source is included when either:
  1. the Person has a `participated_in` edge to the source; or
  2. a source fragment's `speaker` exactly equals the Person's
     `canonical_text`, preserving ADR-0070's pre-backfill fallback.
- Results are deduplicated by source id. If both paths identify the same
  source, `participated_in` is the evidence mode. No duplicate row is shown.
- Only sources with a non-null `occurred_at` participate in this temporal
  projection. Results order by `occurred_at DESC`, then source id for a stable
  tie break.
- Person detail returns at most the 10 newest interactions plus the total
  deduplicated count. Each item contains `source_id`, `source_type`, `title`
  (the source node's `canonical_text`), `occurred_at`, and `evidence_mode`
  (`participated_in` or `legacy_speaker`). Raw ids and evidence-mode labels
  are API provenance and are not rendered as user-facing text.
- The query is one bounded aggregate/read, not one query per interaction.
- The People detail UI adds an unframed **Recent interactions** section before
  Relationship. It renders source title, type, and a human-readable date;
  never a generated summary or raw id. An empty collection renders
  `No recorded interactions.` If more than 10 exist, it honestly displays
  `Showing the latest 10 of N.` without inventing a non-functional load-more
  control.

## Scope

**In scope:** one bounded Person-detail query; the additive response fields;
typed frontend support; the Recent interactions section and honest empty/cap
states; database-backed and browser coverage.

**Out of scope, named honestly:**

- **Backfilling `participated_in` edges.** Legacy speaker evidence remains a
  fallback rather than being converted into identity data.
- **Opening every source from the list.** Meetings have a detail surface, but
  arbitrary email/note/source detail does not; this slice does not add route
  switching that works for only one source type.
- **Fragment quotations or generated interaction summaries.** A random source
  fragment is not necessarily something the Person said, especially when a
  non-speaking participant is linked by metadata. Source identity/date is the
  honest citation available for every interaction.
- **Interaction counts on People list cards.** The list remains lean; the
  collection is progressive disclosure after selecting a Person.
- **Next conversation, calendar integration, or preparation generation.** No
  future-meeting source exists, and ADR-0051's refusal to fabricate one
  remains binding.

## Options considered

- **A capped, source-cited Past section on Person detail (chosen):** directly
  fills the next missing relationship projection using data ADR-0069/0070 now
  make trustworthy, without a new route or model call.
- **Keep only `last_interaction_at`:** preserves a compact page but leaves the
  manager unable to inspect what that date refers to; rejected as insufficient
  for the design's evidence-backed Past view.
- **Show transcript fragments as interactions:** rejected because participant
  identity does not imply authorship of an arbitrary fragment and non-meeting
  sources need the same representation.
- **Return every interaction without a cap:** rejected because Person detail is
  an attention surface, not a database browser; total count keeps truncation
  honest.

## Consequences

- **Positive:** Person detail gains a concrete, inspectable Past section using
  real source records rather than only a relative-date phrase.
- **Positive:** both resolved future interactions and unbackfilled historical
  speaker evidence remain visible under ADR-0070's compatibility posture.
- **Positive:** no migration, new route, model call, or fabricated prose.
- **Negative / trade-off:** legacy same-name speaker matches retain their known
  false-association risk and are explicitly marked in API provenance.
- **Negative / trade-off:** the first 10 items are informational until a
  source-type-independent detail/navigation decision exists.
- **Risk:** low to moderate. The response and UI are additive, but deduplication
  and cap/total behavior require explicit database coverage.

## Exit criteria and evidence

Evidence: [EV-0071](../evidence.d/0071-person-detail-recent-interactions.md)

| Exit criterion | Evidence |
|---|---|
| Person detail returns deduplicated, newest-first interaction sources across edge and legacy paths | `person-detail-returns-recent-interactions` |
| Edge evidence wins when both paths identify one source | `recent-interactions-deduplicate-with-edge-precedence` |
| The response is capped at 10 and reports the honest total | `recent-interactions-cap-and-total` |
| People detail renders Past with source title/type/date and honest empty/cap states | `people-ui-renders-recent-interactions` |
| Backend and focused browser tests pass | `recent-interactions-tests-pass` |