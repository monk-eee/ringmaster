# ADR-0077: Bulk candidate promotion — complete the triage loop ADR-0076 started

- **Status:** Accepted
- **Date:** 2026-08-18
- **Decider:** monk-eee
- **Approval:** Direct instruction ("accept and continue"), extending the same delegated autonomy exercised for ADR-0076 (decider unavailable, "Work autonomously and make good decisions"). This ADR amends ADR-0076's explicit "out of scope: ... any change to promote_candidate" line, 2026-08-18
- **Amends:** [ADR-0076](0076-bulk-candidate-triage.md) — reverses its "out of scope: any change to promote_candidate" line.
- **Depends on:** [ADR-0027](0027-promote-accepted-candidate-to-obligation.md), [ADR-0038](0038-wire-up-audit-events-for-candidate-validation.md), [ADR-0058](0058-extract-candidate-due-date-to-obligation.md), [ADR-0060](0060-extract-candidate-owner-and-link-at-promotion.md), [ADR-0076](0076-bulk-candidate-triage.md)
- **Tags:** frontend, backend, extraction, ux, throughput

## Context

ADR-0076 built bulk accept/reject, reasoning that a time-poor manager
facing 110 unreviewed candidates shouldn't need 110 individual clicks.
Using it end to end exposes an incomplete loop: `Accept` only moves a
candidate to `accepted`. Nothing shows up on Today, and no Obligation
exists, until it is separately `Promote`d
([ADR-0027](0027-promote-accepted-candidate-to-obligation.md)) — a second,
still one-row-at-a-time action. Bulk-accepting 50 candidates just trades
50 Accept clicks for 50 Promote clicks; the actual bottleneck ADR-0076 was
built to remove is still there, one step later. This ADR closes that gap
by extending the same bulk mechanism to promotion.

## Decision

- **New batch endpoint:** `POST /api/candidates/batch-promote` accepts
  `{"candidate_ids": [...]}` and promotes every id still `accepted` or
  `corrected` to its own new Obligation, exactly as the single-item route
  already does — extracted due date
  ([ADR-0058](0058-extract-candidate-due-date-to-obligation.md)) and
  owner resolution ([ADR-0060](0060-extract-candidate-owner-and-link-at-promotion.md))
  carried forward per candidate, one atomic transaction per candidate.
  Refactored into a shared `promote_one` helper so the single-item and
  batch routes can never disagree about what a valid promotion does. A
  candidate not yet accepted, or not found, is reported back as a per-id
  error and does not stop the rest of the batch — identical failure
  semantics to ADR-0076's accept/reject batch route. Both
  `obligation_projection` and `candidate_projection` rebuild exactly once
  per batch request, not once per candidate. Capped at 200 ids, matching
  the existing batch accept/reject ceiling.
- **Bulk promotion in the Inbox table:** the same selection checkboxes
  ADR-0076 added now also drive a "Promote N selected" button in the bulk
  action bar, shown whenever every currently-selected row is `accepted` or
  `corrected` (selection is cleared on any bulk action, so a mixed
  selection of `candidate`-state and `accepted`-state rows never happens
  in practice — a person naturally selects one batch, acts on it, then
  selects the next).
- **No change** to the single-item `promote` route's request/response
  shape, to due-date/owner carry-forward logic, or to what counts as a
  valid `accepted`/`corrected` precondition.

## Scope

**In scope:** the `/api/candidates/batch-promote` route; the shared
`promote_one` refactor; a "Promote N selected" bulk action in
`CandidatesTable.tsx`; a matching client function in `api.ts`.

**Out of scope, named honestly:** auto-promotion without a human
selecting the candidates (unchanged from ADR-0076 — a person still picks
what gets promoted); bulk actions on the Meeting Review page; any change
to due-date/owner extraction or resolution logic itself; batching across
unfetched pages.

## Options considered

- **Extend the existing bulk mechanism to promotion (chosen):** directly
  finishes what ADR-0076 started; reuses the exact transaction/audit logic
  `promote_candidate` already has, refactored rather than duplicated.
- **Leave promotion single-item, encourage accepting in bulk then
  promoting one at a time:** rejected — this is the status quo this ADR
  exists to fix; it still leaves the real bottleneck (getting candidates
  to actually appear as Obligations) untouched.
- **Auto-promote on bulk accept** (skip the separate promote step
  entirely for a bulk action): considered and rejected for the same
  reason ADR-0076 rejected confidence-threshold auto-promotion — accept
  and promote are deliberately distinct states
  ([ADR-0024](0024-candidate-accept-reject-buttons.md)/[ADR-0027](0027-promote-accepted-candidate-to-obligation.md));
  collapsing them removes a real checkpoint (a corrected candidate is
  reviewable between the two steps today) for the sake of one fewer click.

## Exit criteria and evidence

| Exit criterion | Evidence |
|---|---|
| `POST /api/candidates/batch-promote` promotes many candidates in one request, rebuilding both projections once | `batch-promote-rebuilds-projections-once` |
| A candidate not yet accepted, or missing, is reported per-id without failing the rest of the batch | `batch-promote-tolerates-partial-failure` |
| Promoted candidates carry due date and owner forward exactly as the single-item route does | `batch-promote-carries-due-date-and-owner-forward` |
| The Inbox table supports bulk-promoting selected accepted/corrected candidates | `frontend-bulk-promote-action` |
| No change to the single-item promote route's behavior | `single-item-promote-route-unchanged` |
