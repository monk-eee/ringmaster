# ADR-0079: Timeline surfaces a linked source's own occurred_at

- **Status:** Accepted
- **Date:** 2026-08-18
- **Decider:** monk-eee
- **Approval:** Direct instruction ("Work autonomously and make good decisions") selecting this item from `docs/IMPROVEMENT-PLAN.md`'s Priority 1.3, 2026-08-18
- **Depends on:** [ADR-0029](0029-time-horizon-view.md), [ADR-0035](0035-time-horizon-timeline-view.md), [ADR-0040](0040-dated-source-ingestion.md), [ADR-0042](0042-occurred-at-retrieval-and-recall-sources-mcp-tool.md)
- **Tags:** frontend, backend, timeline, data-model

## Context

[ADR-0040](0040-dated-source-ingestion.md) made `occurred_at` required on
every ingested source; [ADR-0042](0042-occurred-at-retrieval-and-recall-sources-mcp-tool.md)
made it readable. Both explicitly deferred "teaching Timeline/Time Horizon
to weigh a linked source's `occurred_at`" — those views still rank and
bucket purely by Obligation due date. `docs/IMPROVEMENT-PLAN.md` names
this the lowest-novelty remaining gap: the field already exists and is
retrievable; this is wiring an existing column into an existing read
model and view, not new design.

## Decision

- `GET /api/time-horizon`'s existing query gains one additional `LEFT
  JOIN nodes sn ON sn.id = sf.source_id` (through the same
  `source_fragments` join the route already performs for evidence text),
  selecting `sn.occurred_at` as a new `source_occurred_at` field on every
  `TimeHorizonItem`. `NULL` when there is no linked source, exactly like
  the existing `source_fragment_id`/`reason` fields already tolerate.
- **Bucket placement is unchanged.** Which of Overdue/Next 7/30/90
  days/Beyond an item falls into still depends only on
  `hard_due_at`/`soft_due_at` ([ADR-0029](0029-time-horizon-view.md)) —
  `source_occurred_at` is additive display data, not a new ranking input.
  Changing what drives bucket placement is a materially larger, separate
  design decision this ADR does not make.
- The Timeline view's (`TimeHorizonTimeline.tsx`, [ADR-0035](0035-time-horizon-timeline-view.md))
  expanded per-item detail renders `source_occurred_at`, when present, as
  a small caption under the existing reason text: "Source occurred
  <date>". Nothing renders when it is absent — no fabricated date, no
  placeholder text.
- No change to `/api/daily-brief` or Focus Blocks; this ADR's scope is the
  Time Horizon route and its Timeline view only.

## Scope

**In scope:** `time_horizon`'s SQL and JSON response gaining
`source_occurred_at`; `TimeHorizonItem`'s TypeScript type; the Timeline
component's expanded-item display of it.

**Out of scope, named honestly:** changing bucket placement/ranking to
weight `source_occurred_at` instead of, or alongside, the due date;
Daily Brief or Focus Blocks gaining the same field (a separate, smaller
follow-up if wanted); any change to `/api/obligations`'s own response
shape (this ADR touches `/api/time-horizon` only); natural-language or
relative date rendering beyond the existing `toLocaleDateString` pattern
already used elsewhere in this component.

## Options considered

- **Additive `source_occurred_at` field, display-only (chosen):** the
  literal "wiring, not design" framing from the improvement plan —
  reuses the existing join, existing nullable-field conventions, and
  existing date-formatting pattern; zero risk to bucket placement or any
  other consumer of `TimeHorizonItem`.
- **Re-bucket by `source_occurred_at` when present:** rejected — Time
  Horizon's entire purpose ([ADR-0029](0029-time-horizon-view.md)) is
  "what's due when," and conflating "when it was discussed" with "when
  it's due" would quietly change the view's meaning for every existing
  Obligation, not just add information.
- **Add it to every route that returns an Obligation** (`/api/obligations`,
  Daily Brief, Focus Blocks): rejected as scope creep — Timeline is the
  one place `docs/IMPROVEMENT-PLAN.md` names as the actual gap; extending
  everywhere increases the diff for no named benefit yet.

## Exit criteria and evidence

| Exit criterion | Evidence |
|---|---|
| `GET /api/time-horizon` includes `source_occurred_at`, null when there is no linked source | `time-horizon-includes-source-occurred-at` |
| Bucket placement is unchanged when `source_occurred_at` differs from the due date | `bucket-placement-still-due-date-only` |
| The Timeline view renders the source's occurred date in expanded item detail, and renders nothing when absent | `timeline-renders-source-occurred-at` |
