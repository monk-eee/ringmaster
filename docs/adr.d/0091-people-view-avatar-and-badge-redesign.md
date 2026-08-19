# ADR-0091: People view redesign — avatars, status badges, elevated card layout

- **Status:** Accepted
- **Date:** 2026-08-19
- **Decider:** monk-eee
- **Approval:** "christ its still really ugly especially the people view / look at this
  elegance" alongside a reference screenshot of a polished HR dashboard
  (rounded colored avatar initials, pill-shaped status badges, a calm
  card grid) — direct, in-the-moment creative direction, the same kind of
  instruction ADR-0074 and ADR-0075 already treated as decider approval
  for a presentational change.
- **Depends on:** [ADR-0074](0074-visual-design-system-refresh.md) (design
  tokens this ADR reuses, does not replace), [ADR-0051](0051-relationship-workspace.md)
  (People tab content this ADR restyles, unchanged)
- **Tags:** frontend, design, presentation, people

## Context

ADR-0074 gave the whole app a considered token palette and decluttered the
header, but it was explicitly scoped to tokens plus the header — it never
touched individual view layouts. The People tab still renders as a plain
list of bordered text blocks: a name, a role string, an at-risk/open count
as a plain colored line of text, and a "last heard from" line, with no
visual anchor (no avatar, no status affordance) to make a person feel like
a person rather than a database row. Compared against a reference like the
one supplied (colored circular initials, pill-shaped status chips, a calm
card grid), the gap is real and specific to this one view.

This is, again, a presentational request: make the People tab's existing
data (nothing new is fetched) read as a considered piece of product design,
without changing any route, response shape, state management, or the
test-relevant text/class contracts the Playwright suite already depends on
(`.people-card` count, `.people-list` visibility, `.people-detail h3`
containing exactly the person's name, a button whose accessible name still
contains the person's name).

## Decision

- Add a small, deterministic **avatar** to every person card and to the
  detail header: a circle with the person's initials, colored by hashing
  their name into one of six fixed accent tones drawn from the existing
  token palette's family (no new arbitrary colors invented outside what
  ADR-0074 already established as this app's palette). Purely presentational
  — computed client-side from `canonical_text` already in hand, no new data.
- Restyle the at-risk/open counts as **pill-shaped badges** reusing the
  already-accepted `--at-risk-bg`/`--at-risk-fg` and `--open-bg`/`--open-fg`
  tokens (same colors the Daily Brief and Obligations table already use for
  these exact states) instead of a plain colored text line.
- Rework `.people-card` into a horizontal layout (avatar left, name/role/
  badges/last-interaction stacked right) with a touch more breathing room
  and a subtle hover lift, instead of the current vertically-stacked plain
  block.
- Give the detail view a matching header: a larger avatar beside the
  person's name, styled consistently with the list view.
- No new fetch, no new route, no change to `careerExportText`,
  `relativeInteraction`, or any other existing function's behavior — only
  markup added around already-rendered values, and CSS.

## Scope

**In scope:** `frontend/src/components/People.tsx` (avatar/initials helper
functions and the JSX in the list-card and detail-header render paths only)
and `frontend/public/style.css` (new `.people-avatar*`/`.people-badge*`
rules plus `.people-card`/`.people-detail` layout rules).

**Out of scope, named honestly:**

- **Every other tab's visual design.** Today/Timeline/Inbox/Graph/Workbench
  etc. are unchanged by this ADR; if they need the same treatment later,
  that is a separate, explicitly scoped decision, not an implied extension
  of this one.
- **Any change to what data is fetched or how `PersonListNode`/`NodeDetail`
  are shaped.** Zero backend touch.
- **The exact text of `.people-detail h3`** (must stay exactly the
  person's name, no avatar text inside it — the avatar is a sibling
  element, not a child of the heading) **and the count/visibility
  contracts** `tests/obligations.spec.ts` already depends on
  (`.people-card` count, `.people-list` visibility, a button whose
  accessible name still contains the person's name via substring match).
- **A generic, reusable avatar component for other views** (e.g. Graph
  Explorer's person nodes). This ADR scopes the avatar helper to
  `People.tsx` only; reuse elsewhere is a future decision if wanted.

## Options considered

- **Avatar + badge redesign scoped to People only (chosen):** directly
  answers the specific, named complaint ("especially the people view")
  without re-litigating every other tab's presentation, which was not
  asked for and would be a much larger, riskier diff.
- **Adopt the reference dashboard's full layout (sidebar nav, schedule
  grid, multi-panel dashboard):** rejected as disproportionate — the
  reference is a different product's information architecture; ADR-0039/
  ADR-0051 already made real, considered IA decisions for People that this
  ADR does not re-open. Only the *visual polish* (avatars, badges, card
  rhythm) is adopted, not the layout.
- **A component library for avatars/badges:** rejected for the same reason
  ADR-0074 rejected a CSS framework — the existing plain-CSS approach
  (ADR-0014) handles this scope of change without a new dependency.

## Consequences

- **Positive:** the People tab's existing, already-correct data reads as a
  considered interface instead of a bordered list of text rows, directly
  answering the complaint that prompted this ADR.
- **Positive:** every Playwright spec keeps passing unchanged — no class
  the suite locates by name is removed, no heading text changes, no
  accessible-name substring the suite matches on is lost.
- **Negative / trade-off:** the avatar color is derived from a client-side
  name hash, not a designed-per-person choice — two people can coincidentally
  land on the same tone; this is accepted as a cosmetic limitation, not a
  data problem.
- **Risk:** low. Additive markup plus CSS only, validated by the existing
  Playwright suite plus a manual visual check against the reference.

## Exit criteria and evidence

Evidence: [EV-0091](../evidence.d/0091-people-view-avatar-and-badge-redesign.md)

| Exit criterion | Evidence |
|---|---|
| Person cards and the detail header render a colored initials avatar | `people-avatar-present` |
| At-risk/open counts render as pill badges reusing the existing status tokens | `people-badges-reuse-tokens` |
| `.people-detail h3` still contains exactly the person's name | `detail-heading-text-unchanged` |
| The full Playwright suite passes unchanged against the restyled People tab | `playwright-suite-passes-people-redesign` |
