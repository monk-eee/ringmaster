# ADR-0053: "What am I forgetting?" — compose existing risk signals into one capped, prominent list

- **Status:** Accepted
- **Date:** 2026-08-17
- **Decider:** monk-eee
- **Approval:** Explicitly accepted by monk-eee on 2026-08-17 ("accept all")
- **Depends on:** [ADR-0022](0022-daily-brief-endpoint.md), [ADR-0041](0041-risk-engine-v1-staleness-and-date-compression-signals.md), [ADR-0046](0046-unowned-obligation-risk-signal.md), [ADR-0050](0050-today-attention-budget.md)
- **Tags:** api, frontend, architecture

## Context

[VISION.md](../VISION.md#one-button-what-am-i-forgetting)'s own named
end-state is one button — *"WHAT AM I FORGETTING?"* — answering directly
in plain language, not a search box. An independent product review of
[docs/current-status.md](../current-status.md)'s audit named this the
single biggest missing surface: Ringmaster can currently report
obligations, candidates, timelines, and relationships, but nothing
currently asks and answers that one question directly. It doesn't need a
new engine — [ADR-0041](0041-risk-engine-v1-staleness-and-date-compression-signals.md)/[ADR-0046](0046-unowned-obligation-risk-signal.md)
already compute three real, non-fabricated signals
(`date_compression`, `stale`, `unowned`) and already attach them to every
Daily Brief/Time Horizon row — they're just never surfaced as their own
answer to their own question.

## Decision

- **A new section on Today, above "Do these together"**: *"What am I
  forgetting?"*, showing at most 5 Obligations that carry at least one
  risk signal (`date_compression`/`stale`/`unowned`), ranked by number of
  signals present (more flags first), then the existing Daily Brief
  urgency order as a tiebreak. Each row states which signal(s) fired in
  plain language — the same `explanation` strings
  [ADR-0041](0041-risk-engine-v1-staleness-and-date-compression-signals.md)/[ADR-0046](0046-unowned-obligation-risk-signal.md)
  already compute, not new wording invented for this surface.
- **No new backend route or signal.** This filters/re-ranks
  `GET /api/daily-brief`'s existing response (which already includes
  `risk_signals` on every row) on the frontend; zero obligations without
  at least one signal ever appear here, and zero obligations are flagged
  that today's signals wouldn't already flag elsewhere.
- **An honest empty state** ("Nothing flagged right now") when no
  Obligation carries any signal — never a fabricated reassurance, matching
  every other empty state in this codebase.

## Scope

**In scope:** a capped, ranked "What am I forgetting?" section on Today,
composing the three existing risk signals; its empty state.

**Out of scope, named honestly:**

- **New signal types** (e.g., "recurring obligation," a fourth kind of
  flag). Nothing in the schema models recurrence today; inventing a
  detector for it now would be exactly the fabrication this repo's
  principles reject. [PRODUCT-SPEC.md §7](../PRODUCT-SPEC.md#7-attention-and-risk-engine)'s
  remaining named signals (repeated concern, work disconnect, outcome
  disconnect, coverage gap, cadence lapse, contradiction) stay real,
  separate, larger future work — most need data this repo doesn't have
  yet (ADO/calendar ingestion).
- **A combined severity score across signals.** [ADR-0041](0041-risk-engine-v1-staleness-and-date-compression-signals.md)
  already rejected this explicitly ("no combined severity score is
  computed here"); ranking by signal *count*, used here, is not a score —
  it's a plain, honest tiebreak using data already shown.
- **A literal single button/modal interaction.** VISION.md's framing is a
  future, larger UX bet; this ADR ships the underlying capability (a real,
  capped, composed answer) as a Today section first.

## Options considered

- **Compose existing signals into a capped Today section (chosen):**
  zero new backend logic; reuses fields already computed and already
  proven; directly answers VISION.md's own named end-state with what
  already exists.
- **Build a dedicated `/api/forgetting` route:** rejected as unnecessary —
  `GET /api/daily-brief` already returns every field this needs; a new
  route over the same data would just be a filtered duplicate.
- **Invent new signal types now to make the list richer:** rejected —
  no evidence exists yet for signals needing data this repo hasn't
  ingested (ADO work items, calendar); premature per this repo's own
  standing objection to building ahead of a real data source
  ([ADR-0001](0001-require-governing-adr-coverage-before-implementation.md)).

## Consequences

- **Positive:** directly answers VISION.md's own stated end-state
  question using only signals already computed, proven, and shown
  elsewhere — no new risk of a fabricated claim.
- **Positive:** composes cleanly with [ADR-0050](0050-today-attention-budget.md)'s
  attention budget — Today gains a third capped section, not an uncapped
  one.
- **Negative / trade-off:** none identified — purely additive frontend
  composition of an existing response.
- **Risk:** low. No schema or backend change; reuses already-tested
  signal computation verbatim.

## Exit criteria and evidence

Evidence: [EV-0053](../evidence.d/0053-what-am-i-forgetting.md)

| Exit criterion | Evidence |
|---|---|
| Today shows a "What am I forgetting?" section listing at most 5 Obligations, each carrying at least one existing risk signal | `forgetting-section-capped-and-signal-filtered` |
| Rows are ranked by signal count, then existing Daily Brief order | `forgetting-section-ranked-by-signal-count` |
| An honest empty state renders when no Obligation carries a signal | `forgetting-section-honest-empty-state` |
