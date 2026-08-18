# ADR-0076: Bulk candidate triage — multi-select accept/reject, confidence-first ordering

- **Status:** Accepted
- **Date:** 2026-08-18
- **Decider:** monk-eee
- **Approval:** Direct instruction via clarifying question ("Yes, build it now" was the presented default; the decider was unavailable to click it and delegated with "Work autonomously and make good decisions" — the safer, human-reviewed option was chosen over the alternative auto-promotion option also on offer), 2026-08-18
- **Depends on:** [ADR-0013](0013-http-endpoints-trigger-and-list-extraction-candidates.md), [ADR-0024](0024-candidate-accept-reject-buttons.md), [ADR-0027](0027-promote-accepted-candidate-to-obligation.md), [ADR-0038](0038-wire-up-audit-events-for-candidate-validation.md), [ADR-0045](0045-correct-candidate-before-accepting.md), [ADR-0059](0059-list-view-pagination.md)
- **Tags:** frontend, backend, extraction, ux, throughput

## Context

[ADR-0024](0024-candidate-accept-reject-buttons.md),
[ADR-0027](0027-promote-accepted-candidate-to-obligation.md),
[ADR-0043](0043-meeting-review-page.md), and
[ADR-0045](0045-correct-candidate-before-accepting.md) each named "bulk
actions" as out of scope, deliberately deferred. That deferral is now the
single biggest obstacle to the product actually working for a time-poor
manager: a live audit of this repo's own dev database found 110 real,
extracted candidates (decisions, expectations, commitments) sitting
unreviewed, zero promoted to Obligations, because the only review
mechanism is a one-row-at-a-time Accept/Reject button in the Inbox table.
[VISION.md](../VISION.md) says the point of this system is that "managers
spend less effort reconstructing reality and more effort making
decisions" — 110 individual clicks is the opposite of that.

The fix has to stay honest about what it automates. `PRODUCT-SPEC.md`'s
"evidence before confidence" principle and every prior candidate-lifecycle
ADR treat a human's accept/reject as the actual validation step — an
extracted candidate is a claim, not a fact, until a person confirms it.
Silently auto-promoting candidates without review would remove that gate
entirely and let a wrong extraction become a first-class Obligation with
no one ever having looked at it. This ADR does not do that. It keeps the
same human-in-the-loop gate, and makes exercising it, many times in a row,
fast.

## Decision

- **New batch endpoint:** `POST /api/candidates/batch` accepts
  `{"candidate_ids": [...], "action": "accept" | "reject"}` and transitions
  every id in one request. Each candidate still goes through the exact
  same state check and atomic event+audit transaction the single-item
  routes already use ([ADR-0024](0024-candidate-accept-reject-buttons.md)/[ADR-0038](0038-wire-up-audit-events-for-candidate-validation.md))
  — refactored into one shared `transition_one` helper so the single-item
  and batch routes can never disagree about what a valid transition is.
  One candidate already transitioned, or not found, is reported back as a
  per-id error and does not stop the rest of the batch. The
  `candidate_projection` rebuild — previously once per single-item request
  — now runs exactly once per batch request, not once per candidate.
  `candidate_ids` is capped at 200, matching the existing list-view page
  ceiling ([ADR-0059](0059-list-view-pagination.md)).
- **Confidence-first ordering:** `GET /api/candidates`'s `ORDER BY` changes
  from `candidate_id` alone to `confidence DESC NULLS LAST, candidate_id`
  (the trailing `candidate_id` keeps pagination fully deterministic, as
  ADR-0059 requires). The highest-confidence, least-ambiguous candidates
  now load first, so selecting and bulk-approving the first page is
  selecting the batch most likely to be genuinely correct — the ambiguous,
  lower-confidence tail is exactly what's left for real per-row attention.
- **Bulk selection in the Inbox table:** `CandidatesTable` gains a header
  checkbox ("select all N loaded still awaiting review") and a per-row
  checkbox on every candidate still in the `candidate` state. A bulk
  action bar appears once anything is selected, with "Accept N selected"
  and "Reject N selected" buttons calling the new batch endpoint, then
  clearing selection and refetching exactly like every existing action
  already does. "Select all" only ever reaches rows already loaded on the
  client (matching this app's existing `Load more` pagination pattern
  honestly — it does not silently reach into unfetched pages), so
  clearing all 110 candidates is "select all, Accept, Load more, select
  all, Accept" a few times rather than 110 individual clicks.
- **No change** to `accept`/`reject`/`correct`/`promote`'s single-item
  routes' request or response shape, to the Meeting Review page's per-row
  actions ([ADR-0043](0043-meeting-review-page.md), untouched, out of
  scope here), or to `validation_state`'s meaning or transitions.

## Scope

**In scope:** the `/api/candidates/batch` route; the shared
`transition_one` refactor; the `list_candidates` ordering change;
multi-select checkboxes and a bulk action bar in `CandidatesTable.tsx`
(used by the Inbox tab); a matching client function in `api.ts`.

**Out of scope, named honestly:** auto-promotion or any action that
skips human review (rejected below); bulk *correct* (still one row at a
time — a correction is an edit, not a plain accept/reject, and batching
edits safely is a materially different, larger problem); bulk actions on
the Meeting Review page's per-fragment candidates (a different component,
different ADR if ever done); selecting across unfetched pages ("select
all 110" without first loading them); any change to the extraction
pipeline itself, to `promote_candidate`, or to Obligation creation.

## Options considered

- **Multi-select bulk accept/reject with confidence-first ordering
  (chosen):** directly attacks the stated problem (too many candidates,
  too little time) without weakening the human-review gate every prior
  candidate ADR relies on. Reuses the exact existing transition/audit
  logic; the only new code is the batch loop, the ordering clause, and
  the selection UI.
- **Confidence-threshold auto-promotion** (e.g., silently promote
  candidates ≥95% confidence straight to Obligations): considered and
  presented as an explicit alternative; not chosen because it removes the
  human validation step this codebase has treated as load-bearing since
  ADR-0024, and because a wrong high-confidence extraction would become a
  first-class Obligation — visible on Today, actionable, presumed true —
  with no one ever having looked at it. Left as a clearly-named option for
  a future ADR if the decider wants to trade that risk for more
  automation.
- **A digest/summary view instead of a faster review UI** (e.g., group
  candidates by type/meeting and show counts only): would help
  understand the backlog but does nothing to clear it; the manager would
  still end up back in the Inbox clicking one row at a time. Rejected as
  not actually solving the stated problem.

## Exit criteria and evidence

| Exit criterion | Evidence |
|---|---|
| `POST /api/candidates/batch` transitions many candidates in one request, rebuilding the projection once | `batch-endpoint-rebuilds-projection-once` |
| A candidate already transitioned, or missing, is reported per-id without failing the rest of the batch | `batch-endpoint-tolerates-partial-failure` |
| `GET /api/candidates` orders by confidence descending, id as tiebreak | `candidates-ordered-by-confidence-first` |
| The Inbox table supports selecting loaded candidates and bulk accept/reject | `frontend-bulk-select-and-act` |
| No change to single-item accept/reject/correct/promote behavior | `single-item-routes-unchanged` |
