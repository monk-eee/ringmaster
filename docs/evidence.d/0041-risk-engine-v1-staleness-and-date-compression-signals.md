# EV-0041: Risk Engine v1 — staleness and date-compression signals

Evidence for [ADR-0041](../adr.d/0041-risk-engine-v1-staleness-and-date-compression-signals.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0041-risk-engine-v1-staleness-and-date-compression-signals"

[[check]]
id = "risk-signals-function-exists"
invariant = "A risk_signals function computes signals independently of the ranking/bucketing routes."
type = "present"
pattern = 'fn risk_signals\('
paths = ["backend/src/api.rs"]

[[check]]
id = "date-compression-reuses-the-7-day-window"
invariant = "Date compression fires within the same 7-day window Time Horizon already uses, not a second invented number."
type = "present"
pattern = "DATE_COMPRESSION_WINDOW_DAYS"
paths = ["backend/src/api.rs"]

[[check]]
id = "stale-signal-has-a-disclosed-threshold"
invariant = "Staleness fires past a disclosed, hardcoded threshold."
type = "present"
pattern = "STALE_THRESHOLD_DAYS"
paths = ["backend/src/api.rs"]

[[check]]
id = "daily-brief-and-time-horizon-expose-risk-signals-field"
invariant = "Both GET /api/daily-brief and GET /api/time-horizon attach a risk_signals field per row."
type = "present"
pattern = '"risk_signals": risk_signals\('
paths = ["backend/src/api.rs"]

[[check]]
id = "frontend-renders-risk-signals-list"
invariant = "Daily Brief and Time Horizon rows render a risk-signals list when signals are present."
type = "present"
pattern = 'risk-signals'
paths = ["frontend/src/components/DailyBrief.tsx", "frontend/src/components/TimeHorizon.tsx"]

[[check]]
id = "playwright-proves-risk-signal-text-renders"
invariant = "Focused browser coverage proves a risk signal's explanation renders as real text when one exists."
type = "present"
pattern = 'ADR-0041'
paths = ["frontend/tests/obligations.spec.ts"]
```

## Notes

`risk_signals` is a pure function (no database access), covered directly
by unit tests: date compression fires when due within 7 days (or overdue)
with no linked evidence, does not fire when evidence is linked or the due
date is far out; staleness fires past 14 days since `updated_at`, does not
fire within the threshold; both signals can fire together on the same
Obligation. Two route-level integration tests (using the repo's
unique-obligation-id-lookup pattern, never an aggregate count) confirm
`risk_signals` is genuinely present in the `/api/daily-brief` and
`/api/time-horizon` JSON responses, not just defined and unused.
`tsc --noEmit` and `vite build` pass. 83/83 backend tests pass
(75 prior + 8 new). No combined severity score or color exists yet —
deliberately deferred, per the ADR's own scope section.
