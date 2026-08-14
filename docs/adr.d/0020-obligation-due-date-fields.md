# ADR-0020: Add due-date fields to Obligation, the schema prerequisite for Epic E7

- **Status:** Accepted
- **Date:** 2026-08-14
- **Decider:** monk-eee
- **Approval:** Direct instruction ("accept and continue"), 2026-08-14
- **Depends on:** [ADR-0005](0005-adopt-rust-event-sourced-postgres-commitment-graph.md), [ADR-0007](0007-generalize-obligation-and-require-pgvector.md), [ADR-0015](0015-expose-source-fragment-traceability-on-candidates.md)
- **Tags:** architecture, data-model, obligation, attention-horizon

## Context

[docs/PRODUCT-SPEC.md §7](../PRODUCT-SPEC.md#7-attention-and-risk-engine)
names Epic E7 ("Attention horizon") as "Dates, staleness, recurrence,
unowned obligations and explainable risk signals," and §7.1's first listed
signal is "Date compression: a transition is due soon but no handover
evidence exists." None of this is computable today: `obligation_projection`
carries only `obligation_id`, `status`, and `updated_at` — no date of any
kind exists anywhere in the Obligation event/projection schema.

This mirrors exactly the sequencing [ADR-0018](0018-generate-and-store-source-fragment-embeddings.md)
and [ADR-0019](0019-semantic-search-over-source-fragments.md) already used
for Epic E6: the data the epic needs had to exist and be provably stored
before any signal/ranking logic built on top of it could be real rather
than speculative. This ADR is that same first step for E7 — carrying due
dates through the event log and projection — and deliberately builds none
of the actual risk-signal computation yet.

## Decision

- `created` and `status_changed` `obligation_events` payloads may optionally
  carry `hard_due_at` and/or `soft_due_at` (ISO 8601 timestamps, both
  nullable — most obligations may have neither, one, or both, per
  [docs/PRODUCT-SPEC.md §6.3](../PRODUCT-SPEC.md#63-extraction-object-contract)'s
  own `time` shape: `hard_due_at`, `soft_due_at`, distinguishing a firm
  deadline from an inferred/soft one).
- `obligation_projection` gains nullable `hard_due_at TIMESTAMPTZ` /
  `soft_due_at TIMESTAMPTZ` columns (migration
  `0009_obligation_due_dates.sql`).
- `rebuild_projection` carries a due date forward across later events the
  same way it already carries `statement`/`candidate_type` forward in
  `extraction.rs`'s candidate projection: only updated when the new event's
  payload actually names that field; otherwise the previously-recorded
  value is preserved. A `status_changed` event that only changes status
  does not silently erase a previously-recorded due date.
- `GET /api/obligations` gains `hard_due_at`/`soft_due_at` in its response
  — additive fields only, mirroring exactly how
  [ADR-0015](0015-expose-source-fragment-traceability-on-candidates.md)
  added `source_fragment_id`/`source_text`/`speaker` to
  `GET /api/candidates` without changing any existing field.
- No validation is added beyond "is it a parseable timestamp" — deciding
  *how* a date is inferred, and labelling that inference explicitly per
  [docs/PRODUCT-SPEC.md §6.2](../PRODUCT-SPEC.md#62-extraction-pipeline)
  step 8, is extraction-pipeline work, not this schema-only ADR's concern.

## Scope

**In scope:** the two nullable projection columns, the migration, carrying
due dates forward through projection rebuild, and the additive API
response fields.

**Out of scope:** any risk-signal computation (date compression,
staleness, or any other §7.1 signal); the attention horizon (7/30/60/90-
day) view or endpoint; recurrence; unowned-obligation detection; an
`owner`/ownership field (a separate, not-yet-decided data-model question);
inferring a date from extracted text (extraction-pipeline work, already
out of scope for [ADR-0011](0011-extraction-pipeline-candidate-schema-and-model-adapter.md));
surfacing dates in the frontend.

## Options considered

- **Two nullable projection columns, carried forward like extraction.rs's
  existing pattern (chosen):** the smallest schema addition that makes a
  due date provably storable and retrievable, reusing an already-proven
  carry-forward technique instead of inventing a new one.
- **A single `due_at` field instead of separate hard/soft dates:** rejected
  — §6.3's extraction contract and §7.2's "hard versus soft dates clearly
  distinguished" UX requirement both explicitly need the distinction;
  collapsing them now would need undoing later.
- **Build the full attention/risk engine in one ADR:** rejected — signal
  computation, scoring, and the horizon view are each real, separately
  debatable design questions; bundling them with the schema prerequisite
  repeats the mistake of deciding too much in one record instead of the
  small, provable slices this repository has consistently used.

## Consequences

- **Positive:** gives Epic E7 a real, queryable foundation; unblocks a
  future ADR to build actual risk signals against real (if still sparse)
  data instead of a hypothetical schema.
- **Negative / trade-off:** no signal or ranking exists yet — an obligation
  can now have a due date visible in the API with nothing yet computing or
  surfacing why it matters.
- **Risk:** none material — nullable, additive columns; existing rows and
  every current consumer of `obligation_projection`/`GET /api/obligations`
  are unaffected.

## Exit criteria and evidence

Evidence: [EV-0020](../evidence.d/0020-obligation-due-date-fields.md)

| Exit criterion | Evidence |
|---|---|
| `obligation_projection` carries nullable `hard_due_at`/`soft_due_at` columns | `due-date-columns-exist` |
| Projection rebuild carries a due date forward across events that don't name it | `rebuild-preserves-due-dates` |
| `GET /api/obligations` includes `hard_due_at`/`soft_due_at` for each row | `obligations-route-includes-due-dates` |
