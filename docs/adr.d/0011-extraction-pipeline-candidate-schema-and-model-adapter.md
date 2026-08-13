# ADR-0011: Extraction pipeline — candidate schema, deterministic validation, and an optional model adapter

- **Status:** Accepted
- **Date:** 2026-08-14
- **Decider:** monk-eee
- **Approval:** Continuation of accepted [docs/PRODUCT-SPEC.md](../PRODUCT-SPEC.md) Epic E4 under "i want it", 2026-08-14
- **Depends on:** [ADR-0005](0005-adopt-rust-event-sourced-postgres-commitment-graph.md), [ADR-0010](0010-transcript-ingestion-parsing-chunking-provenance.md)
- **Tags:** architecture, data-model, extraction, model-adapter

## Context

[docs/PRODUCT-SPEC.md § 6.2](../PRODUCT-SPEC.md#62-extraction-pipeline)
(steps 4–9) and [§ 6.3](../PRODUCT-SPEC.md#63-extraction-object-contract)
describe extracting typed candidates (commitment, request, risk, follow_up,
decision, expectation) from source fragments, each with owner, time,
source, confidence, and a `requires_validation` flag. [§ 6.4](../PRODUCT-SPEC.md#64-validation-states)
names a validation-state lifecycle (Candidate, Accepted, Corrected,
Rejected, Superseded, Observed complete, Closed), and § 12's "Correct
memory" user story requires that "corrections preserve previous values and
provenance." Epic E4 names the deliverable as "structured candidate schema,
model adapter, prompts, deterministic validation and confidence."

"Corrections preserve previous values and provenance" is, structurally, the
same guarantee [ADR-0005](0005-adopt-rust-event-sourced-postgres-commitment-graph.md)
already built for Obligation: an immutable event log plus a derived,
rebuildable projection. Reusing that pattern here is a direct fit, not an
adaptation.

The "model adapter" needs a real design position on how an LLM is invoked.
MindLeak — installed in this same repository — already establishes a
precedent for exactly this shape of decision: an optional, OpenAI-compatible
endpoint, configured through environment variables, that "error[s] cleanly
when no server is reachable" and never blocks the deterministic path. This
ADR adopts that same posture rather than inventing a new one, consistent
with [docs/PRODUCT-SPEC.md § 3.1](../PRODUCT-SPEC.md#31-goals) ("earn trust
before taking actions") and its non-goal that Ringmaster must not be "a
system that silently treats model extraction as fact."

## Decision

- A `candidate_events` table is the immutable, append-only log for
  extraction candidates, enforced the same way as `obligation_events`
  ([ADR-0005](0005-adopt-rust-event-sourced-postgres-commitment-graph.md)/[ADR-0007](0007-generalize-obligation-and-require-pgvector.md)):
  the database rejects `UPDATE`/`DELETE`. Event payloads carry the §6.3
  extraction-object shape (`candidate_type`, `statement`, owner/counterparty,
  time fields, `source_fragment_id`, `extraction_model`, `confidence`,
  `requires_validation`) plus the §6.4 lifecycle transition being recorded
  (`extracted`, `accepted`, `corrected`, `rejected`, `superseded`,
  `observed_complete`, `closed`).
- A `candidate_projection` table is fully derived and rebuilt from
  `candidate_events`, the same way `obligation_projection` is — never
  patched in place, never authoritative over the log.
- Deterministic validation runs before any event is appended:
  `candidate_type` must be one of the six named types; `confidence` must
  fall in `[0.0, 1.0]`. Invalid payloads are rejected before they reach the
  log, mirroring how `obligation.rs` already validates lifecycle payloads.
- A `model_adapter` Rust module calls an OpenAI-compatible chat-completion
  endpoint, configured via `RINGMASTER_LLM_URL` / `RINGMASTER_MODEL`
  (naming deliberately parallel to MindLeak's `MINDLEAK_LLM_URL` /
  `MINDLEAK_MODEL`). When unconfigured or unreachable, it returns a typed
  error; it never panics and never blocks transcript ingestion or storage,
  which remain fully functional with no model configured.
- The prompt sent to the model, and the actual extraction of candidates
  from a real, running LLM, are provisional and untested against a live
  endpoint in this change — no local model is configured in this
  environment, the same honest gap [ADR-0007](0007-generalize-obligation-and-require-pgvector.md)
  left for embeddings.

## Scope

**In scope:** `candidate_events` / `candidate_projection` schema and
immutability; deterministic validation of `candidate_type` and
`confidence`; a model-adapter module with graceful, non-blocking
degradation when no model is configured.

**Out of scope:** deduplication/entity resolution against existing
obligations (§6.2 steps 6–7); the human validation queue/UI (Epic E5);
updating the attention horizon (Epic E7); final prompt design; and any
tested, live round-trip against a real running model.

## Options considered

- **Event-sourced candidates, reusing the Obligation pattern (chosen):**
  directly satisfies "corrections preserve previous values and provenance"
  and keeps the repository's storage model consistent rather than
  introducing a second, different persistence shape for a very similar
  problem.
- **Mutable `candidates` row with a separate correction-history table:**
  would work, but duplicates a problem this repository already solved once,
  for no material benefit.
- **Adopt MindLeak's own optional-model precedent for the adapter (chosen):**
  a proven, already-precedented, low-risk design already running in a
  closely related project, rather than inventing a new integration style.
- **Require a configured model before any extraction code exists:**
  simpler, but blocks deterministic schema, validation, and immutability
  work that does not actually depend on which model is chosen — the same
  reasoning ADR-0007 and ADR-0010 already applied to their own pipelines.

## Consequences

- **Positive:** extraction candidates get the same tamper-evident,
  fully-derivable history Obligations already have; the model adapter fails
  safely and predictably when unconfigured, matching an already-proven
  pattern in this environment.
- **Negative / trade-off:** no real extraction has been exercised against a
  live model; the prompt and the model's actual output shape remain
  unverified until one is configured.
- **Risk:** an untested prompt could produce candidates that pass schema
  validation but are substantively wrong. Mitigated by keeping
  `requires_validation` and the full `Candidate` → `Accepted`/`Corrected`
  lifecycle intact — nothing here treats extraction output as fact.

## Exit criteria and evidence

Evidence: [EV-0011](../evidence.d/0011-extraction-pipeline-candidate-schema-and-model-adapter.md)

| Exit criterion | Evidence |
|---|---|
| Candidate events are appended immutably; the database rejects mutation or deletion | `candidate-events-are-immutable` |
| The projection is fully derived and rebuilt from the event log | `candidate-projection-is-derived` |
| Invalid `candidate_type` or out-of-range `confidence` is rejected before reaching the log | `deterministic-validation-function-exists` |
| The model adapter degrades cleanly, without panicking, when no model is configured | `model-adapter-function-exists` |
