# ADR-0024: Accept/reject buttons for candidates — Epic E5's first interactive slice

- **Status:** Accepted
- **Date:** 2026-08-14
- **Decider:** monk-eee
- **Approval:** Direct instruction ("Accept as drafted"), 2026-08-14
- **Depends on:** [ADR-0011](0011-extraction-pipeline-candidate-schema-and-model-adapter.md), [ADR-0013](0013-http-endpoints-trigger-and-list-extraction-candidates.md), [ADR-0014](0014-react-vite-single-page-app.md)
- **Tags:** architecture, api, frontend, validation-ui

## Context

Every ADR shipped so far in Epic E4/E6/E7 (extraction, embeddings, search,
due dates, evidence-linking) has been read-only: a table, a query, a
ranked list. `extraction::transition_candidate` has existed since
[ADR-0011](0011-extraction-pipeline-candidate-schema-and-model-adapter.md)
and already appends `accepted`/`rejected`/`corrected`/`superseded`/
`observed_complete`/`closed` events with full provenance — but nothing
calls it. No HTTP route exposes it, and `CandidatesTable.tsx` renders a
plain table with zero buttons. Epic E5 ("Validation UI — meeting review
queue, evidence panel, accept/correct/reject/merge controls") remains
entirely unbuilt, and a person using this product today cannot make it
*do* anything — only look at it. This ADR is the smallest real slice of
E5: two buttons that actually work, not the full queue/evidence-panel
epic.

## Decision

- `POST /api/candidates/:id/accept` and `POST /api/candidates/:id/reject`
  each call `extraction::transition_candidate` with event type
  `"accepted"`/`"rejected"` and an empty payload (no field changes — a
  plain accept/reject, not a correction), then rebuild the candidate
  projection and return the updated row. Responses:
  - `200` with the updated candidate row.
  - `404` when the candidate id doesn't exist in the projection.
  - `409` when the candidate is not currently in the `candidate`
    validation state (an already-accepted/rejected/etc. candidate cannot
    be accepted/rejected again through this route) — prevents a stale UI
    from silently double-transitioning something already resolved.
- `CandidatesTable.tsx` renders an "Accept"/"Reject" button pair in each
  row whose `validation_state` is `candidate`; rows in any other state
  render their state as plain text, no buttons. Clicking a button calls
  the new route, then re-fetches the candidates list (the same manual
  refresh pattern `App.tsx` already uses elsewhere) so the table reflects
  the new state immediately.
- No "correct" (edit a field before accepting) or "merge" control is
  added here — those need their own design (what fields are editable, how
  a merge target is chosen) and are explicitly deferred.

## Scope

**In scope:** the two transition routes; button rendering and the
click-to-refresh interaction in `CandidatesTable.tsx`.

**Out of scope:** correcting/editing a candidate's fields before
accepting; merging a candidate into an existing Obligation (the
Candidate → Obligation promotion workflow itself is separate, larger,
undecided work); a dedicated review queue/evidence panel view (Epic E5's
full scope); undo; bulk actions; any change to `transition_candidate`
itself, which already does exactly what this ADR needs.

## Options considered

- **Two narrow HTTP routes plus inline table buttons (chosen):** the
  smallest change that makes the product genuinely interactive today,
  reusing `transition_candidate` exactly as it already exists.
- **Build the full Epic E5 queue/evidence-panel view now:** rejected —
  that is real, separate UX design work (a dedicated review screen,
  layout, navigation) and would take materially longer than closing the
  immediate "there are no buttons at all" gap.
- **A single generic `POST /api/candidates/:id/transition` route taking
  an arbitrary event type:** rejected — accepting arbitrary event types
  (including `corrected`, which needs field-level input this ADR doesn't
  design) from the client is a bigger, vaguer surface than two named,
  narrow actions with a clear contract.

## Consequences

- **Positive:** for the first time, a person can click something in this
  product and see it durably change, closing the sharpest gap in the
  product so far — everything before this was read-only.
- **Negative / trade-off:** still no correct/merge/queue; a rejected or
  accepted candidate has no visible "undo" if clicked by mistake (though
  the underlying event log always preserves the full history regardless).
- **Risk:** low — `transition_candidate` and the append-only event log
  are already proven; the new routes are thin wrappers with one added
  state-conflict check (`409`).

## Exit criteria and evidence

Evidence: [EV-0024](../evidence.d/0024-candidate-accept-reject-buttons.md)

| Exit criterion | Evidence |
|---|---|
| A route accepts a candidate still in the `candidate` state and rejects an already-transitioned one with `409` | `accept-route-exists` |
| A route rejects a candidate the same way | `reject-route-exists` |
| The Candidates table renders working Accept/Reject buttons for candidates still in the `candidate` state | `candidates-table-has-action-buttons` |
