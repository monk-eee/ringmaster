# EV-0075: Restore the mascot logo in the app header

Evidence for [ADR-0075](../adr.d/0075-restore-mascot-logo-in-header.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0075-restore-mascot-logo-in-header"

[[check]]
id = "header-mascot-restored"
invariant = "The app header renders the mascot <img> again."
type = "present"
pattern = 'img className="logo"'
paths = ["frontend/src/App.tsx"]

[[check]]
id = "no-structural-regression"
invariant = "Every test-relevant class name and user-facing string ADR-0074 named stays present, unchanged."
type = "present"
pattern = 'id-marker|recent-interactions-heading|daily-brief-list|Nothing needs your attention right now\.'
paths = ["frontend/src/App.tsx", "frontend/src/components/People.tsx", "frontend/src/components/DailyBrief.tsx"]
```

## Notes

`header-mascot-removed` in [EV-0074](0074-visual-design-system-refresh.md)
now permanently reads `FAIL` against the current header (the pattern it
checks is deliberately, again, present) — that is expected and correct:
this ADR amends that one sub-decision. EV-0074's other checks
(`design-tokens-replaced`, `no-structural-or-copy-change`,
`playwright-suite-passes-restyled`) are unaffected by this change.
