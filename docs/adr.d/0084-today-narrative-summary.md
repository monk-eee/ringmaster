# ADR-0084: Today's narrative summary — the ranked count line VISION.md describes

- **Status:** Accepted
- **Date:** 2026-08-19
- **Decider:** monk-eee
- **Approval:** Direct instruction ("ok more work?"), continuing this session's established practice of drafting and implementing the next item from `docs/IMPROVEMENT-PLAN.md`'s suggested order, 2026-08-19
- **Depends on:** [ADR-0022](0022-daily-brief-endpoint.md), [ADR-0023](0023-evidence-backed-daily-brief-reasons.md), [ADR-0041](0041-risk-engine-v1-staleness-and-date-compression-signals.md), [ADR-0053](0053-what-am-i-forgetting.md)
- **Tags:** frontend, ux

## Context

`docs/VISION.md`'s "The UX is the product" section (2026-08-14) names the
gap directly: Today currently opens with a single count
("N things need your attention right now") followed by a ranked list, a
"What am I forgetting?" section, and "Do these together" — a small
dashboard, not the narrative brief the vision describes:

> 4 things need attention today.
> 2 will become risks this week.
> 1 commitment appears forgotten.

`docs/IMPROVEMENT-PLAN.md` §2.1 names this the first of three UX-reframe
gaps and is explicit about scope: "a presentation change over existing
Today data, not a new signal." Every number the mockup needs already
exists on each `DailyBriefItem` already fetched for Today:
`risk_signals` carries the exact `date_compression` (due within 7 days or
already overdue, [ADR-0041](0041-risk-engine-v1-staleness-and-date-compression-signals.md))
and `stale` (no update in a disclosed number of days, same ADR) signal
names the backend already computes and the frontend already receives.
"Will become risks this week" and "appears forgotten" are not new
concepts to invent — they are `date_compression` and `stale`, worded in
plain language.

The mockup's "Good morning, Lyndon" greeting is *not* implemented here:
no setting, env var, or stored profile anywhere in this codebase holds a
person's display name (checked `.env.example`, `compose.yaml`); inventing
one now would be a fabrication this repo's own conventions explicitly
avoid, and a real one needs its own identity/settings decision, not a
silent addition to a UX-copy record. A plain, time-of-day-aware
"Good morning"/"Good afternoon"/"Good evening" (derived from the browser's
own clock, already available, no new dependency) is added instead — honest
about what is and isn't known.

## Decision

- **A short narrative summary block** renders above the existing ranked
  list on Today, replacing the current single-line greeting:
  - A time-of-day greeting ("Good morning."/"Good afternoon."/
    "Good evening.", derived from `new Date().getHours()`) with no name.
  - "N thing(s) need attention today." — unchanged from today's existing
    count and wording (`dailyBrief.length`).
  - "M will become risks this week." — count of `dailyBrief` items whose
    `risk_signals` include `date_compression`. Omitted entirely when zero,
    never "0 will become risks this week" (an honest omission, not a
    fabricated non-event).
  - "K commitment(s) appear forgotten." — count of `dailyBrief` items
    whose `risk_signals` include `stale`. Same zero-omission rule.
  - When `dailyBrief.length` is zero, the existing honest empty state
    ("Nothing needs your attention right now.") is unchanged and no stat
    lines render.
- **Pure client-side computation over data already fetched.** No new
  route, no new backend field, no new risk signal. `App.tsx` already
  fetches `dailyBrief` with each item's `risk_signals`; the two new counts
  are `Array.prototype.filter` over that same array.
- **The ranked list, "What am I forgetting?", "Do these together", and the
  "Coming soon" strip below the new summary are unchanged** — this record
  touches only the greeting/summary block, not list rendering, ranking,
  or capping.

## Scope

**In scope:** a new `TodaySummary` composing the two counts above the
existing `DailyBrief` list; the time-of-day greeting; the honest
zero-omission rule for each stat line.

**Out of scope, named honestly:** a personalized name in the greeting
(needs a real identity/settings concept — a separate, future ADR);
rewriting the ranked list itself into free prose sentences (the vision's
narrative framing is satisfied by the new summary line; converting each
list row from structured/clickable to prose text is a materially larger,
riskier UI change not required to close this specific named gap);
`unowned`/`isolated` signal counts in the summary (not named in the
mockup; the three lines shown map exactly to the three sentences
`VISION.md` gives, no more); any change to Focus Blocks/Congruence
(`docs/IMPROVEMENT-PLAN.md` §2.2, a separate item).

## Options considered

- **A client-side summary computed from already-fetched `risk_signals`
  (chosen):** delivers exactly the narrative VISION.md asks for with zero
  backend change, reusing the exact signal vocabulary already proven by
  ADR-0041/ADR-0053.
- **A new backend `/api/daily-brief/summary` endpoint returning
  pre-aggregated counts:** rejected — the counts are a trivial filter over
  data the frontend already holds in memory; a new route would be
  unnecessary surface for no additional correctness or performance
  benefit, and IMPROVEMENT-PLAN.md's own framing ("composition over
  existing data, not new extraction") argues against it.
- **Full prose rewrite of the ranked list** (each row as a sentence
  instead of a table-like row): closer to the mockup's exact visual
  style, but a substantially larger, riskier change (loses the existing
  clickable-row-into-detail interaction without a redesign) for a gap
  this ADR's narrower slice already closes. Left as a possible later
  refinement, not attempted here.

## Exit criteria and evidence

| Exit criterion | Evidence |
|---|---|
| Today shows a time-of-day greeting with no fabricated name | `today-greeting-is-time-of-day-aware` |
| The summary reports counts of `date_compression` and `stale` signals from already-fetched daily brief items | `summary-counts-reuse-existing-risk-signals` |
| A zero count for either stat line is omitted, not shown as "0 ..." | `summary-omits-zero-counts` |
| An empty daily brief still shows the existing honest empty state, no stat lines | `summary-honest-empty-state-unchanged` |
| The ranked list, Forgetting section, and Focus Blocks render unchanged below the summary | `existing-today-sections-unchanged` |
