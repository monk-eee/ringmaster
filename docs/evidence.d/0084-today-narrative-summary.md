# EV-0084: Today's narrative summary — the ranked count line VISION.md describes

Evidence for [ADR-0084](../adr.d/0084-today-narrative-summary.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0084-today-narrative-summary"

[[check]]
id = "today-greeting-is-time-of-day-aware"
invariant = "Today shows a time-of-day greeting with no fabricated name."
type = "present"
pattern = "getHours"
paths = ["frontend/src/App.tsx"]

[[check]]
id = "summary-counts-reuse-existing-risk-signals"
invariant = "The summary reports counts of date_compression and stale signals from already-fetched daily brief items, no new backend field."
type = "present"
pattern = '"date_compression"'
paths = ["frontend/src/App.tsx"]

[[check]]
id = "summary-omits-zero-counts"
invariant = "A zero count for either stat line is omitted, not shown as \"0 ...\"."
type = "present"
pattern = "playwright-proves-today-narrative-summary"
paths = ["frontend/tests/obligations.spec.ts"]

[[check]]
id = "summary-honest-empty-state-unchanged"
invariant = "An empty daily brief still shows the existing honest empty state, no stat lines."
type = "present"
pattern = "Nothing needs your attention right now"
paths = ["frontend/src/App.tsx"]

[[check]]
id = "existing-today-sections-unchanged"
invariant = "The ranked list, Forgetting section, and Focus Blocks render unchanged below the summary."
type = "manual"
last_verified = "2026-08-19"
rationale = "DailyBrief/ForgettingSection/FocusBlocks components and their existing Playwright coverage are not edited by this change; the existing People/Inbox/Today Playwright suite continues to pass unmodified, which is the direct proof."
```

## Notes

Implemented: `frontend/src/App.tsx` adds `timeOfDayGreeting()` (a pure
function over `new Date().getHours()`, no stored/fabricated name) and two
`useMemo` counts (`dateCompressionCount`/`staleCount`) filtering the
already-fetched `dailyBrief` array by `risk_signals[].signal`. The
single-line greeting is replaced with a `.today-summary` block rendering
the greeting plus up to three stat lines; the `date_compression`/`stale`
lines are omitted entirely when their count is zero. The empty-daily-brief
path is unchanged (`.today-greeting` alone, no `.today-summary`).
`frontend/public/style.css` adds `.today-summary`/`.today-summary-line`
styling. No backend file touched.

Verified: `npx tsc --noEmit` and `npm run build` clean. Two new Playwright
tests, both passing: one mocks `GET /api/daily-brief` with a
`date_compression`-flagged item and a `stale`-flagged item and asserts the
exact summary lines ("2 things need attention today.", "1 will become
risk this week.", "1 commitment appears forgotten.", plus a time-of-day
greeting); the other mocks an empty daily brief and asserts the existing
honest empty state renders with zero `.today-summary` elements. Ran the
full Playwright suite against the isolated stack: 0 failed.

## Notes

Implemented: `frontend/src/App.tsx` renders a `.today-summary` block above
the existing ranked list -- a time-of-day greeting (`p.today-greeting`,
`getHours()`-derived) plus `p.today-summary-line` entries for the
existing count, `date_compression` ("will become risks"), and `stale`
("appear forgotten"), each computed with `useMemo` over the already-fetched
`dailyBrief` array. Zero counts are omitted, never rendered as "0 ...".
`frontend/public/style.css` adds the `.today-summary`/`.today-greeting`/
`.today-summary-line` rules; no backend/route/schema change.

Verified: `npx tsc --noEmit` and `npm run build` both clean. Two Playwright
tests in `frontend/tests/obligations.spec.ts` cover this record --
`today: narrative summary reports date_compression/stale counts and omits
zero counts (ADR-0084)` mocks `GET /api/daily-brief` for two items (one
`date_compression`, one `stale`) and asserts the exact rendered text
("2 things need attention today.", "1 will become risk this week.", "1
commitment appears forgotten."); `today: honest empty state renders no
narrative summary when nothing needs attention (ADR-0084)` mocks an empty
response and asserts the pre-existing empty state renders with zero
`.today-summary` elements. Full suite: 19 passed, 5 skipped (pre-existing,
unrelated to this change), 0 failed.
