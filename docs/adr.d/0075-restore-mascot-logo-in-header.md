# ADR-0075: Restore the mascot logo in the app header

- **Status:** Accepted
- **Date:** 2026-08-18
- **Decider:** monk-eee
- **Approval:** Direct instruction ("we lost the logo from ringmaster ... fix that"), 2026-08-18
- **Amends:** [ADR-0074](0074-visual-design-system-refresh.md) — reverses only its "remove the mascot image from the header" sub-decision; every other part of ADR-0074 (design tokens, tab bar, card/table chrome, motion) is unaffected and stays as decided.
- **Depends on:** [ADR-0074](0074-visual-design-system-refresh.md)
- **Tags:** frontend, design, presentation

## Context

ADR-0074 deliberately removed the `ringmaster_logo.png` mascot `<img>` from
the header chrome, keeping the file only as the favicon and README image.
On seeing the resulting header live, the decider asked for the logo back.
This is a direct, real-time reversal of that one specific design choice,
not a re-litigation of the rest of ADR-0074's token/chrome refresh.

## Decision

Restore `<img className="logo" src="/ringmaster_logo.png?v=2" alt="Ringmaster" />`
as the first child of `header.app-bar` in `frontend/src/App.tsx`, and
restore its `header.app-bar img.logo { height: 44px; width: auto; display:
block; }` rule plus the `.page-heading`'s left hairline divider in
`frontend/public/style.css`, both exactly as they existed before
ADR-0074's removal. No other ADR-0074 change (tokens, tab bar, card/table
chrome, motion) is touched.

## Scope

**In scope:** the single `<img>` element and its CSS rule.

**Out of scope:** everything else ADR-0074 changed — the new color tokens,
tab bar treatment, card/table chrome, and motion stay as ADR-0074 decided.

## Consequences

- **Positive:** the logo is visible again in the app's own chrome, per
  direct instruction.
- **Trade-off:** re-introduces the exact visual element ADR-0074's Context
  section named as clashing with the refreshed header; the decider has
  weighed that trade-off directly by asking for it back.

## Exit criteria and evidence

Evidence: [EV-0075](../evidence.d/0075-restore-mascot-logo-in-header.md)

| Exit criterion | Evidence |
|---|---|
| The header renders the mascot `<img>` again | `header-mascot-restored` |
| The crate/frontend still builds and the Playwright suite is unaffected structurally | `no-structural-regression` |
