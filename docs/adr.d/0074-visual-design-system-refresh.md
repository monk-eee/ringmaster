# ADR-0074: Visual design system refresh — a considered look, zero behavior change

- **Status:** Accepted
- **Date:** 2026-08-18
- **Decider:** monk-eee
- **Approval:** "imagine you are johny ives... build an engaging clever interface for this - that doesnt suck", 2026-08-18 (standing "accept everything and keep going" instruction earlier this session)
- **Depends on:** [ADR-0012](0012-minimal-http-api-and-node-web-front-end.md), [ADR-0014](0014-react-vite-single-page-app.md), [ADR-0030](0030-human-readable-titles-and-type-iconography.md), [ADR-0039](0039-product-re-steer-primary-navigation.md)
- **Tags:** frontend, design, presentation

## Context

The frontend has real, well-governed information architecture (ADR-0039's
Today/Timeline/People/Inbox restructuring, ADR-0044's management-meaning
rows) but its visual language was never itself a design decision — colors,
type, spacing, and chrome accumulated as whatever a generic component
template produced: a navy/indigo gradient header with a heavy drop shadow,
a stock-admin-panel tab bar, boxy card shadows, and a literal stock-clipart
mascot rendered at full size in the sticky header, clashing with everything
around it. None of this was ever reviewed as a considered whole.

This is a presentational request: make the existing, already-correct
structure look and feel like it was designed on purpose, without touching
any route, response shape, data flow, or test-relevant DOM structure.

## Decision

- Replace the color/spacing/shadow design tokens in
  `frontend/public/style.css`'s `:root` block with a considered, cohesive
  palette: a warm paper canvas and warm-neutral ink (replacing the
  blue-tinted gray template default), a single deliberate accent (a deep
  garnet/aubergine, distinct in hue from the existing status colors —
  open/blue, at-risk/orange, closed/gray — so nothing already meaningful
  gets muddied), softer/more diffuse shadows, and a slightly larger
  , warmer radius scale.
- Replace the header's navy gradient + drop shadow + full-size mascot image
  with a quiet, flat, typographic header: a confident wordmark, a hairline
  bottom border instead of a box shadow, no gradient. The mascot image
  (`ringmaster_logo.png`) is untouched as a file — still the favicon and
  the README's image — it is only removed from the in-app chrome, where a
  colorful stock-icon collage undercuts a considered interface.
- Refine the tab bar into a quieter, more integrated control: a thinner
  active-tab indicator, calmer secondary-tab treatment, tightened spacing
  rhythm.
- Soften card/table/badge chrome (shadows, borders, radius) to match the
  new tokens, and align the handful of hardcoded off-palette colors
  (a stray blue hover tint on the graph filter chip, an indigo-tinted
  textarea background) to the new accent so nothing looks orphaned from
  the rest of the system.
- Add restrained motion: a brief, `prefers-reduced-motion`-respecting
  fade-and-rise on `.card` mount, and slightly smoother hover/focus
  transitions already present on interactive elements.
- Every existing class name, element structure, emoji glyph (ADR-0030,
  unchanged), and text string stays exactly as-is. This is styling only.

## Scope

**In scope:** `frontend/public/style.css`'s design tokens and the rules
built on them; `frontend/src/App.tsx`'s header markup (removing the
mascot `<img>`, keeping the same wordmark text).

**Out of scope, named honestly:**

- **Any API, route, data-shape, or state-management change.** Zero backend
  touch; zero frontend logic touch beyond the header markup above.
- **Any copy/text change.** Every user-facing string (including the
  Today-page greeting/empty-state text some Playwright specs assert on)
  stays byte-for-byte identical.
- **Removing or renaming any class the test suite or evidence checks
  reference** (`id-marker`, `recent-interactions-heading`, `.people-card`,
  `.daily-brief-list`, etc.) — verified absent from this change.
- **A new component library, CSS framework, or build-tool change.** Same
  plain CSS file, same Vite/React setup (ADR-0014).
- **The emoji type-icon system (ADR-0030).** Unchanged — the small color
  accents they already provide work well against the refined neutral
  canvas and are left exactly as decided.
- **Rebranding the product name or logo asset itself.** The mascot file
  stays; only its prominence in the app's own chrome changes.

## Options considered

- **Token-level redesign plus header declutter (chosen):** the existing
  CSS already routes almost every color/shadow/radius through custom
  properties, so replacing the token values plus the handful of rules
  that don't (the header, a couple of hardcoded hex hovers) transforms the
  whole app's feel from one small, reviewable diff — no new dependency, no
  structural risk to the test suite.
- **Adopt a component/CSS framework (Tailwind, a design-system library):**
  rejected as disproportionate — a real UI transformation is achievable
  entirely within the existing plain-CSS approach ADR-0014 already chose,
  and swapping the styling approach itself is a much larger, riskier,
  unrequested change.
- **Redesign the information architecture too:** rejected — ADR-0039/0044
  already made real, considered IA decisions; this ADR is scoped to making
  the existing structure look intentional, not to re-litigating what's
  shown.

## Consequences

- **Positive:** the app reads as a single, considered piece of work rather
  than an assembled template, with zero functional risk.
- **Positive:** every Playwright spec and evidence `present`/`absent`
  check keeps passing unchanged, since no class name, text string, or DOM
  structure moves.
- **Negative / trade-off:** a visual opinion is now embedded in the
  codebase where none was decided before; a future rebrand is a new,
  separate decision, not a revert of this one.
- **Risk:** low. Pure CSS plus one non-text markup removal, validated by
  the existing Playwright suite and a manual visual check.

## Exit criteria and evidence

Evidence: [EV-0074](../evidence.d/0074-visual-design-system-refresh.md)

| Exit criterion | Evidence |
|---|---|
| The design tokens in `:root` no longer match the old template palette | `design-tokens-replaced` |
| The header no longer renders the stock mascot image | `header-mascot-removed` |
| No test-relevant class name, id, or user-facing string changed | `no-structural-or-copy-change` |
| The full Playwright suite passes unchanged against the restyled app | `playwright-suite-passes-restyled` |
