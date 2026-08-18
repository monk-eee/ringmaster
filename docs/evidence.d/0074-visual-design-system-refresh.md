# EV-0074: Visual design system refresh — a considered look, zero behavior change

Evidence for [ADR-0074](../adr.d/0074-visual-design-system-refresh.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0074-visual-design-system-refresh"

[[check]]
id = "design-tokens-replaced"
invariant = "The :root design tokens no longer use the old template's blue-tinted gray canvas/indigo accent."
type = "absent"
pattern = '--bg: #f6f7fb;'
paths = ["frontend/public/style.css"]

[[check]]
id = "header-mascot-removed"
invariant = "The app header no longer renders the stock mascot image; the asset file itself is untouched."
type = "manual"
last_verified = "2026-08-18"
rationale = "Superseded by ADR-0075, which amends ADR-0074 and reverses only this sub-decision on direct instruction: the mascot <img> is restored in frontend/src/App.tsx's header.app-bar, verified by EV-0075's header-mascot-restored check. Every other part of this ADR (tokens, tab bar, card/table chrome, motion) is unaffected and still holds."

[[check]]
id = "no-structural-or-copy-change"
invariant = "Every test-relevant class name and user-facing string this ADR names stays present, unchanged."
type = "present"
pattern = 'id-marker|recent-interactions-heading|daily-brief-list|Nothing needs your attention right now\.'
paths = ["frontend/src/App.tsx", "frontend/src/components/People.tsx", "frontend/src/components/DailyBrief.tsx"]

[[check]]
id = "playwright-suite-passes-restyled"
invariant = "The full Playwright suite passes against the restyled app."
type = "manual"
last_verified = "2026-08-18"
rationale = "ADR-0073's harness was blocked by an unrelated ringmaster_test migration-tracking gap (tables existed but _sqlx_migrations was empty from being created outside sqlx's tracked path); fixed by dropping and recreating that disposable, isolated test database so migrations reapply cleanly. With that fixed, `npx playwright test --project=chromium` ran the full suite against the restyled app: 14 passed, 5 skipped (model-dependent), 0 failed."
```

## Notes

Implemented entirely in `frontend/public/style.css` (design tokens, header,
tabs, cards, motion) plus one markup removal in `frontend/src/App.tsx` (the
mascot `<img>`). No class name, id, DOM nesting, or user-facing text
changed. `playwright-suite-passes-restyled` starts honestly `manual` with no
`last_verified` until a fresh Playwright run against the restyled app is
recorded.

[ADR-0075](../adr.d/0075-restore-mascot-logo-in-header.md) reverses the
mascot-removal sub-decision by direct instruction; the mascot `<img>` is
back in `frontend/src/App.tsx`'s header, so `header-mascot-removed` above
is now a `manual` check documenting that supersession rather than an
`absent` pattern check (which would otherwise always fail against the
deliberately-restored markup). See
[EV-0075](0075-restore-mascot-logo-in-header.md) for the current check on
that markup. Every other check in this record is unaffected.
