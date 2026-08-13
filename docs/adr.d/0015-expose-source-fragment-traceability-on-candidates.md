# ADR-0015: Expose source-fragment traceability on candidates

- **Status:** Proposed
- **Date:** 2026-08-14
- **Decider:** monk-eee
- **Approval:** Pending — awaiting monk-eee's decision
- **Depends on:** [ADR-0010](0010-transcript-ingestion-parsing-chunking-provenance.md), [ADR-0011](0011-extraction-pipeline-candidate-schema-and-model-adapter.md), [ADR-0013](0013-http-endpoints-trigger-and-list-extraction-candidates.md)
- **Tags:** architecture, api, extraction, data-model

## Context

[docs/PRODUCT-SPEC.md §12](../PRODUCT-SPEC.md#12-mvp-user-stories-and-acceptance-criteria)
names "Validate an ask" as an MVP acceptance criterion: "the user can trace
an extracted request back to the exact transcript segment and accept,
correct or reject it." [§6.3](../PRODUCT-SPEC.md#63-extraction-object-contract)'s
extraction object carries `source.quote`/`speaker` alongside the candidate
itself for exactly this reason. Epic E5 ("Validation UI — meeting review
queue, evidence panel, accept/correct/reject/merge controls") is the epic
that needs this, and remains unbuilt and unproposed.

The pieces already exist but aren't connected. [ADR-0011](0011-extraction-pipeline-candidate-schema-and-model-adapter.md)'s
`extract_candidate` already writes `source_fragment_id` into every
`extracted` event's payload, and [ADR-0010](0010-transcript-ingestion-parsing-chunking-provenance.md)'s
`source_fragments` table already stores each fragment's immutable `text`
and `speaker`. But `rebuild_candidate_projection` drops `source_fragment_id`
when it derives `candidate_projection` from the event log, and
[ADR-0013](0013-http-endpoints-trigger-and-list-extraction-candidates.md)'s
`GET /api/candidates` has no way to surface it. A future Validation UI
cannot show "which transcript passage produced this candidate" until that
gap closes — this ADR closes only that gap, not the UI itself.

## Decision

- `candidate_projection` gains a nullable `source_fragment_id UUID` column
  (migration `0007_candidate_projection_source_fragment.sql`), populated by
  `rebuild_candidate_projection` by reading the field that already exists in
  each candidate's `extracted` event payload. Nullable because a candidate
  created before this column existed, or without a source fragment, must
  not fail projection rebuild.
- `GET /api/candidates` performs a read-only `LEFT JOIN` against
  `source_fragments` and adds `source_fragment_id`, `source_text`, and
  `speaker` to each row in the response — additive fields only; every
  field [ADR-0013](0013-http-endpoints-trigger-and-list-extraction-candidates.md)
  already returns is unchanged.
- No new route is added. `POST /api/source-fragments/:id/extract`'s
  behavior and response are unchanged.

## Scope

**In scope:** the `candidate_projection` column, its migration, the
projection-rebuild change that populates it, and the additive
`GET /api/candidates` response fields.

**Out of scope:** Epic E5's actual validation UI, queue, or accept/
correct/reject/merge controls; the `owner`/`counterparty`/`time` fields
from the full §6.3 extraction-object contract (still deferred, same as
[ADR-0011](0011-extraction-pipeline-candidate-schema-and-model-adapter.md)
left them); real `start_ms`/`end_ms` population (still blocked on
[ADR-0010](0010-transcript-ingestion-parsing-chunking-provenance.md)'s
named placeholder-transcript-format gap); any change to transcript
ingestion itself.

## Options considered

- **Read-time `LEFT JOIN` in the API route (chosen):** keeps the
  append-only projection's job (candidate/validation state) separate from
  source-fragment lookup. `source_fragments` is already immutable
  ([ADR-0010](0010-transcript-ingestion-parsing-chunking-provenance.md)), so
  a join can never return a quote that has silently changed underneath a
  candidate.
- **Denormalize `text`/`speaker` directly into `candidate_projection` at
  rebuild time:** avoids a join on every read, but duplicates data that's
  already available by id, for no benefit at this data volume, and
  reintroduces a copy that could (in principle) drift from the immutable
  source if the join logic and the denormalization logic ever disagreed.
- **Add a separate `GET /api/source-fragments/:id` read route instead:**
  would work, but forces a future UI into two round-trips to show one
  candidate's evidence, and doesn't fit this ADR's narrower goal of making
  `GET /api/candidates` alone sufficient for an evidence panel.

## Consequences

- **Positive:** `GET /api/candidates` alone becomes sufficient to build
  Epic E5's evidence panel later; no second read route is needed yet.
- **Negative / trade-off:** candidates with no recorded
  `source_fragment_id` return `null` evidence fields; the response schema
  gains new fields (additive; should not break a consumer reading only the
  fields [ADR-0013](0013-http-endpoints-trigger-and-list-extraction-candidates.md)
  already named).
- **Risk:** minimal — a read-only, additive change to an already read-only
  route, over an already-immutable source table.

## Exit criteria and evidence

Evidence: [EV-0015](../evidence.d/0015-expose-source-fragment-traceability-on-candidates.md)

| Exit criterion | Evidence |
|---|---|
| `candidate_projection` carries `source_fragment_id`, populated by the projection rebuild | `source-fragment-id-column-exists` |
| `GET /api/candidates` includes `source_fragment_id`, `source_text`, and `speaker` for each row | `candidates-route-includes-source-fields` |
