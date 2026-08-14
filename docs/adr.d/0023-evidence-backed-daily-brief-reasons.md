# ADR-0023: Evidence-backed Daily Brief reasons — source-fragment traceability on Obligation

- **Status:** Proposed
- **Date:** 2026-08-14
- **Decider:** monk-eee
- **Approval:** Pending — awaiting monk-eee's decision
- **Depends on:** [ADR-0010](0010-transcript-ingestion-parsing-chunking-provenance.md), [ADR-0015](0015-expose-source-fragment-traceability-on-candidates.md), [ADR-0020](0020-obligation-due-date-fields.md), [ADR-0022](0022-daily-brief-endpoint.md)
- **Tags:** architecture, data-model, api, attention-horizon

## Context

[ADR-0022](0022-daily-brief-endpoint.md) explicitly named its own biggest
honest gap: "evidence-backed reasons... need a link between an Obligation
and the source fragments/candidates that support it. No such link exists
in the schema today." [VISION.md § The Daily Brief](../VISION.md#the-daily-brief)'s
own mockup depends on exactly this: "Transition Plan — Roopa expectation.
Due in 8 days. **No evidence recorded.**" The due-date half of that
sentence is real today; the evidence half is not.

[ADR-0015](0015-expose-source-fragment-traceability-on-candidates.md)
already solved the identical problem once, for Candidate instead of
Obligation: a nullable `source_fragment_id` carried in the event payload,
projected forward, and joined read-only against the already-immutable
`source_fragments` table at query time. This ADR applies that exact same,
already-proven pattern to Obligation.

## Decision

- `created` and `status_changed` `obligation_events` payloads may
  optionally carry `source_fragment_id` (a `source_fragments.id`), the
  same way [ADR-0020](0020-obligation-due-date-fields.md) already added
  optional `hard_due_at`/`soft_due_at` to the same two event types.
- `obligation_projection` gains a nullable `source_fragment_id UUID`
  column (migration `0010_obligation_source_fragment.sql`). `rebuild_projection`
  carries it forward across later events that don't name it, using the
  exact same carry-forward logic
  [ADR-0020](0020-obligation-due-date-fields.md)'s `payload_timestamp`
  helper already established for due dates — a status_changed event that
  doesn't mention `source_fragment_id` never erases a previously-recorded
  one.
- `GET /api/obligations` performs a read-only `LEFT JOIN` against
  `source_fragments` (mirroring
  [ADR-0015](0015-expose-source-fragment-traceability-on-candidates.md)'s
  `GET /api/candidates` treatment exactly) and adds `source_fragment_id`
  and `source_text` to each row — additive only.
- `GET /api/daily-brief`'s `reason` string gains a second clause appended
  to the existing due-date clause: `"Last evidence: \"<source_text,
  truncated to 80 characters>\"."` when a source fragment is linked, or
  `"No evidence recorded."` when it isn't — matching
  [VISION.md](../VISION.md#the-daily-brief)'s own wording exactly. The
  due-date clause (`"Marked at risk."` / `"Overdue by N day(s)."` / etc.)
  is unchanged; this only adds the second sentence.

## Scope

**In scope:** the `source_fragment_id` projection column and migration;
carrying it forward through rebuild; the additive
`GET /api/obligations` fields; the second clause on
`GET /api/daily-brief`'s `reason`.

**Out of scope:** any UI/workflow for actually *linking* an Obligation to
a fragment (there is no accept/promote-a-Candidate-into-an-Obligation
flow yet — that is Epic E5, the Validation UI, still unbuilt); Congruence
grouping (needs graph traversal across shared people/services/meetings,
which nothing populates yet); citing more than one fragment per
Obligation; any change to Candidate's own, already-shipped evidence
fields.

## Options considered

- **Mirror ADR-0015's exact pattern, applied to Obligation (chosen):** a
  proven, already-shipped design in this codebase, reused rather than
  reinvented, for the structurally identical problem.
- **Wait for Epic E5's validation/promotion flow to exist, then derive
  evidence through that instead:** rejected for now — E5 is a
  substantially larger, unbuilt feature (queue UI, accept/correct/reject/
  merge controls); this ADR does not need to wait for it to close
  ADR-0022's specific named gap using data that can already be recorded
  directly.
- **Link Obligation to a Candidate instead of directly to a
  source_fragment:** rejected — no schema or workflow currently connects
  Obligation and Candidate at all; introducing that relationship is a
  larger, separate data-model decision than citing the same underlying
  evidence source both aggregates already reference identically.

## Consequences

- **Positive:** closes ADR-0022's own explicitly-named gap; the Daily
  Brief's reason text now matches the vision's own worked example
  ("No evidence recorded.") using real, provable data.
- **Negative / trade-off:** an Obligation still has no *automatic* way to
  acquire a `source_fragment_id` — it must be set explicitly when the
  event is appended, since no promotion workflow exists yet. Every
  existing Obligation will show "No evidence recorded." until one is
  set.
- **Risk:** none material — nullable, additive columns and fields; no
  existing consumer's response shape changes.

## Exit criteria and evidence

Evidence: [EV-0023](../evidence.d/0023-evidence-backed-daily-brief-reasons.md)

| Exit criterion | Evidence |
|---|---|
| `obligation_projection` carries a nullable `source_fragment_id`, preserved across events that don't name it | `obligation-source-fragment-id-preserved` |
| `GET /api/obligations` includes `source_fragment_id`/`source_text` | `obligations-route-includes-source-fields` |
| `GET /api/daily-brief`'s `reason` cites the linked evidence, or states none is recorded | `daily-brief-reason-cites-evidence` |
