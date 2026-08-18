# EV-0079: Timeline surfaces a linked source's own occurred_at

Evidence for [ADR-0079](../adr.d/0079-timeline-surfaces-source-occurred-at.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0079-timeline-surfaces-source-occurred-at"

[[check]]
id = "time-horizon-includes-source-occurred-at"
invariant = "GET /api/time-horizon includes source_occurred_at, null when there is no linked source."
type = "present"
pattern = '"source_occurred_at"'
paths = ["backend/src/api/obligations.rs"]

[[check]]
id = "bucket-placement-still-due-date-only"
invariant = "time_horizon_bucket still buckets purely by hard_due_at/soft_due_at, unchanged by source_occurred_at."
type = "absent"
pattern = 'fn time_horizon_bucket\([^)]*source_occurred_at'
paths = ["backend/src/api/obligations.rs"]

[[check]]
id = "timeline-renders-source-occurred-at"
invariant = "The Timeline view renders the linked source's occurred date in expanded item detail, and renders nothing when it is absent."
type = "present"
pattern = "source_occurred_at"
paths = ["frontend/src/components/TimeHorizonTimeline.tsx"]
```

## Notes

Implemented: `backend/src/api/obligations.rs`'s `time_horizon` query gains
a `LEFT JOIN nodes sn ON sn.id = sf.source_id` (through the existing
`source_fragments` join) selecting `sn.occurred_at`, serialized as the new
`source_occurred_at` field -- null when there is no linked source.
`time_horizon_bucket` is untouched; bucket placement still depends only
on `hard_due_at`/`soft_due_at`. `frontend/src/api.ts`'s `TimeHorizonItem`
gains the matching typed field. `frontend/src/components/TimeHorizonTimeline.tsx`
renders it as a small caption ("Source occurred <date>") under each
expanded item's existing reason text, only when present; `frontend/public/style.css`
gains the matching `.time-horizon-source-occurred-at` style.

Verified: a new backend test,
`time_horizon_includes_source_occurred_at_without_changing_bucket_placement`,
proves both halves of the exit criteria in one pass -- an obligation with
a linked source (whose node has a distinct `occurred_at`) reports that
exact date as `source_occurred_at` while still landing in `next_7_days`
(driven by `hard_due_at`), and a sibling obligation with no linked source
reports `null` rather than a fabricated date. Full backend suite (including
all pre-existing Time Horizon tests, unaffected) passed via Unit Test MCP
against an isolated `ringmaster_test` database. `cargo build --workspace`,
`cargo clippy --all-targets --all-features -- -D warnings` (zero
warnings), `cargo fmt --all -- --check` (clean), `npx tsc --noEmit`, and
`npm run build` all passed.
