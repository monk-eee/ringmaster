# ADR-0037: Meeting-scoped candidate listing and extraction progress

- **Status:** Accepted
- **Date:** 2026-08-14
- **Decider:** monk-eee
- **Approval:** Direct instruction ("create adr and action the additional work"), 2026-08-14
- **Depends on:** [ADR-0011](0011-extraction-pipeline-candidate-schema-and-model-adapter.md), [ADR-0013](0013-http-endpoints-trigger-and-list-extraction-candidates.md), [ADR-0036](0036-meeting-detail-read.md)
- **Tags:** architecture, api, meeting, extraction, candidate

## Context

[ADR-0036](0036-meeting-detail-read.md) added `GET /api/meetings/:id`,
returning a meeting and its ordered transcript fragments, and explicitly
deferred "listing or counting candidates already extracted per fragment" as
this ADR's own named next slice.
[MEETING-REVIEW-DESIGN.md](../MEETING-REVIEW-DESIGN.md) lists that slice
third, directly after ingestion and meeting detail, and its meeting header
mockup depends on it: "7 proposals / 4 unreviewed | 2 accepted | 1
deferred" cannot be computed today for one specific meeting.

`GET /api/candidates` ([ADR-0013](0013-http-endpoints-trigger-and-list-extraction-candidates.md))
already lists every candidate in the system with its
`validation_state`, but globally, not scoped to a meeting, and with no
indication of which of a meeting's fragments have not been extracted at
all yet (no candidate row exists for them). Building the meeting review
workspace this design document describes is not possible without a
meeting-scoped read that shows both what has been extracted and what
has not.

`candidate_projection.validation_state` today takes one of `candidate`
(the initial, unreviewed state), `accepted`, `rejected`, `corrected`,
`superseded`, `observed_complete`, `closed`, or `promoted`
([ADR-0011](0011-extraction-pipeline-candidate-schema-and-model-adapter.md)).
There is no `deferred` state; the design document's mockup use of
"deferred" is design intent for a later slice
([MEETING-REVIEW-DESIGN.md](../MEETING-REVIEW-DESIGN.md)'s own "Defer"
review action), not something this ADR invents or claims to satisfy.

## Decision

- Add `GET /api/meetings/:id/candidates`. Reuses the same 404 contract as
  [ADR-0036](0036-meeting-detail-read.md): 404 for an unknown id or a node
  whose `node_type` is not `"meeting"`.
- The route reads the meeting's fragments (the same `source_fragments`
  rows [ADR-0036](0036-meeting-detail-read.md) orders by
  `sequence ASC NULLS LAST, created_at ASC, id ASC`), left-joined against
  `candidate_projection` on `source_fragment_id`. A fragment with no
  candidate yet appears with an empty `candidates` array rather than being
  silently omitted — extraction progress requires seeing what is still
  pending, not only what already happened.
- A fragment can have more than one candidate (nothing in
  [ADR-0011](0011-extraction-pipeline-candidate-schema-and-model-adapter.md)/[ADR-0013](0013-http-endpoints-trigger-and-list-extraction-candidates.md)
  prevents re-running extraction on the same fragment), so `candidates` is
  always an array, never a single nullable object, an honest reflection of
  what the data model actually allows today.
- The response is:

  ```json
  {
    "meeting_id": "...",
    "fragments": [
      {
        "fragment_id": "...",
        "sequence": 0,
        "speaker": "Roopa",
        "text": "Can you bring me a transition plan next Friday?",
        "candidates": [
          { "candidate_id": "...", "candidate_type": "request", "statement": "...", "validation_state": "candidate", "confidence": 0.82 }
        ]
      }
    ],
    "progress": {
      "fragment_count": 5,
      "extracted_fragment_count": 3,
      "pending_fragment_count": 2,
      "by_validation_state": { "candidate": 2, "accepted": 1 }
    }
  }
  ```

  `progress` counts fragments (not candidates) for `fragment_count`/
  `extracted_fragment_count`/`pending_fragment_count`;
  `by_validation_state` counts candidates, keyed only by states that
  actually occur among this meeting's candidates.
- Read-only. No new table, column, candidate state, or write path. No
  route triggers extraction; a caller still uses the existing
  `POST /api/source-fragments/:id/extract`
  ([ADR-0013](0013-http-endpoints-trigger-and-list-extraction-candidates.md))
  per fragment.

## Scope

**In scope:** the `GET /api/meetings/:id/candidates` route; its 404
contract; per-fragment candidate listing; meeting-scoped progress counts.

**Out of scope, named honestly (later slices of
[MEETING-REVIEW-DESIGN.md](../MEETING-REVIEW-DESIGN.md)):** any frontend
meeting-review screen or transcript/proposal synchronization (slice 4
onward); a `deferred` candidate state or any other new lifecycle state;
correction, merge, split semantics; claim bundles spanning multiple
nodes/edges; triggering extraction from this route; pagination for very
large meetings.

## Options considered

- **A dedicated `GET /api/meetings/:id/candidates` route (chosen):** keeps
  [ADR-0036](0036-meeting-detail-read.md)'s meeting-detail response
  unchanged and focused on the transcript, mirrors that ADR's own reasoning
  for a dedicated route over overloading a generic one, and matches this
  design document's slice boundary exactly.
- **Fold candidates into `GET /api/meetings/:id`'s response:** rejected —
  would change an already-Accepted, already-implemented response shape for
  a second, different concern (extraction status vs. transcript content),
  repeating the objection [ADR-0036](0036-meeting-detail-read.md) itself
  raised against overloading `GET /api/nodes/:id`.
- **Reuse `GET /api/candidates` with a `meeting_id` query parameter:**
  rejected — that route has no notion of "fragments with zero candidates";
  adding one would materially change its existing, already-proven
  contract for every other caller, and still would not show pending
  (not-yet-extracted) fragments without further changes.
- **Invent a `deferred` validation state now to match the design
  document's mockup:** rejected — a new candidate lifecycle state changes
  the event vocabulary [ADR-0011](0011-extraction-pipeline-candidate-schema-and-model-adapter.md)
  established and needs its own bounded decision alongside the "Defer"
  review action, not a side effect of a read-only listing route.

## Consequences

- **Positive:** a caller can finally see, per meeting, both what has been
  extracted and what has not — the data this ADR's design document
  requires for its review-workspace header and default review ordering.
- **Positive:** keeps [ADR-0036](0036-meeting-detail-read.md)'s response
  shape stable; nothing that already depends on it changes.
- **Negative / trade-off:** `candidates` as an array (rather than a single
  nullable object) is slightly more work for a caller expecting exactly
  one candidate per fragment, but avoids hiding a real, already-possible
  multi-candidate case.
- **Risk:** low. One new read-only route; no schema, migration, or
  existing-route change.

## Exit criteria and evidence

Evidence: [EV-0037](../evidence.d/0037-meeting-scoped-candidate-listing.md)

| Exit criterion | Evidence |
|---|---|
| `GET /api/meetings/:id/candidates` lists every fragment, including those with zero candidates | `meeting-candidates-route-lists-all-fragments` |
| A fragment's extracted candidates appear with their real validation state | `meeting-candidates-route-includes-candidate-state` |
| `progress` counts fragments (extracted/pending), not candidates | `meeting-candidates-route-computes-fragment-progress` |
| The route 404s for an unknown id or a non-meeting node | `meeting-candidates-route-404s-for-non-meeting` |
| No extraction is triggered by this route | `meeting-candidates-route-never-triggers-extraction` |
