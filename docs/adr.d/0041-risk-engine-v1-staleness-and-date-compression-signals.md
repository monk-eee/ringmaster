# ADR-0041: Risk Engine v1 — staleness and date-compression signals

- **Status:** Accepted
- **Date:** 2026-08-14
- **Decider:** monk-eee
- **Approval:** Implemented following this session's established delegation pattern for well-scoped, low-risk proposals ("keep going"), 2026-08-14
- **Depends on:** [ADR-0020](0020-obligation-due-date-fields.md), [ADR-0022](0022-daily-brief-endpoint.md), [ADR-0023](0023-evidence-backed-daily-brief-reasons.md), [ADR-0029](0029-time-horizon-view.md)
- **Tags:** architecture, api, frontend

## Context

[PRODUCT-SPEC.md §7](../PRODUCT-SPEC.md#7-attention-and-risk-engine) names
nine risk signals the Risk Engine should combine (date compression,
staleness, unowned obligation, repeated concern, work disconnect, outcome
disconnect, coverage gap, cadence lapse, contradiction), plus, per
VISION.md, a 🔴🟠🟡🟢 severity color combining them. monk-eee's own stated
priority order (recorded in [ADR-0028](0028-person-relationship-view.md)
and [ADR-0029](0029-time-horizon-view.md)) sequences the Risk Engine as
priority #4/#5, after the Congruence Engine. But
[ADR-0031](0031-suggested-focus-blocks.md) already found the Congruence
Engine's real definition (commitment/goal/work-item drift detection) is
blocked on ADO integration, which does not exist yet — so building the
Congruence Engine next, as literally specified, is not honestly possible
today. Two named Risk Engine signals, however, are derivable from data
that exists right now with zero schema change:

- **Date compression** — "a transition is due soon but no handover
  evidence exists" — computable from the same `hard_due_at`/
  `soft_due_at`/`source_fragment_id` fields
  [ADR-0022](0022-daily-brief-endpoint.md) and
  [ADR-0029](0029-time-horizon-view.md) already select.
- **Staleness** — "an accepted commitment has not been touched since the
  source meeting" — computable from `obligation_projection.updated_at`,
  already selected by both routes.

The other seven signals each need a concept or data source that doesn't
exist yet (an owner/accountable field, cross-meeting semantic-similarity
tuning, ADO work items, customer-outcome nodes, calendar/leave ingestion,
a recurrence field, conflict detection across evidence) — building any of
them now would mean fabricating data or inventing a scoring model with no
real basis, which this repo's evidence discipline forbids.

## Decision

- A new pure function, `risk_signals(hard_due_at, soft_due_at,
  updated_at, source_fragment_id)`, computing zero or more independent
  signals:
  - `date_compression`: the effective due date (`hard_due_at` else
    `soft_due_at`) is within 7 days or already overdue (reusing
    [ADR-0029](0029-time-horizon-view.md)'s own "Next 7 Days" boundary
    rather than inventing a second number), **and** no
    `source_fragment_id` is linked.
  - `stale`: more than 14 days have passed since `updated_at` (a
    disclosed, hardcoded first-cut threshold, the same kind of round,
    stated choice ADR-0029 already made for its 7/30/90-day bucket
    boundaries).
  - Both signals can apply to the same Obligation at once; each carries a
    plain, deterministic `explanation` string (no fabricated severity,
    matching every prior reason-string precedent).
- `GET /api/daily-brief` and `GET /api/time-horizon` each gain one new
  field per row, `risk_signals: [{ signal, explanation }]`, computed via
  the shared function above — no new route, no duplicated logic, no
  change to either route's existing ranking or bucketing.
- **Frontend:** Daily Brief rows and Time Horizon bucket rows render a
  `.risk-signals` list of each present signal's full `explanation` text,
  directly below the existing reason line. An Obligation with no signals
  renders no list at all — never an empty placeholder.

## Scope

**In scope:** the `risk_signals` function; the new field on the two
existing routes; badge rendering in `DailyBrief.tsx` and
`TimeHorizon.tsx`.

**Out of scope, named honestly (deferred, larger/separate work):** the
other seven §7.1 signals (unowned obligation, repeated concern, work
disconnect, outcome disconnect, coverage gap, cadence lapse,
contradiction) — each blocked on a concept or data source that doesn't
exist yet, named above; any combined single severity score or 🔴🟠🟡🟢
color — combining independent signals into one score needs a weighting
model this ADR does not decide, and an arbitrary weighting would be
exactly the kind of fabrication this repo's evidence discipline forbids;
any change to Daily Brief ranking or Time Horizon bucketing based on
these signals (they are additive, informational fields only); making the
14-day/7-day thresholds user-configurable.

## Options considered

- **Compute two derivable signals as an additive field on existing
  routes (chosen):** zero schema change, zero new route, reuses every
  existing field and both routes' existing evidence-join; ships something
  real today instead of waiting on the Congruence Engine or ADO
  integration, neither of which are ready.
- **Wait until all nine signals and a real severity model can be built
  together:** would match VISION's fuller mockup in one step, but has no
  committed timeline (several signals need integrations not yet started)
  and repeats the same mistake ADR-0031 already rejected for the
  Congruence Engine — descoping into fabrication or an indefinite wait.
- **A new dedicated `GET /api/risk-signals` route instead of decorating
  existing rows:** VISION's own mockup shows the severity marker inline
  with each item in the Daily Brief/workbench, not as a separate list; a
  new route would need its own duplicate query against
  `obligation_projection` for no benefit over adding one field to the two
  routes that already select every input this needs — rejected as
  needless duplication.

## Consequences

- **Positive:** first real, evidence-based step toward monk-eee's Risk
  Engine priority, without fabricating the seven signals or the severity
  model that aren't honestly buildable yet; zero schema change; zero new
  reasoning duplicated across routes.
- **Negative / trade-off:** only 2 of 9 named signals exist after this
  ADR; no single "how bad is this" color exists yet, so the UI shows
  named badges instead of VISION's 🔴🟠🟡🟢 shorthand until a real
  weighting model is decided.
- **Risk:** low. Purely additive read-side field; no writes; no schema
  migration; both new signal computations are pure functions with direct
  unit test coverage, independent of the shared development database.
