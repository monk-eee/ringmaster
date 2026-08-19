# ADR-0092: Declutter shared row typography — real bold rendering, compact risk-signal pills, quiet quote treatment

- **Status:** Accepted
- **Date:** 2026-08-19
- **Decider:** monk-eee
- **Approval:** "i fucking hate the ui it it is not modern and everything feels crowded can you
  do celaner and better" — direct, in-the-moment creative direction, the same
  kind of instruction ADR-0074/ADR-0075/ADR-0091 already treated as decider
  approval for a presentational change.
- **Depends on:** [ADR-0074](0074-visual-design-system-refresh.md) (design
  tokens reused here, not replaced), [ADR-0091](0091-people-view-avatar-and-badge-redesign.md)
  (the pill-badge visual language this ADR extends to risk signals)
- **Tags:** frontend, design, presentation

## Context

Today/Timeline/Focus Blocks/Workbench/Graph's person view/"What am I
forgetting?" all share one rendering pattern (`DailyBrief.tsx`'s
`itemTitle()`/`.today-item-title`, `.daily-brief-reason`, and a
`.risk-signals` list), since ADR-0044 correctly decided a row's title must
be the real evidence quote, never a fabricated summary. Looking at the
running app with real ingested data (not synthetic fixtures) surfaced two
concrete, fixable problems in that one shared pattern, not a vague
"modernize everything":

1. **Literal, un-rendered markdown.** Real meeting transcripts already
   contain `**bold**` markdown (e.g. `"**1:1s and Coaching:** - Weekly
   1:1..."`). Rendered as plain text, the asterisks show up literally,
   reading as noise rather than emphasis.
2. **A real CSS selector-leakage bug.** `.daily-brief-list li` (no child
   combinator) matches every `<li>` descendant, including the nested
   `<li>` elements inside a row's own `.risk-signals` list. Each risk
   signal was silently inheriting the *outer row's* `1rem 1.25rem` padding
   and `1px solid` bottom border — a row with two signals cost four
   padded, hairline-divided lines for what should be two short badges.
   This is the single largest concrete contributor to "everything feels
   crowded": it was never a considered design, it was an accidental
   selector match.

This ADR fixes exactly those two things, plus gives the evidence-quote
title its own quiet, quote-like typographic treatment (matching how a real
product distinguishes "this is a citation" from "this is UI copy") — it
does not redesign the information architecture, add new data, or touch any
other tab's layout beyond this one shared, widely-reused pattern.

## Decision

- Add `frontend/src/markdown.ts`: `renderBoldSegments(text)` splits text on
  `**bold**` pairs and returns React nodes (`<strong>` for bold segments,
  plain strings otherwise) — **never** `dangerouslySetInnerHTML`, so there
  is no HTML-injection surface regardless of what real transcript text
  contains. Applied everywhere `itemTitle()`'s evidence quote and a
  `.reason`/`.statement` field render: `DailyBrief.tsx`, `FocusBlocks.tsx`,
  `ForgettingSection.tsx`, `GraphExplorer.tsx`'s `renderRelationshipGroup`,
  `ObligationDetail.tsx`, `PersonBriefPanel.tsx`, `TimeHorizon.tsx`,
  `TimeHorizonTimeline.tsx`. **Not** applied to `People.tsx`'s
  `careerExportText` — that stays literal, copyable plain text for Connect
  per ADR-0088, unaffected by this ADR.
- Fix the selector leak: `.daily-brief-list li` → `.daily-brief-list > li`
  (and its `:last-child`/`:hover` companions), so a row's own padding/
  border no longer leaks onto nested `.risk-signals` items.
- Restyle `.risk-signals` from a vertical, hairline-divided list into
  compact inline-wrapping pill badges (same visual language ADR-0091 gave
  People's at-risk/open counts) — reusing the existing `--at-risk-bg`/
  `--at-risk-fg` tokens, no new colors.
- Give `.today-item-title` its own rule (previously unstyled, inheriting
  plain body-text/heading defaults depending on which tag rendered it): a
  quiet left border, italic, comfortable `1.6` line-height — reads as a
  quoted excerpt, not a status paragraph. Explicit `font-size`/`font-weight`
  so it looks identical whether the caller uses a `<p>` (most callers) or
  `<h2>` (`ObligationDetail.tsx`).

## Scope

**In scope:** `frontend/src/markdown.ts` (new), the eight component files
named above (import + apply the helper only — no other logic changed),
and `frontend/public/style.css`'s `.daily-brief-list li`/`.risk-signals`/
`.today-item-title` rules.

**Out of scope, named honestly:**

- **Any other markdown syntax** (headings, links, lists, italics via `_x_`).
  Only `**bold**` — the one pattern actually observed in real ingested
  data. Anything richer is a separate decision if real content ever needs it.
- **`People.tsx`'s Career export text.** Stays byte-for-byte literal per
  ADR-0088 — this ADR does not touch it.
- **Every other tab's layout.** This is one shared rendering pattern
  reused across many tabs, not an app-wide redesign; Inbox's table,
  Meetings, Activity, and the People-view work from ADR-0091 are unchanged.
- **Any data/route change.** Zero backend touch; `item.reason`,
  `itemTitle()`'s return value, and `risk_signals` are rendered exactly as
  returned, just with `**bold**` interpreted and less accidental chrome.

## Options considered

- **Fix the two concrete, evidenced problems (chosen):** targeted,
  low-risk, directly grounded in what the running app with real data
  actually showed — not a subjective, unbounded "make it prettier."
- **A real markdown renderer (e.g. `react-markdown`) for full CommonMark:**
  rejected as disproportionate — no observed real content uses anything
  beyond `**bold**`, and a full markdown dependency is a much larger,
  riskier addition (arbitrary HTML/link rendering surface) for a pattern
  this narrow.
- **Redesign every tab's spacing/density from scratch:** rejected — the
  actual, evidenced complaint traced to one reused pattern and one CSS
  bug; a repo-wide redesign is unbounded and unrequested beyond that.

## Consequences

- **Positive:** every tab that reuses the Daily Brief row pattern (Today,
  Timeline, Focus Blocks, "What am I forgetting?", Workbench, Graph's
  person view, People's relationship groups) gets legible bold emphasis
  and a real fix to the accidental extra padding/borders, in one change.
- **Positive:** zero new dependencies, zero HTML-injection surface (no
  `dangerouslySetInnerHTML`), zero data/route change.
- **Negative / trade-off:** only `**bold**` is recognized; other markdown
  in future ingested content still renders literally until a separate
  decision extends this.
- **Risk:** low. Presentational plus one pure-function helper, validated by
  the existing Playwright suite (no text content changes, only how it's
  wrapped) and a manual visual check.

## Exit criteria and evidence

Evidence: [EV-0092](../evidence.d/0092-daily-brief-row-decluttering.md)

| Exit criterion | Evidence |
|---|---|
| `renderBoldSegments` exists and is applied across every shared-pattern component | `bold-segments-applied` |
| The `.daily-brief-list` child-combinator fix is in place | `selector-leak-fixed` |
| Risk signals render as compact pills reusing existing status tokens | `risk-signals-are-pills` |
| The full Playwright suite passes unchanged against the decluttered rows | `playwright-suite-passes-declutter` |
