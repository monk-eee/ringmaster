# ADR-0013: HTTP endpoints trigger and list model-based extraction candidates

- **Status:** Accepted
- **Date:** 2026-08-14
- **Decider:** monk-eee
- **Approval:** Direct instruction ("Accept as drafted"), 2026-08-14
- **Depends on:** [ADR-0011](0011-extraction-pipeline-candidate-schema-and-model-adapter.md), [ADR-0012](0012-minimal-http-api-and-node-web-front-end.md)
- **Tags:** architecture, api, extraction

## Context

[ADR-0011](0011-extraction-pipeline-candidate-schema-and-model-adapter.md)
built `candidate_events`/`candidate_projection`, deterministic validation, and
a `model_adapter` module reachable via `extract_candidate_via_model`, but left
"the actual extraction of candidates from a real, running LLM ... provisional
and untested against a live endpoint" as an explicitly named gap. That gap has
since closed: with `RINGMASTER_LLM_URL`/`RINGMASTER_MODEL` now configured in
`compose.yaml` against a local Ollama server,
`extraction::tests::extract_candidate_via_model_round_trips_against_a_live_endpoint_when_configured`
passes end-to-end against a real model.

Nothing outside that test calls `extract_candidate_via_model`, however.
`main.rs` only migrates, rebuilds the Obligation projection, and serves
[ADR-0012](0012-minimal-http-api-and-node-web-front-end.md)'s two read-only
routes. [docs/PRODUCT-SPEC.md §6.2](../PRODUCT-SPEC.md#62-extraction-pipeline)
describes extraction (step 4) as following ingestion (steps 1-3); Epic E4's
deliverable — schema, adapter, prompts, validation — is now fully built, but
nothing in the running system can actually invoke it against a real source
fragment. Epic E5 ("Validation UI" — meeting review queue, evidence panel,
accept/correct/reject/merge controls) remains unbuilt and is not proposed
here; a human or script still needs some way to produce a Candidate before
there is anything for a future validation UI to review.

## Decision

- `POST /api/source-fragments/:id/extract` calls
  `extraction::extract_candidate_via_model` for exactly the named
  `source_fragment_id`, using the same `ModelConfig::from_env()` as
  [ADR-0011](0011-extraction-pipeline-candidate-schema-and-model-adapter.md).
  It is a deliberate, synchronous, explicit trigger — never automatic on
  ingestion — so ingestion keeps the non-blocking guarantee ADR-0011 already
  established for the model adapter. Responses:
  - `201` with the created candidate's projection row when the model
    extracted something.
  - `204` when the model reported nothing worth extracting.
  - `503` with a typed error body when no model is configured or the model
    is unreachable — the route never blocks or panics on a missing model,
    mirroring ADR-0011's own "never panics, never blocks" posture for the
    adapter itself.
  - `404` when `source_fragment_id` does not exist.
- `GET /api/candidates` returns current `candidate_projection` rows as JSON
  (`candidate_id`, `candidate_type`, `statement`, `validation_state`,
  `confidence`), mirroring
  [ADR-0012](0012-minimal-http-api-and-node-web-front-end.md)'s
  `GET /api/obligations` exactly: read-only, no write, a direct projection
  read.
- Calling the trigger route twice on the same fragment appends two separate
  `extracted` events; there is no idempotency guard. This mirrors ADR-0011's
  own already-accepted deferral of deduplication
  ([docs/PRODUCT-SPEC.md §6.2](../PRODUCT-SPEC.md#62-extraction-pipeline)
  steps 6-7) rather than inventing a new guarantee here.

## Scope

**In scope:** the one trigger route and the one read route named above;
translating the model adapter's existing typed errors into HTTP statuses
instead of swallowing or panicking on them.

**Out of scope:** Epic E5's validation UI/queue and any accept/correct/
reject/merge controls; automatic extraction on ingestion; deduplication or
entity resolution against existing obligations; batching multiple fragments
in one call; authentication (already deferred by
[ADR-0004](0004-defer-multi-user-access-control-single-user-v1.md)); any
change to the extraction prompt or the model adapter itself.

## Options considered

- **Explicit synchronous HTTP trigger plus a companion read route (chosen):**
  the smallest addition consistent with ADR-0012's already-accepted "minimal
  HTTP API" precedent; keeps the model adapter's non-blocking guarantee
  intact because extraction is never implicitly invoked; requires no new
  infrastructure.
- **Automatic extraction inside `ingest_transcript`:** matches the product
  spec's step-ordered pipeline description most literally, but ties
  ingestion latency to an LLM call and would either violate ADR-0011's "never
  blocks ingestion" guarantee or require a background job queue — a
  materially larger, undecided infrastructure question this record does not
  open.
- **A one-off startup hook in `main.rs` that extracts all pending fragments
  once at boot:** avoids a new HTTP route entirely, but doesn't fit the
  HTTP-surfaced interface [ADR-0012](0012-minimal-http-api-and-node-web-front-end.md)
  already established, and is harder to exercise deliberately or test against
  one specific fragment.

## Consequences

- **Positive:** for the first time, a person or script can make the running
  system actually produce an extraction candidate from a real fragment and
  see it, closing the last "built but unreachable" piece of Epic E4.
- **Negative / trade-off:** the API gains its first write/mutation route,
  which [ADR-0012](0012-minimal-http-api-and-node-web-front-end.md) explicitly
  scoped out; this record is what accepts that trade-off.
- **Risk:** repeated calls on the same fragment silently create duplicate
  candidates with no idempotency guard. Mitigated by candidates always
  remaining in the `Candidate` validation state until a human (still Epic E5,
  still unbuilt) reviews them — nothing here treats extraction output as fact
  or auto-accepts it.

## Exit criteria and evidence

Evidence: [EV-0013](../evidence.d/0013-http-endpoints-trigger-and-list-extraction-candidates.md)

| Exit criterion | Evidence |
|---|---|
| A route triggers extraction for one named source fragment and returns a typed, non-panicking result for every model-adapter outcome | `extract-route-exists` |
| A read-only route lists current candidate_projection rows | `candidates-route-exists` |
