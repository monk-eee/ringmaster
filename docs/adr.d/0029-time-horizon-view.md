# ADR-0029: Time Horizon view — Obligations bucketed by due-date window

- **Status:** Accepted
- **Date:** 2026-08-14
- **Decider:** monk-eee
- **Approval:** Explicitly accepted by monk-eee on 2026-08-14
- **Depends on:** [ADR-0020](0020-obligation-due-date-fields.md), [ADR-0022](0022-daily-brief-endpoint.md), [ADR-0023](0023-evidence-backed-daily-brief-reasons.md)
- **Tags:** architecture, api, frontend, data-model

## Context

[VISION.md § Timeline, not graph, not table, not kanban](../VISION.md#timeline-not-graph-not-table-not-kanban----the-future-risk-horizon)
names this as monk-eee's own **#3 priority**, directly after the
now-shipped Daily Brief and Relationship View: *"Managers think in time,
not hierarchy or entity type."* Its own mockup:

> **Next 7 Days** — Transition Plan (no evidence recorded); Team onboarding
> (no intro meetings completed).
>
> **Next 30 Days** — John leave coverage (no successor identified);
> Training follow-up (no activity recorded).
>
> **Next 90 Days** — Connect cycle; Service review cycle; Team morale
> checkpoint.

The Daily Brief ([ADR-0022](0022-daily-brief-endpoint.md)/[ADR-0023](0023-evidence-backed-daily-brief-reasons.md))
already answers "what's most urgent right now" as a single flat, ranked
list. This is a different, complementary lens on the same Obligation data:
not "what's most urgent" but "what falls due when" — grouped by time
window rather than ranked by urgency. Both due-date fields
([ADR-0020](0020-obligation-due-date-fields.md)) and the deterministic
`reason` string ([ADR-0022](0022-daily-brief-endpoint.md)/[ADR-0023](0023-evidence-backed-daily-brief-reasons.md))
already exist and are reused here unchanged.

## Decision

- A new read-only route, `GET /api/time-horizon`, reads non-closed
  `obligation_projection` rows the same way the Daily Brief does (same
  `LEFT JOIN` against `source_fragments` for evidence), and buckets each
  by its effective due date (`hard_due_at` if present, else `soft_due_at`,
  else "no date") relative to now:
  - **Overdue:** effective due date is in the past.
  - **Next 7 days**
  - **Next 30 days**
  - **Next 90 days**
  - **Beyond 90 days / no date:** effective due date is more than 90 days
    out, or absent entirely.
  - An `at_risk`-status Obligation with no due date at all is bucketed
    under **Overdue** (matching the Daily Brief's own precedent of
    treating `at_risk` as the most urgent state regardless of dates),
    not "no date" — this is the one exception to pure date-bucketing.
  - Within each bucket, Obligations are ordered soonest-due-first (ties
    broken by `updated_at DESC`), the same ordering direction the Daily
    Brief already uses.
  - Each entry carries the same fields `GET /api/daily-brief` already
    returns: `obligation_id`, `status`, `hard_due_at`, `soft_due_at`,
    `source_fragment_id`, `reason` (via the existing `daily_brief_reason`
    function, unchanged).
- **Frontend:** a new **Time Horizon** tab, presented as five stacked
  sections (Overdue, Next 7 Days, Next 30 Days, Next 90 Days, Beyond /
  No Date), each rendering its Obligations with the same status badge +
  reason presentation the Daily Brief and Relationship view already use
  ([ADR-0022](0022-daily-brief-endpoint.md)/[ADR-0028](0028-person-relationship-view.md)).
  An empty bucket is simply omitted, not shown as an empty section.

## Scope

**In scope:** the `GET /api/time-horizon` route; date-window bucketing
logic; a Time Horizon frontend tab reusing existing presentation.

**Out of scope, named honestly (deferred to later, named priorities):**
any severity/color scoring within a bucket (VISION's 🔴🟠🟡🟢 markers) —
that needs the Risk Engine (monk-eee's stated #5 priority), which
computes a real signal this ADR does not; grouping by management
direction (People/Delivery/Leadership/Operations) within a bucket — that
needs the Congruence Engine (#4); recurrence-aware due dates (a
recurring Obligation's *next* occurrence) — no recurrence field exists on
Obligation today; a combined/merged view with the Daily Brief — this
ships as a separate tab, reconciling the two into one home view is
explicitly left as a future, separate decision.

## Options considered

- **A new route + new tab, reusing existing fields (chosen):** smallest
  change that satisfies the ask; no new reasoning logic, no schema
  change, consistent with every prior ADR's read-only-first sequencing.
- **Extend `GET /api/daily-brief` with a `?group_by=horizon` query
  parameter instead of a new route:** would avoid a second route, but
  conflates two different response shapes (a flat ranked array vs. a
  bucketed object) behind one endpoint and one query flag — rejected as
  a worse API shape for a real difference in what's being asked.
- **Compute buckets client-side from the existing `/api/daily-brief` or
  `/api/obligations` data:** would need no new backend route at all, but
  `/api/daily-brief` already excludes closed Obligations for its own
  urgency-ranking reasons that happen to suit this view too, so the
  duplication is small either way, and a dedicated route keeps the
  bucketing rule (including the at-risk-Obligation-with-no-date
  exception) in one server-side place rather than reimplemented in the
  frontend — rejected in favor of the dedicated route for that reason.

## Consequences

- **Positive:** directly serves monk-eee's stated #3 priority; zero new
  reasoning logic (reuses `daily_brief_reason` verbatim); zero schema
  change (buckets are computed from fields that already exist).
- **Negative / trade-off:** the "5 buckets, soonest-first within each"
  shape is a deliberately simple first cut; VISION's fuller mockup
  (severity color, cross-referencing what's driving each risk) waits on
  the Risk Engine and Congruence Engine named above.
- **Risk:** low. Purely additive read route; no writes; reuses two
  already-proven building blocks (the evidence-join pattern, the reason
  function) rather than adding new ones.

## Exit criteria and evidence

Evidence: [EV-0029](../evidence.d/0029-time-horizon-view.md)

| Exit criterion | Evidence |
|---|---|
| `GET /api/time-horizon` groups non-closed Obligations into Overdue/7/30/90/Beyond buckets | `time-horizon-route-buckets-by-due-date` |
| An at-risk Obligation with no due date lands in Overdue, not "no date" | `at-risk-no-date-lands-in-overdue` |
| A closed Obligation never appears in any bucket | `closed-excluded-from-time-horizon` |
| A Time Horizon tab exists and renders the bucketed Obligations | `time-horizon-tab-exists` |
