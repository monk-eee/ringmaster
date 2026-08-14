# ADR-0022: A read-only Daily Brief endpoint — Obligations ranked by urgency

- **Status:** Accepted
- **Date:** 2026-08-14
- **Decider:** monk-eee
- **Approval:** Direct instruction ("accept continue"), 2026-08-14
- **Depends on:** [ADR-0012](0012-minimal-http-api-and-node-web-front-end.md), [ADR-0020](0020-obligation-due-date-fields.md)
- **Tags:** architecture, api, attention-horizon

## Context

[VISION.md § The UX is the product](../VISION.md#the-ux-is-the-product)
(monk-eee, 2026-08-14) names the Daily Brief as *the* product: "Good
morning, Lyndon. 4 things need attention today." — a ranked narrative, not
a dashboard of panel counts. It is also named the #1 priority in
[VISION.md § A reframed priority order](../VISION.md#a-reframed-priority-order).
[ADR-0020](0020-obligation-due-date-fields.md) just closed the one hard
prerequisite this needed: `obligation_projection` now actually carries
`hard_due_at`/`soft_due_at`. Nothing today ranks obligations by urgency or
states why one matters — `GET /api/obligations`
([ADR-0012](0012-minimal-http-api-and-node-web-front-end.md)) is a flat,
unordered-by-urgency list.

This ADR is deliberately the smallest real slice of the Daily Brief: a
ranked list with a plain, deterministic reason, built only from data that
already exists. It is **not** the vision's full Daily Brief. Two things
that vision names are explicitly not buildable yet and are out of scope
here:

- **Evidence-backed reasons** ("No evidence recorded") need a link between
  an Obligation and the source fragments/candidates that support it. No
  such link exists in the schema today — Obligations and Candidates are
  entirely separate aggregates ([ADR-0005](0005-adopt-rust-event-sourced-postgres-commitment-graph.md)/[ADR-0011](0011-extraction-pipeline-candidate-schema-and-model-adapter.md)).
  Building that link is its own future, bounded decision.
- **Congruence grouping** ("these belong together, spend 90 minutes")
  needs graph traversal across shared people/services/meetings, which
  [PRODUCT-SPEC.md §7](../PRODUCT-SPEC.md#7-attention-and-risk-engine)'s
  risk-signal work hasn't built yet either.

Shipping the plain ranked list now — honestly, without fabricating
evidence citations or groupings that don't exist — is consistent with
[PRODUCT-SPEC.md §7](../PRODUCT-SPEC.md#7-attention-and-risk-engine)'s own
rule: "Scores are advisory and explainable, never employee performance
metrics," and with this repository's practice throughout of shipping the
provable slice before the speculative one.

## Decision

- `GET /api/daily-brief` is a new, read-only route. It selects every
  non-`closed` row from `obligation_projection` and ranks them by:
  1. `status = 'at_risk'` first,
  2. then ascending `hard_due_at` (nulls sort last — an obligation with no
     hard date is never treated as more urgent than one with a known,
     approaching date),
  3. then ascending `soft_due_at` (nulls last),
  4. then most-recently-`updated_at` first, as a final, arbitrary but
     deterministic tiebreak.

  This is a plain, explainable SQL `ORDER BY` — no scoring model, no
  weights, nothing to tune. It exists so the ranking can be reasoned about
  by reading the query.
- Each item's `reason` is generated deterministically from the same
  fields the ranking already used, in this priority order: `"Marked at
  risk."` → `"Overdue by N day(s)."` (when `hard_due_at` is in the past)
  → `"Due in N day(s)."` (when `hard_due_at` is in the future) →
  `"Expected around <date>."` (when only `soft_due_at` is set) → `"No due
  date recorded."` (neither is set). No reason ever cites evidence,
  meetings, or grouped obligations that this ADR does not have data for.
- Response shape: an ordered JSON array of `{obligation_id, status,
  hard_due_at, soft_due_at, updated_at, reason}` — the same fields
  `GET /api/obligations` already returns, plus the derived `reason`.
- No new frontend surface is added by this ADR. Whether/how a Daily Brief
  view gets its own frontend page follows whatever precedent
  [ADR-0021](0021-ratify-search-tab-surfaced-without-its-own-adr.md)
  settles for "does surfacing an existing read route in the SPA need its
  own ADR."

## Scope

**In scope:** the one new read-only route; the deterministic ranking
query; the deterministic, evidence-free `reason` string.

**Out of scope:** evidence-backed reasons (needs Obligation↔source-
fragment/candidate linkage, not yet decided); Congruence/grouping
("Suggested Focus Blocks"); Focus Sessions; the Attention Pressure gauge;
Relationship pages; a frontend Daily Brief page; recurrence, staleness, or
any other [§7.1](../PRODUCT-SPEC.md#71-initial-risk-signals) risk signal
beyond due-date ordering; writing/mutating anything.

## Options considered

- **A plain SQL `ORDER BY` with a deterministic reason string (chosen):**
  the smallest thing that's honestly better than an unordered list, built
  entirely from data [ADR-0020](0020-obligation-due-date-fields.md) just
  made real. Fully explainable — a person can read the query and know
  exactly why row 1 outranks row 2.
- **A weighted/scored ranking model:** rejected for now — there is no real
  signal yet (staleness, repetition, coverage gaps) to weight against
  [PRODUCT-SPEC.md §7.1](../PRODUCT-SPEC.md#71-initial-risk-signals)'s
  other named risks; inventing weights this early would be guesswork
  dressed up as a decision.
- **Wait until evidence-linkage and Congruence grouping both exist, ship
  the "real" Daily Brief in one ADR:** rejected — repeats the mistake this
  repository has consistently avoided of bundling a large, undiscussed
  design (evidence linkage, graph clustering) with a small, immediately
  provable one.

## Consequences

- **Positive:** the first real, ordered, explainable step toward the
  vision's #1 priority; ships today using only data that
  already, provably, exists.
- **Negative / trade-off:** the `reason` text is honest but plain — it
  will not yet say "no evidence recorded" or group related obligations,
  even though the vision's mockup shows both. Closing that gap needs the
  two explicitly-named future decisions above.
- **Risk:** none material — read-only, additive, new route.

## Exit criteria and evidence

Evidence: [EV-0022](../evidence.d/0022-daily-brief-endpoint.md)

| Exit criterion | Evidence |
|---|---|
| `GET /api/daily-brief` ranks non-closed obligations by urgency (at-risk, then soonest hard due date, then soonest soft due date) | `daily-brief-route-exists` |
| Each item includes a deterministic, evidence-free `reason` string | `daily-brief-includes-reason` |
