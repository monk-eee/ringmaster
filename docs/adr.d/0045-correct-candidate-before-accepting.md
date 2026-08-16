# ADR-0045: Correct a candidate before accepting it

- **Status:** Accepted
- **Date:** 2026-08-17
- **Decider:** monk-eee
- **Approval:** Implemented following this session's established delegation pattern for well-scoped, low-risk proposals ("keep going"), 2026-08-17
- **Depends on:** [ADR-0011](0011-extraction-pipeline-candidate-schema-and-model-adapter.md), [ADR-0024](0024-candidate-accept-reject-buttons.md), [ADR-0027](0027-promote-accepted-candidate-to-obligation.md), [ADR-0038](0038-wire-up-audit-events-for-candidate-validation.md)
- **Tags:** architecture, api, frontend, validation-ui

## Context

[ADR-0024](0024-candidate-accept-reject-buttons.md) shipped plain
Accept/Reject, and named exactly what it deliberately left out: *"No
'correct' (edit a field before accepting) or 'merge' control is added
here — those need their own design... and are explicitly deferred."*
[PRODUCT-SPEC.md §6.4](../PRODUCT-SPEC.md#64-validation-states) lists
`Corrected` as its own validation state, distinct from `Accepted`:
*"Human changed type, owner, date, wording or linkage."*
`extraction::transition_candidate` and `rebuild_candidate_projection`
have supported a generic `"corrected"` event type since
[ADR-0011](0011-extraction-pipeline-candidate-schema-and-model-adapter.md)
— any transition event already applies an optional `statement`/
`candidate_type` override from its payload — but nothing has ever called
it with those fields set, and no UI has ever offered to edit one.

Of the four fields PRODUCT-SPEC names (type, owner, date, wording,
linkage), only two exist on `Candidate` today: `candidate_type` (type)
and `statement` (wording). `owner` and `date` are not fields on this
schema at all (no candidate has ever carried either), and "linkage" (to
an existing Obligation, distinct from promoting into a *new* one) is the
"merge" workflow ADR-0024 also named as separate, harder, undecided work.
Building owner/date correction or merge now would mean inventing schema
and semantics with no accepted design — this ADR builds only the two
fields that genuinely exist, and defers the rest by name, matching
ADR-0024's own precedent.

## Decision

- A new route, `POST /api/candidates/:id/correct`, accepting a JSON body
  with optional `candidate_type` and/or `statement`. Only a candidate
  currently in the `candidate` state may be corrected (`409` otherwise,
  mirroring accept/reject's own one-way semantics). `candidate_type`, if
  given, must be one of the same six allowed values `extract_candidate`
  already validates (`400` otherwise). At least one field must actually
  change from its current value, or the request is rejected (`400`) as a
  meaningless correction rather than silently accepted as a no-op event.
  On success, appends a `"corrected"` event carrying only the field(s)
  that changed (never the unchanged one, so the event payload states
  plainly what was actually corrected), in the same transaction as its
  [ADR-0038](0038-wire-up-audit-events-for-candidate-validation.md) audit
  row, then rebuilds the projection and returns the updated candidate row
  — the same transactional and response pattern accept/reject/promote
  already use.
- `POST /api/candidates/:id/promote` ([ADR-0027](0027-promote-accepted-candidate-to-obligation.md))
  now accepts a candidate in either the `accepted` **or** `corrected`
  state (previously `accepted` only) — both mean "a human has validated
  this is ready to become a real Obligation," and PRODUCT-SPEC's own
  table treats them as siblings, not a to-do list. This is the only
  change to promotion; nothing else about it changes.
- **Frontend:** `CandidatesTable.tsx` gains a "Correct" button alongside
  Accept/Reject for any `candidate`-state row, revealing an inline edit
  form (a `candidate_type` dropdown of the six allowed values, a
  `statement` textarea, "Save Correction"/"Cancel"). A `corrected`-state
  row renders the same "Promote to Obligation" button an `accepted` row
  already does, rather than the dead-end "—" every other terminal state
  shows.

## Scope

**In scope:** the `correct` route; the promotion guard's small extension;
the inline edit form and corrected-row promotion button in
`CandidatesTable.tsx`.

**Out of scope, named honestly (deferred, larger/separate work):**
correcting an `owner` or `date` — neither is a field on `Candidate` today;
adding either is a real, separate schema decision, not something to
smuggle into this ADR; "merge" (linking a candidate into an *existing*
Obligation instead of promoting into a new one) — ADR-0024 already named
this as needing its own design (how a merge target is chosen, what
merging changes on the existing Obligation) and nothing here changes
that; a full review-queue/evidence-panel experience (Epic E5's larger,
still-unbuilt scope); undo of a correction; bulk corrections.

## Options considered

- **A dedicated `correct` route, editing only the two fields that exist
  (chosen):** zero schema change, reuses `transition_candidate`'s already-
  proven generic field-override handling verbatim, ships the two-thirds
  of Epic E5's "accept/correct/reject" trio that ADR-0024 didn't.
- **Fold correction into `accept` via an optional request body instead of
  a separate route:** would mean one route serving two different
  validation-state outcomes (`accepted` vs `corrected`) depending only on
  whether a body was sent — a more surprising API shape for a real
  difference PRODUCT-SPEC itself tracks as separate states; rejected for
  the same "different response shapes deserve different routes" reasoning
  [ADR-0029](0029-time-horizon-view.md) already used.
- **Add `owner`/`date` fields now so all four PRODUCT-SPEC.md fields are
  correctable together:** rejected — no accepted ADR has ever added
  either field to `Candidate`, and inventing them here would be exactly
  the kind of unscoped schema change this repo's governance model exists
  to prevent.

## Consequences

- **Positive:** closes the two genuinely-buildable-today thirds of
  ADR-0024's named gap; zero schema change; zero new reasoning duplicated
  (reuses `transition_candidate`, the existing audit-transaction pattern,
  and the existing candidate-row JSON shape verbatim).
- **Negative / trade-off:** a corrected candidate's `owner`/`date`/
  linkage still cannot be fixed through this UI — only type and wording.
  "Merge" remains entirely unbuilt.
- **Risk:** low. Purely additive route and UI; no migration; the existing
  event-sourced projection rebuild already handles the new event type
  correctly today because it was built generically from the start.
