# EV-0053: "What am I forgetting?" — compose existing risk signals into one capped, prominent list

Evidence for [ADR-0053](../adr.d/0053-what-am-i-forgetting.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0053-what-am-i-forgetting"

[[check]]
id = "forgetting-section-capped-and-signal-filtered"
invariant = "Today shows a What am I forgetting section listing at most 5 Obligations, each with at least one risk signal."
type = "present"
pattern = 'FORGETTING_CAP = 5'
paths = ["frontend/src/components/ForgettingSection.tsx"]

[[check]]
id = "forgetting-section-ranked-by-signal-count"
invariant = "Rows are ordered by risk_signals.length descending, then existing Daily Brief order."
type = "present"
pattern = 'b\.risk_signals\.length - a\.risk_signals\.length'
paths = ["frontend/src/components/ForgettingSection.tsx"]

[[check]]
id = "forgetting-section-honest-empty-state"
invariant = "An honest empty state renders when zero Obligations carry a risk signal."
type = "present"
pattern = 'Nothing flagged right now\.'
paths = ["frontend/src/components/ForgettingSection.tsx"]
```

## Notes

Implemented: `ForgettingSection.tsx` filters the same `dailyBrief` array
Today already fetches to items with `risk_signals.length > 0`, sorts by
signal count descending (ties keep the API's own existing urgency order),
caps to 5, and renders each with the same `itemTitle`/`duePhrase` helpers
`DailyBrief.tsx` exports (ADR-0044) -- zero duplicated logic, zero new
backend route. Rendered on Today between the ranked list and "Do these
together". A Playwright test proves it renders either real flagged rows or
the honest empty state, tolerant of the shared dev database's contents.
