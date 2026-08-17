# ADR-0061: A derived Obligation health label — composing existing status and signals, not a new score

- **Status:** Proposed
- **Date:** 2026-08-17
- **Decider:** monk-eee
- **Approval:** Pending — awaiting monk-eee's decision
- **Depends on:** [ADR-0041](0041-risk-engine-v1-staleness-and-date-compression-signals.md), [ADR-0046](0046-unowned-obligation-risk-signal.md), [ADR-0054](0054-congruence-engine-v1-isolated-commitment-signal.md)
- **Tags:** api, frontend, product

## Context

An independent product review argued Ringmaster's Obligations need
distinct health states — Healthy, At Risk, Stalled, Waiting, Completed,
Broken — instead of just a raw `status` plus a list of independent risk
signals. The review's own examples are already derivable from fields this
repo already computes: "Due in 5 days, no evidence" is
`date_compression`; "No activity 21 days" is `stale`
([ADR-0041](0041-risk-engine-v1-staleness-and-date-compression-signals.md));
"Due date passed, no completion evidence" is close to `status=open` with
`hard_due_at` in the past. Nothing here needs a new signal — it needs one
more readable label naming what the existing combination *means*, the
same relationship [ADR-0041](0041-risk-engine-v1-staleness-and-date-compression-signals.md)'s
signals already have to `status`.

This is explicitly **not** the review's other ask, a continuous "evidence
score" that raises or lowers a number —
[ADR-0041](0041-risk-engine-v1-staleness-and-date-compression-signals.md)
("no combined severity score is computed here") and
[ADR-0053](0053-what-am-i-forgetting.md) both already chose independent,
named signals over a score, because no validated weighting model exists.
A `health` label is a **deterministic lookup**, not a score: it can only
take one of five fixed values, computed the same way
`time_horizon_bucket()` already classifies a due date into one of five
fixed buckets — no weighting, no tuning, nothing to validate.

## Decision

A new pure function, `obligation_health`, taking the same fields
`risk_signals`/`time_horizon_bucket` already take (`status`,
`hard_due_at`, `risk_signals`), returns exactly one of:

- **`Completed`** — `status == "closed"`. Named limitation: this repo's
  `status` doesn't yet distinguish a deliberately-dismissed Obligation
  from a genuinely-finished one ([PRODUCT-SPEC.md §6.4](../PRODUCT-SPEC.md#64-validation-states)
  names both "Observed complete" and "Closed" as distinct states this
  schema doesn't yet carry) — both currently map to `Completed`, an
  honest approximation, not a claim of certainty.
- **`At Risk`** — `status == "at_risk"`.
- **`Broken`** — `status == "open"` and `hard_due_at` is in the past.
  Approximates the review's "due date passed, no completion evidence" —
  this repo has no distinct "completion evidence" field yet, so an
  overdue-and-still-open Obligation is the closest honest proxy.
- **`Stalled`** — `status == "open"`, not `Broken`, and the `stale` signal
  is present (no update in 14+ days).
- **`Healthy`** — `status == "open"`, none of the above.

Attached alongside `risk_signals` everywhere it's already computed
(`GET /api/daily-brief`, `GET /api/time-horizon`,
`GET /api/obligations/:id`) — no new route.

## Scope

**In scope:** the `obligation_health` function and its five-value output;
attaching `health` next to `risk_signals` at the three existing call
sites.

**Out of scope, named honestly:**

- **`Waiting`** (the review's sixth state — "blocked on someone else").
  Nothing in this schema models a blocking relationship between
  Obligations or people today; inventing one to populate this label would
  be fabrication, not derivation. Five states ship; a sixth waits for a
  real "blocked_by" edge type to be decided separately.
- **Any numeric score, weighting, or threshold tuning.** `health` is a
  fixed five-way classification, not a continuous measure — deliberately,
  per this ADR's own Context section.
- **Surfacing `health` in the frontend.** This ADR computes and attaches
  the field to existing API responses only; whether/how Today, Time
  Horizon, or the Obligation detail page render it is separate, later
  work — the same "backend first, presentation later" sequencing
  [ADR-0041](0041-risk-engine-v1-staleness-and-date-compression-signals.md)
  itself used.

## Options considered

- **A deterministic five-value lookup, reusing existing fields (chosen):**
  matches `time_horizon_bucket()`'s own established shape for exactly
  this kind of classification; adds real readability without adding a
  score.
- **A continuous 0–100 health score:** rejected — directly contradicts
  [ADR-0041](0041-risk-engine-v1-staleness-and-date-compression-signals.md)'s
  and [ADR-0053](0053-what-am-i-forgetting.md)'s already-made decision
  against combined scoring; no validated model exists to weight signals
  against each other honestly.
- **Include `Waiting` now, inferred from *any* edge at all:** rejected —
  an Obligation merely having some edge (e.g., `owns`) says nothing about
  being blocked; a real blocking signal needs a real blocking edge type,
  not an overloaded existing one.

## Consequences

- **Positive:** a single, readable label that names what a status +
  signal combination already means, with zero new fabricated logic.
- **Positive:** directly and honestly addresses the review's health-state
  idea while explicitly declining the score-based version of the same
  idea, keeping this repo's own risk-modeling position consistent.
- **Negative / trade-off:** `Completed` conflates "genuinely done" and
  "deliberately dismissed" until `status` itself can distinguish them — a
  named, pre-existing schema limitation, not introduced by this ADR.
- **Risk:** low. One new pure function reusing existing fields; additive
  to three existing responses; no schema change.

## Exit criteria and evidence

Evidence: [EV-0061](../evidence.d/0061-obligation-health-label.md)

| Exit criterion | Evidence |
|---|---|
| `obligation_health` returns exactly one of `Completed`/`At Risk`/`Broken`/`Stalled`/`Healthy`, never a sixth value or a number | `obligation-health-returns-fixed-five-values` |
| `health` is attached alongside `risk_signals` on Daily Brief, Time Horizon, and Obligation detail | `obligation-health-attached-at-existing-call-sites` |
| An overdue, still-open Obligation with no stale signal returns `Broken`, not `Stalled` | `obligation-health-distinguishes-broken-from-stalled` |
