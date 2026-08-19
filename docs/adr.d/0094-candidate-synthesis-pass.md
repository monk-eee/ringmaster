# ADR-0094: Candidate synthesis pass — re-assemble same-source fragments before they reach review

- **Status:** Accepted
- **Date:** 2026-08-19
- **Decider:** monk-eee
- **Approval:** Direct instruction ("yeah synthesis pass for sure"), following a live, evidence-based diagnosis of why the candidate pipeline feels less useful than reading raw notes with a frontier model, 2026-08-19
- **Depends on:** [ADR-0010](0010-transcript-ingestion-parsing-chunking-provenance.md), [ADR-0011](0011-extraction-pipeline-candidate-schema-and-model-adapter.md), [ADR-0065](0065-model-adapter-optional-api-key-hosted-first.md)
- **Tags:** extraction, backend, product

## Context

A live diagnosis against the real (non-test) `ringmaster` database, prompted
by monk-eee's own skepticism ("the extractions are not granular enough for
me and the linkages are not obvious"), found the actual root cause:

- Of 227 real candidates, **175 sit in `accepted` state, never promoted** —
  only 50 ever became a real Obligation. Today's "nothing needs attention"
  emptiness is a promotion-funnel problem, not an ingestion problem: 907
  real source fragments across `1on1`/`comms`/`connect`/`note`/`perspective`
  content exist and are being extracted from.
