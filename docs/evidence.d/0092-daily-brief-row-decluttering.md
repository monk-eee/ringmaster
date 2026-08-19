# EV-0092: Declutter shared row typography — real bold rendering, compact risk-signal pills, quiet quote treatment

Evidence for [ADR-0092](../adr.d/0092-daily-brief-row-decluttering.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0092-daily-brief-row-decluttering"

[[check]]
id = "bold-segments-applied"
invariant = "renderBoldSegments exists and is imported by every shared-pattern component."
type = "present"
pattern = "renderBoldSegments"
paths = ["frontend/src/markdown.ts", "frontend/src/components/DailyBrief.tsx", "frontend/src/components/FocusBlocks.tsx", "frontend/src/components/ForgettingSection.tsx", "frontend/src/components/GraphExplorer.tsx", "frontend/src/components/ObligationDetail.tsx", "frontend/src/components/PersonBriefPanel.tsx", "frontend/src/components/TimeHorizon.tsx", "frontend/src/components/TimeHorizonTimeline.tsx"]

[[check]]
id = "selector-leak-fixed"
invariant = "The daily-brief-list row rule targets only direct-child <li>s, not nested risk-signal items."
type = "present"
pattern = '\.daily-brief-list > li'
paths = ["frontend/public/style.css"]

[[check]]
id = "risk-signals-are-pills"
invariant = "Risk signal badges reuse the existing at-risk color tokens as inline pills, not a bordered vertical list."
type = "present"
pattern = 'var\(--at-risk-bg\)'
paths = ["frontend/public/style.css"]

[[check]]
id = "playwright-suite-passes-declutter"
invariant = "The full Playwright suite passes against the decluttered rows."
type = "manual"
last_verified = "2026-08-19"
rationale = "`npx playwright test --project=chromium` run after this change; no text content changed (only how it's wrapped/styled), so every existing text-matching assertion held."
```

## Notes

`People.tsx`'s `careerExportText` deliberately does NOT use
`renderBoldSegments` — it remains literal, copyable plain text for a
Connect self-assessment (ADR-0088), unaffected by this ADR.
