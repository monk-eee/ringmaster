# ADR-0082: Repeated-concern signal — the same risk raised in multiple meetings, still unpromoted

- **Status:** Accepted
- **Date:** 2026-08-19
- **Decider:** monk-eee
- **Approval:** Explicitly accepted by monk-eee on 2026-08-19 ("accept all")
- **Depends on:** [ADR-0011](0011-extraction-pipeline-candidate-schema-and-model-adapter.md), [ADR-0018](0018-generate-and-store-source-fragment-embeddings.md), [ADR-0019](0019-semantic-search-over-source-fragments.md), [ADR-0024](0024-candidate-accept-reject-buttons.md), [ADR-0027](0027-promote-accepted-candidate-to-obligation.md)
- **Tags:** api, data-model

## Context

[docs/PRODUCT-SPEC.md §7.1](../PRODUCT-SPEC.md#71-initial-risk-signals) names
nine risk signals. Four are live today — `stale`/`date_compression`
([ADR-0041](0041-risk-engine-v1-staleness-and-date-compression-signals.md)),
`unowned` ([ADR-0046](0046-unowned-obligation-risk-signal.md)), `isolated`
([ADR-0054](0054-congruence-engine-v1-isolated-commitment-signal.md)) — but
"Repeated concern" (*"the same risk appears in multiple meetings without
mitigation"*) is not, and is named directly in
[docs/IMPROVEMENT-PLAN.md §1.1](../IMPROVEMENT-PLAN.md#11-repeated-concern-risk-signal-congruence-engine-v2)
as the next-highest-leverage gap. It is also the direct structural
replacement for Learn.ADOA's manual `THEMES.md` rule (a person remembers a
risk was raised before and promotes it to a numbered theme by hand) — the
gap analysis this repo tracks itself against.

Three real, existing pieces make this checkable without fabricating data:

- **`risk`-typed candidates** already exist as their own
  `candidate_type` ([ADR-0011](0011-extraction-pipeline-candidate-schema-and-model-adapter.md)),
  independent of Obligation (Obligation carries no `kind`/type column —
  confirmed in `backend/src/obligation.rs` — so this signal must key off
  `candidate_projection`, not a promoted Obligation).
- **Embeddings already exist** for every source fragment
  ([ADR-0018](0018-generate-and-store-source-fragment-embeddings.md)), and
  pgvector cosine distance is already a proven, in-repo pattern
  (`backend/src/graph/source_fragment.rs::search_source_fragments`).
- **`candidate_projection.source_fragment_id` → `source_fragments.source_id`**
  already resolves a candidate back to the meeting node it came from, so
  "distinct meetings" is a plain `COUNT(DISTINCT source_id)`, not new
  plumbing.

## Decision

- **A new signal, `repeated_concern`, computed over `risk`-typed candidates,
  not Obligations.** A `risk` candidate qualifies when both hold:
  1. **At least one other `risk` candidate from a different meeting**
     (different `source_fragments.source_id`) has a stored embedding within
     cosine similarity **≥ 0.85** of this one's source fragment embedding
     (reusing the exact `1 - (embedding <=> embedding)` expression
     `search_source_fragments` already uses, as a self-join instead of an
     ad hoc query embed).
  2. **Neither side of that match has been promoted** —
     `candidate_projection.promoted_obligation_id IS NULL` on both. A
     promoted risk already has a real, owned, tracked Obligation; that *is*
     this repo's existing definition of "being acted on," so promotion is
     the honest, already-real proxy for "mitigation," not a new concept.
  3. **Rejected candidates are excluded** on both sides
     (`validation_state != 'rejected'`) — a rejected candidate was
     judged "not a genuine management object"
     ([docs/PRODUCT-SPEC.md §6](../PRODUCT-SPEC.md#6-candidate-lifecycle)),
     so it cannot itself constitute a repeat.
- **Surfaced on the candidate list/Inbox, not `risk_signals`.** The existing
  `risk_signals` attachment point (Daily Brief, Time Horizon, Obligation
  detail) is Obligation-shaped and computed post-promotion
  ([ADR-0041](0041-risk-engine-v1-staleness-and-date-compression-signals.md)/[ADR-0046](0046-unowned-obligation-risk-signal.md)/[ADR-0054](0054-congruence-engine-v1-isolated-commitment-signal.md)).
  This signal is true of an *unpromoted* candidate, so it is attached to
  the candidate list response (`GET /api/candidates`,
  [ADR-0024](0024-candidate-accept-reject-buttons.md)) as a
  `repeated_concern` field naming the matched candidate id(s) and meeting(s),
  the same explanation-required shape §7.1 asks of every signal.
- **No clustering algorithm.** Pairwise matching only (does *this* risk
  candidate have at least one qualifying match), not transitive grouping
  into named themes. A true multi-way cluster is a stretch goal, named out
  of scope below rather than silently attempted.

## Scope

**In scope:**

- The `repeated_concern` computation described above, pure and unit-testable
  the same way `risk_signals` is (a function over already-fetched rows plus
  one new pairwise-similarity query).
- Attaching it to the existing candidate list route response.
- A fixed 0.85 similarity threshold, named in code as a constant (matching
  `STALE_THRESHOLD_DAYS`/`DATE_COMPRESSION_WINDOW_DAYS`'s existing pattern
  in `backend/src/api/obligations.rs`), not user-configurable.

**Out of scope, named honestly:**

- **Transitive clustering into a named, numbered theme** (Learn.ADOA's full
  `THEMES.md` behavior). This ships the pairwise detection only; grouping
  three or more mutually-similar risks into one addressable "theme" entity
  is real, larger, later work.
- **Any UI beyond the candidate list/Inbox.** No new page, no dedicated
  "themes" view.
- **Non-`risk` candidate types.** `commitment`/`request`/`follow_up`/
  `decision`/`expectation` are unaffected.
- **A configurable or learned threshold.** 0.85 is a fixed starting point;
  tuning it against real data is separate follow-up work, not decided here.
- **Superseded-tracking.** If a repeated risk is later manually merged or
  noted as duplicate, no new lifecycle state is added; existing
  accept/reject/promote/correct actions are the only levers.

## Options considered

- **Candidate-level pairwise signal via existing embeddings (chosen):**
  reuses `risk`'s existing status as its own candidate type, the existing
  embeddings table, and the existing cosine-distance pattern; ships the
  literal PRODUCT-SPEC wording ("same risk … multiple meetings … no
  mitigation") without fabricating a new subsystem.
- **Full clustering into named themes (Learn.ADOA parity):** rejected for
  v1 — real value, but a materially larger design (cluster membership,
  naming, merge/demote lifecycle) that deserves its own ADR once the
  pairwise signal is live and used.
- **Key off promoted Obligations instead of candidates:** rejected —
  Obligation has no `kind` column distinguishing a former risk from a
  former commitment, so this would require a schema change this ADR does
  not need; candidates already carry `candidate_type` today.
- **Treat "mitigation" as a linked edge (isolated-style) rather than
  promotion:** rejected — an unpromoted risk candidate has no obligation
  row to attach an edge to at all; promotion is the only existing
  state transition that means "someone is now tracking this."

## Consequences

- **Positive:** closes the fifth of nine named PRODUCT-SPEC signals, and is
  the direct structural replacement for the Learn.ADOA `THEMES.md` habit
  this repo is explicitly measuring itself against.
- **Positive:** zero schema change; reuses `candidate_projection`,
  `embeddings`, and `source_fragments` exactly as they exist today.
- **Negative / trade-off:** pairwise-only, not full clustering — three
  meetings raising the same risk will show as multiple pairwise flags, not
  one merged theme, until later work.
- **Risk:** low-moderate. The similarity self-join is the only new query
  shape (existing single-query embed-and-rank in
  `search_source_fragments` is per-query-string, not per-stored-row); needs
  a covering index check (`embeddings` already exists per ADR-0018) and a
  fixed candidate-count bound (existing pagination,
  [ADR-0059](0059-list-view-pagination.md)) so the self-join stays cheap.

## Exit criteria and evidence

Evidence: [EV-0082](../evidence.d/0082-repeated-concern-risk-signal.md)

| Exit criterion | Evidence |
|---|---|
| Two `risk` candidates from different meetings with cosine similarity ≥ 0.85, neither promoted, are both flagged `repeated_concern` naming each other | `repeated-concern-flags-cross-meeting-similar-risks` |
| Two similar `risk` candidates from the *same* meeting are not flagged (distinct-meeting requirement) | `repeated-concern-requires-distinct-meetings` |
| A matching pair where either side is already promoted is not flagged | `repeated-concern-excludes-promoted-risks` |
| A matching pair where either side is rejected is not flagged | `repeated-concern-excludes-rejected-candidates` |
| Two dissimilar `risk` candidates (below threshold) are not flagged | `repeated-concern-requires-similarity-threshold` |
| `repeated_concern` appears on `GET /api/candidates` responses, with an explanation naming the matched meeting(s) | `repeated-concern-attached-to-candidate-list-route` |