- Tracing `person_brief` for a real person (Sowkot Osman) surfaced 10
  candidates from what is really **one coherent Connect/goal-review
  document with three goals**. Each candidate came from a *different*
  `source_fragments` row (confirmed: distinct `source_fragment_id`s at
  sequence 11/13/14/24/29, each only 150–530 characters — [ADR-0010](0010-transcript-ingestion-parsing-chunking-provenance.md)'s
  paragraph-level chunking splits a multi-paragraph document into many
  small fragments). [ADR-0011](0011-extraction-pipeline-candidate-schema-and-model-adapter.md)'s
  extraction runs **independently per fragment**, with no awareness of
  sibling fragments from the same source — so two sentences from the same
  paragraph ("I will responsibly adopt tooling..." / "Adopting tooling
  will improve velocity...") become two disconnected candidates instead of
  one synthesized goal. One candidate was even mislabeled `risk` for text
  that is actually positive/aspirational framing.
- A direct side-by-side comparison proved the gap: asking a frontier model
  (via WorkIQ) to read the same person's real source documents directly
  produced three clean, cited goals in one pass. Ringmaster's structured,
  per-fragment pipeline produced ten disconnected fragments and zero
  promotions from the same underlying material.

The conclusion, and monk-eee's explicit direction: add a **synthesis
pass** that re-assembles same-source candidates that describe the same
underlying goal/commitment/topic into fewer, clearer, still-evidence-backed
units, before (or instead of) presenting the raw per-fragment list.

## Decision

- **A new, additive read/compose step**, not a replacement of extraction:
  raw per-fragment candidates ([ADR-0011](0011-extraction-pipeline-candidate-schema-and-model-adapter.md))
  are unchanged — synthesis runs *after* extraction, over one source's
  still-`accepted` candidates, and produces a smaller set of synthesized
  groups, each naming which raw candidates it consolidates. Nothing is
  deleted or silently hidden; a synthesized group is a lens over the same
  underlying evidence, not a replacement of it.
- **New table `candidate_synthesis_groups`** (migration `0014`):
  `id`, `source_id` (the document/meeting node these candidates share),
  `synthesized_statement`, `candidate_type` (still one of
  [ADR-0011](0011-extraction-pipeline-candidate-schema-and-model-adapter.md)'s
  six allowed types), `member_candidate_ids` (`uuid[]`, the raw candidates
  it merges — always at least one, so a "synthesis" of a single
  already-atomic candidate is a legal, honest no-op group, not a special
  case), `synthesis_model`, `created_at`. Insert-only, matching
  `source_fragments`' own immutable-evidence posture (a `BEFORE
  UPDATE/DELETE` trigger rejects mutation, for the same reason: a
  synthesis result is a dated interpretation, and revising it should mean
  running synthesis again, not silently rewriting history).
- **`synthesize_candidates_for_source(pool, model_config, source_id)`**
  (new `backend/src/synthesis.rs` module, mirroring
  `extraction::extract_candidate_via_model`'s existing model-adapter call
  shape exactly): fetches every `accepted`-state candidate whose
  `source_fragment_id` joins to a `source_fragments` row with that
  `source_id`, sends their statements/types/confidences as ONE prompt
  (not one call per fragment), and asks the model to group candidates
  describing the same real goal/commitment/topic into synthesized
  statements. A never-grouped candidate becomes its own one-member group
  rather than being dropped — every accepted candidate is still
  accounted for.
- **Manual and synchronous, matching extraction's own existing posture**
  (`extract_source_fragment`'s doc comment already states extraction "is
  ... synchronous, never automatic on ingestion"): synthesis runs on
  explicit request for one source, not automatically on every ingest or
  every accept.
- **This record is backend-only.** The API route to trigger synthesis
  (`POST /api/sources/:id/synthesize`), the read route
  (`GET /api/sources/:id/synthesis`), and any frontend surface to view
  synthesized groups are named here but deliberately deferred to a
  follow-up slice — see Scope.

## Scope

**In scope:** migration `0014_candidate_synthesis_groups.sql`;
`backend/src/synthesis.rs`'s `synthesize_candidates_for_source` function,
its prompt construction, and its persistence into
`candidate_synthesis_groups`; unit tests for the persistence/grouping
logic (following [ADR-0011](0011-extraction-pipeline-candidate-schema-and-model-adapter.md)'s
own established convention of skipping the live-model-call test gracefully
when no `RINGMASTER_LLM_URL`/`RINGMASTER_MODEL` is configured, rather than
mocking the HTTP call).

**Out of scope, named honestly and deferred to a follow-up ADR/slice:**
wiring `POST /api/sources/:id/synthesize` and
`GET /api/sources/:id/synthesis` into `backend/src/api/mod.rs`'s router,
and any frontend surface to view or trigger synthesis. Deferred
specifically because, at the time this record was written, a concurrent
session held uncommitted, in-progress edits to `backend/src/api/mod.rs`,
`backend/src/api/obligations.rs`, `backend/src/obligation.rs`, and nearly
every Today/Focus-Blocks/Graph/Obligation-detail frontend component —
editing those same files now would risk clobbering that work rather than
extending it. Also out of scope: automatically re-running synthesis when
new candidates are accepted for an already-synthesized source (a real
question — does a new group form, or does an existing group grow? —
better answered once the manual trigger has real usage); driving the
175-candidate promotion backlog down automatically (synthesis makes the
backlog *reviewable*, it does not itself promote anything); changing
[ADR-0010](0010-transcript-ingestion-parsing-chunking-provenance.md)'s
chunking grain (a synthesis pass after the fact was chosen over
re-chunking coarser — see Options below).

## Options considered

- **A synthesis pass over existing fragment-level extraction (chosen):**
  keeps [ADR-0010](0010-transcript-ingestion-parsing-chunking-provenance.md)'s
  fine-grained chunking and its real benefits (precise per-sentence
  evidence citation, small stable hashes) while fixing the actual
  complaint — nothing today re-assembles siblings into one story. Additive
  and reversible: raw candidates remain queryable exactly as before.
- **Re-chunk coarser at ingestion time (rejected):** would reduce fragment
  count, but trades away exact-sentence evidence citation (an existing,
  valued property — `daily_brief_reason`, `person_brief`'s recent-asks,
  and Timeline's `source_occurred_at` all cite fragment-level text) for a
  cruder fix that doesn't address the deeper issue: extraction itself has
  zero cross-fragment awareness, no matter the chunk size.
  Also,structural: existing fragments are immutable and hashed
  ([ADR-0010](0010-transcript-ingestion-parsing-chunking-provenance.md)/
  [ADR-0056](0056-local-test-database-isolation-and-dev-data-cleanup.md)'s
  own caution about touching already-ingested evidence) — re-chunking
  would mean re-ingesting everything, a much larger and riskier change.
- **Synthesize automatically on every accept (rejected):** matches neither
  extraction's own established manual/synchronous posture nor this
  record's honest uncertainty about re-run semantics; a manual trigger per
  source is the smaller, safer first step.
- **Ship the API route and frontend surface in the same pass (rejected
  for now):** the backend logic is independently valuable and testable;
  wiring it into files several other concurrent edits currently touch is
  an avoidable, unforced collision risk for zero added correctness.

## Consequences

- **Positive:** directly answers monk-eee's diagnosis — a source's
  candidates can be requested as a smaller, synthesized, still-cited set
  instead of a flat per-fragment list. Every synthesized statement still
  names its member candidates, so "how do I know this is real" is always
  one hop from "here's the synthesis."
- **Negative / trade-off:** this record alone does not change what
  monk-eee sees in the product — no route, no UI — until the follow-up
  slice lands. Named honestly rather than silently shipped as "done."
- **Risk:** synthesis quality depends entirely on the configured model
  (same dependency [ADR-0011](0011-extraction-pipeline-candidate-schema-and-model-adapter.md)
  already carries); a bad grouping is still reversible (raw candidates are
  untouched, and nothing stops running synthesis again).

## Exit criteria (evidence-checkable)

| Invariant | Evidence check id |
|---|---|
| `candidate_synthesis_groups` exists, insert-only (rejects UPDATE/DELETE) | `synthesis-table-is-insert-only` |
| `synthesize_candidates_for_source` groups accepted candidates from one source into synthesized statements, each naming its member candidate ids | `synthesize-groups-candidates-with-members` |
| A candidate not grouped with any other becomes its own one-member group, never dropped | `synthesize-never-drops-a-candidate` |
| No API route or frontend change lands in this record (deferred, named honestly) | `no-route-or-frontend-change-this-record` |
